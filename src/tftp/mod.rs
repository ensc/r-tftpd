mod request;
mod datagram;
mod mode;
mod errors;
mod session;
mod oack;
mod xfer;

pub use datagram::Datagram;
use request::Request;
use mode::Mode;
use oack::Oack;
use xfer::Xfer;

pub use errors::{ RequestError, RequestResult };
pub use session::Session;
pub use session::Stats as SessionStats;

pub const SEQUENCE_SIZE: u32 = 65536;


#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct SequenceId(u16);

impl SequenceId {
    pub fn new(v: u16) -> Self {
	Self(v)
    }

    pub fn delta(self, other: Self) -> u16 {
	if self.0 < other.0 {
	    u16::try_from(SEQUENCE_SIZE - (other.0 as u32) + (self.0 as u32)).unwrap()
	} else {
	    self.0 - other.0
	}
    }

    pub fn add(self, other: Self) -> Self {
	Self((((self.0 as u32) + (other.0 as u32)) % SEQUENCE_SIZE) as u16)
    }

    pub fn as_u16(self) -> u16 {
	self.0
    }

    pub fn as_slice(self) -> [u8;2] {
	[(self.0 >> 8) as u8, (self.0 & 0xff) as u8]
    }

    #[cfg(test)]
    pub fn in_range(self, a: Self, b: Self) -> bool
    {
	assert!(a.0 != b.0);

	(a.0 < self.0 && self.0 < b.0) ||
	    (self.0 < b.0 && b.0 < a.0)
    }
}

impl std::ops::AddAssign<u16> for SequenceId {
    fn add_assign(&mut self, rhs: u16) {
        self.0 = (*self + rhs).0;
    }
}

#[cfg(test)]
impl std::ops::SubAssign<u16> for SequenceId {
    fn sub_assign(&mut self, rhs: u16) {
        self.0 = ((SEQUENCE_SIZE + self.0 as u32 - rhs as u32) % SEQUENCE_SIZE) as u16;
    }
}

impl std::ops::Add<u16> for SequenceId {
    type Output = Self;

    fn add(self, rhs: u16) -> Self::Output {
	Self((((self.0 as u32) + (rhs as u32)) % SEQUENCE_SIZE) as u16)
    }
}

impl std::fmt::Debug for SequenceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	self.0.fmt(f)
    }
}
