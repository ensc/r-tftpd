use crate::{ Result, Error };
use std::io::IoSlice;
use std::os::unix::io::RawFd;
use std::net::IpAddr;
use std::os::unix::prelude::AsRawFd;
use nix::sys::socket::{self, SockaddrLike, SockaddrStorage};
use tokio::io::unix::AsyncFd;
use nix::libc;

use super::SocketAddr;

fn sockaddrlike_to_storage(addr: &dyn SockaddrLike) -> SockaddrStorage
{
    unsafe { SockaddrStorage::from_raw(addr.as_ptr(), Some(addr.len())) }.unwrap()
}

trait OptUnwrap<T: Sized> {
    fn unwrap_opt(self) -> Result<T>;
}

impl <T: Sized> OptUnwrap<T> for Option<T>
{
    fn unwrap_opt(self) -> Result<T> {
        self.ok_or(Error::Internal("Option is None"))
    }
}

#[derive(Default, Debug)]
struct RecvInfoOpt {
    size:	usize,
    if_idx:	Option<libc::c_int>,
    local:	Option<IpAddr>,
    remote:	Option<SocketAddr>,
    spec_dest:	Option<IpAddr>,
}

impl RecvInfoOpt {
    fn convert_v4(ip_raw: nix::libc::in_addr) -> Option<std::net::IpAddr>
    {
	Some(std::net::Ipv4Addr::from(ip_raw.s_addr.to_be()).into())
    }

    fn convert_v6(ip_raw: nix::libc::in6_addr) -> Option<std::net::IpAddr>
    {
	Some(std::net::Ipv6Addr::from(ip_raw.s6_addr).into())
    }

    pub fn set_local_v4(&mut self, ip_raw: nix::libc::in_addr) {
	self.local = Self::convert_v4(ip_raw);
    }

    pub fn set_local_v6(&mut self, ip_raw: nix::libc::in6_addr) {
	self.local = Self::convert_v6(ip_raw);
    }

    pub fn set_spec_dest_v4(&mut self, ip_raw: nix::libc::in_addr) {
	self.spec_dest = Self::convert_v4(ip_raw);
    }
}

#[derive(Clone, Copy)]
pub struct RecvInfo {
    pub size:	usize,
    if_idx:	libc::c_int,
    pub local:	IpAddr,
    pub remote:	SocketAddr,
}


impl RecvInfo {
    pub fn local(&self) -> IpAddr {
	// TODO: handle spec_dest?
	self.local
    }
}

impl TryFrom<RecvInfoOpt> for RecvInfo {
    type Error = Error;

    fn try_from(v: RecvInfoOpt) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
	    size:	v.size,
	    if_idx:	v.if_idx.unwrap_opt()?,
	    // TODO: prefer spec_dest when set?
	    local:	v.local.unwrap_opt()?,
	    remote:	v.remote.unwrap_opt()?,
	})
    }
}

pub struct UdpSocket {
    fd:		RawFd,
    af:		socket::AddressFamily,
    // must be an `Option` so that we can control the destruction order of
    // 'fd' itself and 'async_fd' in drop()
    async_fd:	Option<AsyncFd<RawFd>>,
}

impl UdpSocket {
    fn get_fd(&self) -> &AsyncFd<RawFd>
    {
	self.async_fd.as_ref().unwrap()
    }

    pub async fn sendto(&self, buf: &[u8], addr: &SocketAddr) -> Result<()>
    {
	use socket::MsgFlags as M;
	use nix::Error as E;

	let addr = addr.as_nix();

	loop {
	    let mut async_guard = self.get_fd().writable().await?;

	    match self.sendto_sync(buf, &*addr, M::MSG_NOSIGNAL | M::MSG_DONTWAIT) {
		Ok(_)			=> break Ok(()),
		Err(E::EAGAIN)		=> async_guard.clear_ready(),
		Err(e)			=> break Err(e.into())
	    };
	}
    }

    fn sendto_sync(&self, buf: &[u8], addr: &dyn SockaddrLike,
		   flags: socket::MsgFlags) -> nix::Result<()>
    {
	use nix::Error as E;

	match socket::sendto(self.fd, buf, addr, flags) {
	    Ok(sz) if sz == buf.len()	=> Ok(()),
	    Ok(sz)			=> {
		error!("sent only {} bytes out of {} ones", sz, buf.len());
		Err(E::ENOPKG)
	    },
	    Err(e)			=> Err(e)
	}
    }

    pub async fn sendmsg(&self, iov: &[IoSlice<'_>], addr: &SocketAddr) -> Result<()>
    {
	use socket::MsgFlags as M;
	use nix::Error as E;

	let addr = addr.as_nix();

	loop {
	    let mut async_guard = self.get_fd().writable().await?;

	    match self.sendmsg_sync(iov, &*addr, M::MSG_NOSIGNAL | M::MSG_DONTWAIT) {
		Ok(_)			=> break Ok(()),
		Err(E::EAGAIN)		=> async_guard.clear_ready(),
		Err(e)			=> break Err(e.into())
	    }
	}
    }

    fn sendmsg_sync(&self, iov: &[IoSlice<'_>], addr: &dyn SockaddrLike,
		    flags: socket::MsgFlags) -> nix::Result<()>
    {
	use nix::Error as E;

	let total_sz: usize = iov.iter().map(|v| v.len()).sum();

	// TODO: this is too expensive but nix api makes it difficulty/impossible
	// to use the `dyn SockaddrLike` object directly
	let addr = sockaddrlike_to_storage(addr);

	match socket::sendmsg(self.fd, iov, &[], flags, Some(&addr)) {
	    Ok(sz) if sz == total_sz	=> Ok(()),
	    Ok(sz)			=> {
		error!("sent only {} bytes out of {} ones", sz, total_sz);
		Err(E::ENOPKG)
	    },
	    Err(e)			=> Err(e),
	}
    }

    pub async fn recvfrom(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)>
    {
	use nix::Error as E;

	loop {
	    let mut async_guard = self.get_fd().readable().await?;

	    match socket::recvfrom::<SockaddrStorage>(self.fd, buf) {
		Ok((sz, Some(addr)))	=> break Ok((sz, addr.try_into()?)),
		Ok((_, None))		=> break Err(Error::Internal("no address from recvfrom")),
		Err(E::EAGAIN)		=> async_guard.clear_ready(),
		Err(e)			=> break Err(e.into())
	    }
	}
    }

    pub async fn recvmsg(&self, buf: &mut [u8]) -> Result<RecvInfo>
    {
	use socket::MsgFlags as M;
	use nix::Error as E;

	loop {
	    let mut async_guard = self.get_fd().readable().await?;

	    match self.recvmsg_sync(buf, M::MSG_DONTWAIT) {
		Ok(info)			=> break Ok(info),
		Err(Error::Nix(E::EAGAIN))	=> async_guard.clear_ready(),
		Err(e)				=> break Err(e),
	    }
	}
    }

    fn recvmsg_sync(&self, buf: &mut [u8], flags: socket::MsgFlags) -> Result<RecvInfo>
    {

	let mut iov = [std::io::IoSliceMut::new(buf)];
	let mut cmsg = nix::cmsg_space!(libc::in6_pktinfo,
					libc::in_pktinfo);

	let recv = socket::recvmsg::<SockaddrStorage>(self.fd, &mut iov, Some(&mut cmsg), flags)?;

	let mut res = RecvInfoOpt {
	    size:	recv.bytes,
	    ..Default::default()
	};

	for msg in recv.cmsgs() {
	    use socket::ControlMessageOwned as C;

	    match msg {
		C::Ipv4PacketInfo(i)	=> {
		    res.set_local_v4(i.ipi_addr);
		    res.set_spec_dest_v4(i.ipi_spec_dst);
		    res.if_idx = Some(i.ipi_ifindex);
		},

		C::Ipv6PacketInfo(i)	=> {
		    res.set_local_v6(i.ipi6_addr);
		    res.spec_dest = None;
		    res.if_idx = Some(i.ipi6_ifindex as libc::c_int);
		},

		m			=> {
		    debug!("unhandled msg {:?}", m);
		},
	    }
	}

	match recv.address {
	    Some(addr)	=> res.remote = Some(SocketAddr::try_from(addr)?),
	    None	=> {
		warn!("missing remote address");
		return Err(Error::Internal("missing remote address"));
	    },
	};

	res.try_into()
    }

    pub fn bind(addr: &SocketAddr) -> Result<Self> {
	let fd = unsafe { addr.socket() }?;

	let af = addr.get_af();
	let addr = addr.as_nix();

	match socket::bind(fd, &*addr) {
	    Ok(_)	=> Ok(Self {
		fd:		fd,
		af:		af,
		async_fd:	Some(AsyncFd::new(fd)?),
	    }),

	    Err(e)	=> {
		unsafe { libc::close(fd) };
		Err(std::io::Error::from(e).into())
	    }
	}
    }

    pub fn from_raw(fd: RawFd) -> Result<Self> {
	let addr = SocketAddr::from_raw_fd(fd)?;

	Ok(Self {
	    fd:		fd,
	    af:		addr.get_af(),
	    async_fd:	Some(AsyncFd::new(fd)?),
	})
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
	let addr: SockaddrStorage = socket::getsockname(self.fd.as_raw_fd())?;

	addr.try_into()
    }

    pub fn set_request_pktinfo(&mut self) -> Result<()> {
	use socket::AddressFamily as AF;
	use nix::sys::socket::sockopt as O;

	match self.af {
	    AF::Inet	=> socket::setsockopt(self.fd, O::Ipv4PacketInfo, &true),
	    AF::Inet6	=> socket::setsockopt(self.fd, O::Ipv6RecvPacketInfo, &true),
	    _		=> return Err(Error::Internal("unexpected af")),
	}?;

	Ok(())
    }

    pub fn set_nonblocking(&self) -> Result<()> {
	let rc = unsafe { libc::fcntl(self.fd, libc::F_GETFL) };

	if rc < 0 {
	    return Err(std::io::Error::last_os_error().into());
	}

	let flags = rc as u32;

	if flags & (libc::O_NONBLOCK as u32) != 0 {
	    return Ok(());
	}

	let rc = unsafe { libc::fcntl(self.fd, libc::F_SETFL, flags | (libc::O_NONBLOCK as u32)) };

	if rc < 0 {
	    return Err(std::io::Error::last_os_error().into());
	}

	Ok(())
    }
}

impl Drop for UdpSocket {
    fn drop(&mut self) {
	self.async_fd = None;
        unsafe { libc::close(self.fd) };
    }
}
