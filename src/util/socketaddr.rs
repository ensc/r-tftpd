use std::net::{ IpAddr };
use std::os::unix::prelude::RawFd;

use nix::sys::socket::{self, SockaddrLike, SockaddrStorage};

use crate::{ Result, Error };

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SocketAddr(std::net::SocketAddr);

impl std::fmt::Display for SocketAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<SockaddrStorage> for SocketAddr {
    type Error = Error;

    fn try_from(addr: SockaddrStorage) -> std::result::Result<Self, Self::Error> {
	if let Some(ip) = addr.as_sockaddr_in() {
	    use std::net::SocketAddrV4 as V4;

	    Ok(Self(V4::new(ip.ip().into(), ip.port()).into()))
	} else if let Some(ip) = addr.as_sockaddr_in6() {
	    use std::net::SocketAddrV6 as V6;

	    Ok(Self(V6::new(ip.ip(), ip.port(),
			    ip.flowinfo(), ip.scope_id()).into()))
	} else {
	    Err(crate::Error::Internal("unsupported address type"))
	}
    }
}

impl SocketAddr {
    pub fn new(ip: IpAddr, port: u16) -> Self {
	Self(std::net::SocketAddr::new(ip, port))
    }

    pub fn from_raw_fd(fd: RawFd) -> Result<Self>
    {
	socket::getsockname::<SockaddrStorage>(fd)
	    .map_err(|e| e.into())
	    .and_then(Self::try_from)
    }

    /// # Safety
    ///
    /// can leak a file descriptor
    pub unsafe fn socket(&self) -> Result<RawFd> {
	use socket::SockFlag as SF;

	socket::socket(self.get_af(), socket::SockType::Datagram,
		       SF::SOCK_CLOEXEC | SF::SOCK_NONBLOCK, None)
	    .map_err(|e| e.into())
    }

    pub fn get_af(&self) -> socket::AddressFamily {
	use std::net::SocketAddr as SA;
	use socket::AddressFamily as AF;

	match self.0 {
	    SA::V4(_)	=> AF::Inet,
	    SA::V6(_)	=> AF::Inet6,
	}
    }

    pub fn as_std(&self) -> &std::net::SocketAddr
    {
	&self.0
    }

    pub fn as_nix(&self) -> Box<dyn SockaddrLike + Send>
    {
	use std::net::SocketAddr as SA;

	match self.0 {
	    SA::V4(a)	=> Box::new(socket::SockaddrIn::from(a)),
	    SA::V6(a)	=> Box::new(socket::SockaddrIn6::from(a)),
	}
    }
}
