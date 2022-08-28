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

impl std::fmt::Display for Datagram<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
	    Self::Read(r)	=> write!(f, "RRQ({:?})", r),
	    Self::Write(r)	=> write!(f, "WRQ({:?})", r),
	    Self::Data(id, d)	=> write!(f, "DATA({}, ..{})", id, d.len()),
	    Self::Ack(id)	=> write!(f, "ACK({}", id),
	    Self::Error(err, s)	=> write!(f, "ERROR({}, \"{}\")", err, String::from_utf8_lossy(s)),
	    Self::OAck		=> write!(f, "OACK"),
	}
    }
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

	    // ACK
	    4 if v.len() == 4	=> Datagram::Ack(v.get_sequence_id(2)),
	    4	=> Err(E::MalformedAck)?,

	    // ERROR
	    5 if v.last() == Some(&b'\0')		=> {
		v.assert_len(2 + 2 + 1)?;
		Datagram::Error(v.get_u16(2), &v[4..v.len() - 1])
	    },
	    5	=> Err(E::MissingZero)?,

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

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! assert_datagram {
	($v:expr, $pat:pat) => {
	    assert_datagram!($v, $pat, true)
	};

	($v:expr, $pat:pat, $test:expr) => {
	    assert_datagram!($v, pat => Ok($pat), test => $test)
	};

	($v:expr, err => $pat:pat) => {
	    assert_datagram!($v, pat => Err(Error::RequestError($pat)), test => true)
	};

	($v:expr, err => $pat:pat, $test:expr) => {
	    assert_datagram!($v, pat => Err(Error::RequestError($pat)), test => $test)
	};

	($v:expr, pat => $pat:pat, test => $test:expr) => {
	    match Datagram::try_from($v as &[u8]) {
		$pat if $test		=> {},
		Ok(v)			=> panic!("bad result {}", v),
		Err(e)			=> panic!("err {:?}", e),
	    }
	};
    }

    #[test]
    #[allow(unused_variables)]
    fn test_datagram() {
	use super::super::RequestError as RE;

	// RRQ
	assert_datagram!(b"\x00\x01file\x00octet\x00",    Datagram::Read(_rrq));
	assert_datagram!(b"\x00\x01file\x00netascii\x00", Datagram::Read(_rrq));
	assert_datagram!(b"\x00\x01file\x00binary\x00",   Datagram::Read(_rrq));
	assert_datagram!(b"\x00\x01file\x00mail\x00",     Datagram::Read(_rrq));
	assert_datagram!(b"\x00\x01file\x00mail\x00",     Datagram::Read(_rrq));

	assert_datagram!(b"\x00\x01file\x00XXX\x00", err => RE::BadMode);
	assert_datagram!(b"\x00\x01file\x00octet",   err => RE::MissingZero);
	assert_datagram!(b"\x00\x01file\x00",        err => RE::MissingMode);
	assert_datagram!(b"\x00\x01file",            err => RE::MissingZero);
	assert_datagram!(b"\x00\x01",                err => RE::TooShort);

	assert_datagram!(b"\x00\x01file\x00octet\x00\
			   blksize\x002000\x00\
			   unsupported\x001234\x00\
			   timeout\x005\x00\
			   tsize\x000\x00\
			   windowsize\x0064\x00", Datagram::Read(rrq),
			 rrq.get_filename().to_str().unwrap() == "file" &&
			 rrq.mode == super::super::Mode::Octet &&
			 rrq.block_size == Some(2000) &&
			 rrq.timeout == Some(std::time::Duration::from_secs(5)) &&
			 rrq.tsize == Some(0) &&
			 rrq.window_size == Some(64));

	assert_datagram!(b"\x00\x01file\x00octet\x00\
			   tsize\x0042\x00", err => RE::NumberOutOfRange);

	assert_datagram!(b"\x00\x01file\x00octet\x00\
			   tsize\x0042\x00", err => RE::NumberOutOfRange);

	assert_datagram!(b"\x00\x01file\x00octet\x00\
			   windowsize\x000\x00", err => RE::NumberOutOfRange);

	assert_datagram!(b"\x00\x01file\x00octet\x00\
			   blksize\x007\x00", err => RE::NumberOutOfRange);

	// WRQ
	assert_datagram!(b"\x00\x02file\x00octet\x00",    Datagram::Write(_wrq));
	assert_datagram!(b"\x00\x02file\x00octet\x00\
			   tsize\x0042\x00", Datagram::Write(wrq),
			 wrq.get_filename().to_str().unwrap() == "file" &&
			 wrq.mode == super::super::Mode::Octet &&
			 wrq.tsize == Some(42));

	// DATA
	assert_datagram!(b"\x00\x03\x01\x02data" as &[u8], Datagram::Data(seq, data),
			 seq.as_u16() == 0x0102 && data == b"data");

	// ACK
	assert_datagram!(b"\x00\x04\x01\x02", Datagram::Ack(seq),
			 seq.as_u16() == 0x0102);

	assert_datagram!(b"\x00\x04\x01\x02X", err => RE::MalformedAck);

	// ERROR
	assert_datagram!(b"\x00\x05\x01\x02error\x00", Datagram::Error(code, msg),
			 code == 0x0102 && msg == b"error");
	assert_datagram!(b"\x00\x05\x01\x02\x00", Datagram::Error(code, msg),
			 code == 0x0102 && msg == b"");

	assert_datagram!(b"\x00\x05\x01\x02error", err => RE::MissingZero);

	// OACK
	assert_datagram!(b"\x00\x06...", Datagram::OAck);

	// misc errors
	assert_datagram!(b"\x00\x07", err => RE::BadOpCode(c), c == 7);
	assert_datagram!(b"\x00",     err => RE::TooShort);
	assert_datagram!(b"",         err => RE::TooShort);
    }
}
