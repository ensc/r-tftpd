mod udpsocket;
mod bucket;
mod socketaddr;

pub use socketaddr::SocketAddr;
pub use udpsocket::{ UdpSocket,
		     RecvInfo as UdpRecvInfo };
pub use bucket::Bucket;

pub trait ToLower {
    type Char;
    fn to_lower(self) -> Vec<Self::Char>;
}

impl ToLower for &[u8] {
    type Char = u8;

    fn to_lower(self) -> Vec<u8>
    {
	let mut res = Vec::<u8>::with_capacity(self.len());

	for c in self {
	    res.push(match c {
		b'A'..=b'Z'	=> *c + 32,
		_		=> *c,
	    });
	}

	res
    }
}
