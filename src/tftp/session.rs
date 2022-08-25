use std::io::IoSlice;

const RETRY_CNT: u32 = 5;
const GENERIC_PKT_SZ: usize = 512;

use crate::{ Error, Result };
use crate::util::{ SocketAddr, UdpSocket, ToFormatted };

use super::{ Request, RequestError, Datagram, Oack, Xfer, SequenceId };

#[derive(Default, Debug)]
pub struct Stats {
    pub filesize:	u64,
    pub xmitsz:		u64,
    pub retries:	u32,
    pub wastedsz:	u64,
    pub num_timeouts:	u32,
    pub window_size:	u16,
    pub block_size:	u16,
    pub filename:	String,
    pub remote_ip:	String,
    pub local_ip:	String,
    pub is_complete:	bool,
}

impl Stats {
    pub fn has_errors(&self) -> bool {
	self.filesize != self.xmitsz ||
	    self.retries != 0 ||
	    self.wastedsz != 0 ||
	    self.num_timeouts != 0
    }

    pub fn speed_bit_per_s(&self, duration: std::time::Duration) -> Option<(f32, f32)> {
	if duration.is_zero() {
	    return None;
	}

	Some(((self.filesize as f64 / duration.as_secs_f64()) as f32,
	      (self.xmitsz as f64 / duration.as_secs_f64()) as f32))
    }
}

impl std::fmt::Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	write!(f, "\"{}\" ({} => {}, {}x {}) {} bytes", self.filename,
	       self.local_ip, self.remote_ip,
	       self.window_size, self.block_size,
	       self.filesize.to_formatted())?;

	if self.has_errors() {
            write!(f, ", sent={} ({} retries, {} blocks wasted, {} timeouts)",
		   self.xmitsz.to_formatted(),
		   self.retries, self.wastedsz.to_formatted(),
		   self.num_timeouts)?
	}

	Ok(())
    }
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
	self.sock.sendto(msg, self.remote).await
    }

    async fn send_slice(&self, data: &[IoSlice<'_>]) -> Result<()>
    {
	self.sock.sendmsg(data, self.remote).await
    }

    async fn send_datagram(&self, msg: Datagram<'_>) -> Result<()>
    {
	match msg {
	    Datagram::Data(seq, data)	=> {
		let mut hdr: [u8; 4] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };

		hdr[0..2].copy_from_slice(&[0, 3]);	    // DATA
		hdr[2..4].copy_from_slice(&seq.as_slice()); // sequence

		let data = &[
		    IoSlice::new(&hdr),
		    IoSlice::new(data),
		];

		self.send_slice(data).await
	    },

	    _	=> Err(Error::Internal("send_datagram not implemented for this message"))
	}
    }

    async fn send_err(self, e: Error) -> Result<()>
    {
	let mut msg = Vec::<u8>::with_capacity(GENERIC_PKT_SZ);

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

    async fn send_ack(&self, id: SequenceId) -> Result<()>
    {
	let msg: [u8; 4] = [ 0, 4,
			     ((id.as_u16() >> 8) & 0xff) as u8,
			     ((id.as_u16() >> 0) & 0xff) as u8 ];

	self.send(&msg).await
    }

    async fn send_oack(&self, oack: Oack) -> Result<()>
    {
	let mut msg = Vec::<u8>::with_capacity(GENERIC_PKT_SZ);

	oack.fill_buf(&mut msg);

	self.send(&msg).await
    }

    async fn wrq_oack(&mut self, mut oack: Oack) -> Result<()>
    {
	oack.update_block_size(self.env.max_block_size,   |v| self.block_size = v);
	// TODO: only window size of 1 is supported
	oack.update_window_size(1, |v| self.window_size = v);
	oack.update_timeout(|v| self.timeout = v);

	self.send_oack(oack).await?;

	Ok(())
    }

    async fn run_wrq_devnull(mut self, req: Request<'_>) -> Result<Stats>
    {
	self.log_request(&req, "write");

	let mut stats = Stats::default();

	stats.filename = req.get_filename().to_string_lossy().into_owned();
	stats.remote_ip = self.remote.to_string();
	stats.local_ip = self.sock.local_addr().unwrap().to_string();

	if !self.env.no_rfc2374 && req.has_options() {
	    self.wrq_oack(Oack::from_request(&req)).await?;
	} else {
	    self.send_ack(SequenceId::new(0)).await?;
	}

	stats.window_size = self.window_size;
	stats.block_size  = self.block_size;

	let alloc_len = 4 + self.block_size as usize;
	let mut buf = Vec::<u8>::with_capacity(alloc_len);
	let mut seq = SequenceId::new(1);

	#[allow(clippy::uninit_vec)]
	unsafe { buf.set_len(alloc_len) };

	loop {
	    let resp = Datagram::recv(&self.sock, buf.as_mut_slice(), &self.remote, self.timeout).await;

	    match resp {
		Ok(Datagram::Data(id, ..)) if id != seq	=> {
		    debug!("got DATA with wrong id #{}...", id.as_u16());
		},

		Ok(Datagram::Data(id, data))		=> {
		    debug!("got DATA #{} with len {}; throwing it away...", id.as_u16(), data.len());
		    self.send_ack(id).await?;
		    seq += 1;

		    stats.xmitsz += data.len() as u64;

		    if data.len() < self.block_size as usize {
			// last packet
			break;
		    }
		},

		Ok(Datagram::Error(code, info))		=> {
		    info!("remote site sent error #{} ({})", code, String::from_utf8_lossy(info));
		    break;
		}

		Err(Error::Timeout)			=> {
		    warn!("timeout while waiting for DATA");
		    return Err(Error::Timeout);
		},

		r					=> {
		    warn!("bad response for WRQ: {:?}", r);
		    return Err(Error::Protocol("bad response to WRQ"));
		},
	    }
	}

	debug!("stats: {:?}", stats);

	Ok(stats)
    }

    async fn run_wrq(self, _req: Request<'_>) -> Result<Stats>
    {
	self.send_err(RequestError::WriteUnsupported.into()).await?;

	Err(Error::NotImplemented)
    }

    async fn rrq_oack(&mut self, mut oack: Oack, file_size: Option<u64>) -> Result<()>
    {
	oack.update_tsize(file_size);
	oack.update_block_size(self.env.max_block_size,   |v| self.block_size = v);
	oack.update_window_size(self.env.max_window_size, |v| self.window_size = v);
	oack.update_timeout(|v| self.timeout = v);

	self.send_oack(oack).await?;

	let mut buf = vec![0u8; GENERIC_PKT_SZ];

	let resp = Datagram::recv(&self.sock, &mut buf, &self.remote, self.timeout).await?;

	match resp {
	    Datagram::Ack(id) if id.as_u16() == 0	=> {},
	    Datagram::Ack(id)	=> {
		warn!("ACK of OACK with invalid id {}", id.as_u16());
		return Err(Error::BadAck);
	    }
	    r			=> {
		warn!("bad response to OACK: {}", r);
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

	self.log_request(&req, "read");

	let mut stats = Stats::default();
	let mut fetcher = Builder::new(self.env).instanciate(&req.get_filename())?;

	stats.filename = req.get_filename().to_string_lossy().into_owned();
	stats.remote_ip = self.remote.to_string();
	stats.local_ip = self.sock.local_addr().unwrap().to_string();

	if let Err(e) = fetcher.open().await {
	    self.send_err(e.clone()).await?;
	    return Err(e);
	}

	let fsize = fetcher.get_size().await;

	if let Some(sz) = fsize {
	    stats.filesize = sz;
	}

	if !self.env.no_rfc2374 && req.has_options() {
	    self.rrq_oack(Oack::from_request(&req), fsize).await?;
	}

	stats.window_size = self.window_size;
	stats.block_size  = self.block_size;

	let mut seq = SequenceId::new(1);
	let mut xfer = Xfer::new(&fetcher, self.block_size, self.window_size);
	let mut retry = RETRY_CNT;
	let mut is_startup = true;
	let mut buf = Vec::<u8>::with_capacity(GENERIC_PKT_SZ);

	#[allow(clippy::uninit_vec)]
	unsafe { buf.set_len(GENERIC_PKT_SZ) };

	loop {
	    match xfer.fill_window(seq, &mut fetcher).await? {
		0	=> {},
		v	=> {
		    debug!("retransmitting {:?}+", seq);

		    stats.retries += 1;
		    stats.wastedsz += v as u64;
		}
	    }

	    if xfer.is_eof() {
		stats.is_complete = true;
		break;
	    }

	    for d in xfer.iter() {
		stats.xmitsz += d.get_data_len() as u64;
		self.send_datagram(d).await?;
	    }

	    debug_assert!(buf.len() == GENERIC_PKT_SZ);

	    let resp = Datagram::recv(&self.sock, buf.as_mut_slice(), &self.remote, self.timeout).await;

	    match resp {
		Err(Error::Timeout) if retry > 0    => {
		    debug!("timeout; resending seq {}", seq.as_u16());
		    retry -= 1;
		    stats.num_timeouts += 1;
		},

		Ok(Datagram::Ack(id))	=> {
		    debug!("got ACK #{}", id.as_u16());
		    is_startup = false;
		    retry = RETRY_CNT;
		    seq = id + 1
		},

		Ok(Datagram::Error(code, info))	if is_startup => {
		    debug!("remote site sent error #{} ({}) on startup; probably just testing for existence",
			   code, String::from_utf8_lossy(info));
		    break;
		}

		Ok(Datagram::Error(code, info)) => {
		    info!("remote site sent error #{} ({})", code, String::from_utf8_lossy(info));
		    break;
		}

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
	    Ok(Datagram::Write(r)) if self.env.wrq_devnull	=> self.run_wrq_devnull(r).await,
	    Ok(Datagram::Write(r))				=> self.run_wrq(r).await,
	    Ok(Datagram::Read(r))				=> self.run_rrq(r).await,
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
