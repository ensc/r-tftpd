use std::net::IpAddr;
use std::os::fd::{OwnedFd, AsRawFd, BorrowedFd};

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

    pub fn from_fd(fd: BorrowedFd) -> Result<Self>
    {
	socket::getsockname::<SockaddrStorage>(fd.as_raw_fd())
	    .map_err(|e| e.into())
	    .and_then(Self::try_from)
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
