use std::io::IoSlice;

const RETRY_CNT: u32 = 5;

use crate::{ Error, Result };
use crate::util::{ SocketAddr, UdpSocket };

use super::{ Request, RequestError, Datagram, Oack, Xfer, SequenceId };

#[derive(Default, Debug)]
pub struct Stats {
    pub filesize:	u64,
    pub xmitsz:		u64,
    pub retries:	u32,
    pub num_timeouts:	u32,
}

pub struct Session<'a> {
    remote:	SocketAddr,
    sock:	UdpSocket,
    env:	&'a crate::Environment,

    window_size:	u16,
    block_size:		u16,
    timeout:		std::time::Duration,
}

impl <'a> Session<'a> {
    pub async fn new(env: &'a crate::Environment,
		     remote: SocketAddr,
		     local: std::net::IpAddr) -> Result<Session<'a>> {
	let local_addr = SocketAddr::new(local, 0);

	let sock = UdpSocket::bind(local_addr)?;

	tracing::Span::current().record("remote", &remote.to_string());
	tracing::Span::current().record("local",  &sock.local_addr().unwrap().to_string());

	Ok(Self {
	    remote:		remote,
	    sock:		sock,
	    env:		env,

	    window_size:	1,
	    block_size:		512,
	    timeout:		env.timeout,
	})
    }

    async fn send(&self, msg: &[u8]) -> Result<()>
    {
	self.sock.sendto(msg, self.remote).await?;
	Ok(())
    }

    async fn send_slice(&self, data: &[IoSlice<'_>]) -> Result<()>
    {
	self.sock.sendmsg(data, self.remote).await
    }

    async fn send_datagram(&self, msg: Datagram<'_>) -> Result<()>
    {
	match msg {
	    Datagram::Data(seq, data)	=> {
		let seq = seq.as_slice();
		let data = &[
		    IoSlice::new(&[0, 3]),
		    IoSlice::new(&seq),
		    IoSlice::new(data),
		];

		self.send_slice(data).await
	    },

	    _	=> Err(Error::Internal("send_datagram not implemented for this message"))
	}
    }

    async fn send_err(self, e: Error) -> Result<()>
    {
	let mut msg = Vec::<u8>::with_capacity(1500);

	warn!("error: {}", e);

	msg.extend([0, 5]);

	match e {
	    Error::RequestError(d)	=> {
		msg.extend([0, 4]);
		msg.extend(d.to_string().as_bytes());
		msg.push(0);
	    },

	    Error::FileMissing		=> {
		msg.extend([0, 1]);
	    },

	    Error::TooMuchClients	=> {
		msg.extend([0, 4]);
		msg.extend(b"too much clients");
		msg.push(0);
	    },
	    _				=> {
		msg.extend([0, 0]);
	    },
	};

	msg.push(0);

	self.send(&msg).await
    }

    async fn send_oack(&self, oack: Oack) -> Result<()>
    {
	let mut msg = Vec::<u8>::with_capacity(1500);

	oack.fill_buf(&mut msg);

	self.send(&msg).await
    }

    async fn run_wrq(self, _req: Request<'_>) -> Result<Stats>
    {
	self.send_err(RequestError::WriteUnsupported.into()).await?;

	Err(Error::NotImplemented)
    }

    async fn run_oack(&mut self, mut oack: Oack, file_size: Option<u64>) -> Result<()>
    {
	oack.update_tsize(file_size);
	oack.update_block_size(self.env.max_block_size,   |v| self.block_size = v);
	oack.update_window_size(self.env.max_window_size, |v| self.window_size = v);
	oack.update_timeout(|v| self.timeout = v);

	let mut buf = vec![0u8; 1500];

	self.send_oack(oack).await?;

	let resp = Datagram::recv(&self.sock, &mut buf, &self.remote, self.timeout).await?;

	match resp {
	    Datagram::Ack(id) if id.as_u16() == 0	=> {},
	    Datagram::Ack(id)	=> {
		warn!("ACK of OACK with invalid id {}", id.as_u16());
		return Err(Error::BadAck);
	    }
	    r			=> {
		warn!("bad response to OACK: {:?}", r);
		return Err(Error::Protocol("bad response to OACK"));
	    },
	};

	Ok(())
    }

    fn log_request(&self, req: &Request<'_>, op: &'static str)
    {
	tracing::Span::current().record("op", &op.to_string());
	tracing::Span::current().record("filename", &req.get_filename().to_string_lossy().into_owned());

	debug!("request={:?}", req);
    }

    async fn run_rrq(mut self, req: Request<'_>) -> Result<Stats>
    {
	use crate::fetcher::Builder;

	let mut stats = Stats::default();

	self.log_request(&req, "read");

	let mut fetcher = Builder::new(self.env).instanciate(&req.get_filename())?;

	if let Err(e) = fetcher.open() {
	    self.send_err(e.clone()).await?;
	    return Err(e);
	}

	let fsize = fetcher.get_size();

	if let Some(sz) = fsize {
	    stats.filesize = sz;
	    tracing::Span::current().record("filesize", &sz);
	}

	let mut seq = match req.has_options() {
	    false	=> SequenceId::new(0),
	    true	=> {
		self.run_oack(Oack::from_request(&req), fsize).await?;
		SequenceId::new(1)
	    }
	};

	let mut xfer = Xfer::new(&fetcher, self.block_size, self.window_size);
	let mut retry = RETRY_CNT;

	loop {
	    xfer.fill_window(seq, &mut fetcher).await?;

	    if xfer.is_eof() {
		break;
	    }

	    for d in xfer.iter() {
		stats.xmitsz += d.get_data_len() as u64;
		self.send_datagram(d).await?;
	    }


	    let mut buf = vec![0u8; 1500];
	    let resp = Datagram::recv(&self.sock, &mut buf, &self.remote, self.timeout).await;

	    match resp {
		Err(Error::Timeout) if retry > 0    => {
		    retry -= 1;
		    stats.num_timeouts += 1;
		    debug!("timeout; resending seq {}", seq.as_u16());
		},
		Ok(Datagram::Ack(id))	=> {
		    debug!("got ACK #{}", id.as_u16());
		    retry = RETRY_CNT;
		    seq = id + 1
		},
		Err(Error::Timeout)	=> {
		    warn!("timeout while waiting for ACK");
		    return Err(Error::Timeout);
		},
		r			=> {
		    warn!("bad response to DATA: {:?}", r);
		    return Err(Error::Protocol("bad response to DATA"));
		},
	    }
	}

	debug!("stats: {:?}", stats);

	Ok(stats)
    }

    pub async fn run(self, req: Vec<u8>) -> Result<Stats>
    {
	let op = Datagram::try_from(req.as_slice());

	match op {
	    Ok(Datagram::Write(r))	=> self.run_wrq(r).await,
	    Ok(Datagram::Read(r))	=> self.run_rrq(r).await,
	    Ok(_)	=> {
		self.send_err(RequestError::OperationUnsupported.into()).await?;
		Err(RequestError::OperationUnsupported.into())
	    },
	    Err(e)	=> {
		self.send_err(e.clone()).await?;
		Err(e)
	    }
	}
    }

    pub async fn do_reject(self) -> Result<Stats>
    {
	self.send_err(Error::TooMuchClients).await?;
	Err(Error::TooMuchClients)
    }
}
