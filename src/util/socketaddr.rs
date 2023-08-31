use std::net::IpAddr;
use std::os::fd::{OwnedFd, AsRawFd};

use nix::sys::socket::{self, SockaddrStorage};

use crate::{ Result, Error };

#[derive(Clone, Debug)]
pub struct SocketAddr(SockaddrStorage);

impl std::fmt::Display for SocketAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq for SocketAddr {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl TryFrom<SockaddrStorage> for SocketAddr {
    type Error = Error;

    fn try_from(addr: SockaddrStorage) -> std::result::Result<Self, Self::Error> {
	use socket::SockaddrLike;
	use socket::AddressFamily as AF;

	match addr.family() {
	    Some(AF::Inet) |
	    Some(AF::Inet6)	=> Ok(Self(addr)),
	    _			=> Err(crate::Error::Internal("unsupported address type"))
	}
    }
}

impl SocketAddr {
    pub fn new(ip: IpAddr, port: u16) -> Self {
	let addr = std::net::SocketAddr::new(ip, port);

	Self(addr.into())
    }

    pub fn from_raw_fd<T: AsRawFd>(fd: &T) -> Result<Self>
    {
	socket::getsockname::<SockaddrStorage>(fd.as_raw_fd())
	    .map_err(|e| e.into())
	    .and_then(Self::try_from)
    }

    pub fn to_stdnet(&self) -> std::net::SocketAddr {
	if let Some(ip) = self.0.as_sockaddr_in() {
	    use std::net::SocketAddrV4 as V4;

	    V4::new(ip.ip().into(), ip.port()).into()
	} else if let Some(ip) = self.0.as_sockaddr_in6() {
	    use std::net::SocketAddrV6 as V6;

	    V6::new(ip.ip(), ip.port(), ip.flowinfo(), ip.scope_id()).into()
	} else {
	    panic!("addr {:?} is not ipv4 or ipv6", self.0);
	}
    }

    pub fn socket(&self) -> Result<OwnedFd> {
	use socket::SockFlag as SF;

	socket::socket(self.get_af(), socket::SockType::Datagram,
		       SF::SOCK_CLOEXEC | SF::SOCK_NONBLOCK, None)
	    .map_err(|e| e.into())
    }

    pub fn get_af(&self) -> socket::AddressFamily {
	use socket::SockaddrLike;

	self.0.family().unwrap()
    }

    pub fn as_nix(&self) -> &SockaddrStorage
    {
	&self.0
    }
}
