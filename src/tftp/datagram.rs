use std::time::Duration;

use crate::{ Error, Result };
use crate::util::{ UdpSocket, SocketAddr };
use super::{ Request, RequestError as E, RequestResult, SequenceId };


#[derive(Debug)]
pub enum Datagram<'a> {
    Read(Request<'a>),
    Write(Request<'a>),
    Data(SequenceId, &'a[u8]),
    Ack(SequenceId),
    Error(u16, &'a[u8]),
    OAck
}

trait TftpSlice {
    fn assert_len(&self, sz: usize) -> RequestResult<()>;
    fn get_u16(&self, idx: usize) -> u16;
    fn get_sequence_id(&self, idx: usize) -> SequenceId;
}

impl TftpSlice for &[u8]
{
    fn assert_len(&self, sz: usize) -> RequestResult<()>
    {
	if self.len() < sz {
	    return Err(E::TooShort);
	}

	Ok(())
    }

    fn get_u16(&self, idx: usize) -> u16
    {
	(self[idx] as u16) << 8 | (self[idx + 1] as u16)
    }

    fn get_sequence_id(&self, idx: usize) -> SequenceId
    {
	SequenceId::new(self.get_u16(idx))
    }
}

impl <'a> TryFrom<&'a[u8]> for Datagram<'a> {
    type Error = Error;

    #[instrument(level = "trace", skip(v), ret)]
    fn try_from(v: &'a [u8]) -> Result<Self> {
	use super::request::Dir;

	v.assert_len(2)?;
	let op = v.get_u16(0);

	Ok(match op {
	    1	=> {
		v.assert_len(2 + 1)?;
		Datagram::Read(Request::from_slice(&v[2..], Dir::Read)?)
	    },
	    2	=> {
		v.assert_len(2 + 1)?;
		Datagram::Write(Request::from_slice(&v[2..], Dir::Write)?)
	    },
	    3	=> {
		v.assert_len(2 + 2)?;
		Datagram::Data(v.get_sequence_id(2), &v[4..])
	    },
	    4	=> {
		v.assert_len(2 + 2)?;
		Datagram::Ack(v.get_sequence_id(2))
	    },
	    5	=> {
		v.assert_len(2 + 2)?;
		Datagram::Error(v.get_u16(2), &v[4..])
	    },
	    6	=> Datagram::OAck,
	    _	=> Err(E::BadOpCode(op))?,
	})
    }
}

impl <'a> Datagram<'a> {
    async fn recv_inner(sock: &UdpSocket,
			buf: &'a mut [u8], exp_addr: &SocketAddr) -> Result<Datagram<'a>>
    {
	loop {
	    let (len, addr) = sock.recvfrom(buf).await?;

	    if &addr != exp_addr {
		error!("unexpected address: {} vs {}", addr, exp_addr);
		// TODO: audit this event?
		continue;
	    }

	    return Self::try_from(&buf[0..len])
	}
    }

    pub async fn recv(sock: &UdpSocket,
		      buf: &'a mut [u8], exp_addr: &SocketAddr, to: Duration) -> Result<Datagram<'a>>
    {
	use tokio::time::timeout;

	timeout(to, Self::recv_inner(sock, buf, exp_addr)).await
	    .map_err(|_| Error::Timeout)
	    .and_then(|v| v)
    }

    pub fn is_ack(&self) -> bool {
	matches!(self, Self::Ack(_))
    }

    pub fn get_data_len(&self) -> usize {
	match self {
	    &Self::Data(_, d)	=> d.len(),
	    _			=> 0,
	}
    }
}
