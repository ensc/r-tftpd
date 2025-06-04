mod udpsocket;
mod bucket;
mod socketaddr;

pub use socketaddr::SocketAddr;
pub use udpsocket::{ UdpSocket,
		     RecvInfo as UdpRecvInfo };
pub use bucket::Bucket;

mod uninit;
pub use uninit::*;

lazy_static::lazy_static!{
    pub static ref RUST_FMT: num_format::CustomFormat =
	num_format::CustomFormat::builder()
	.separator("_")
	.build()
	.unwrap();
}

pub trait ToFormatted {
    fn to_formatted(&self) -> String
    where
	Self: num_format::ToFormattedString,
    {
	self.to_formatted_string(&*RUST_FMT)
    }
}

impl <T: num_format::ToFormattedString> ToFormatted for T {}

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
