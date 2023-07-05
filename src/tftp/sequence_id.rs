#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct SequenceId(u16);

impl std::fmt::Display for SequenceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Debug for SequenceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	self.0.fmt(f)
    }
}

impl SequenceId {
    pub const fn new(v: u16) -> Self {
	Self(v)
    }

    pub fn delta(self, other: Self) -> u16 {
	self.0.wrapping_sub(other.0)
    }

    pub const fn as_u16(self) -> u16 {
	self.0
    }

    #[allow(dead_code)]
    pub const fn as_slice(self) -> [u8;2] {
	[(self.0 >> 8) as u8, (self.0 & 0xff) as u8]
    }

    #[inline]
    pub const fn as_u8_hi(self) -> u8 {
	((self.0 >> 8) & 0xff) as u8

    }

    #[allow(clippy::identity_op)]
    #[inline]
    pub const fn as_u8_lo(self) -> u8 {
	((self.0 >> 0) & 0xff) as u8
    }
}

impl PartialOrd for SequenceId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
	// MAX must be odd to avoid equality on a delta of MAX/2
	#[allow(clippy::assertions_on_constants)]
	const _: () = assert!(u16::MAX % 2 != 0);

	match other.0.wrapping_sub(self.0) as u32 {
	    0	=> Some(std::cmp::Ordering::Equal),
	    d	=> (2 * d).partial_cmp(&(u16::MAX as u32)),
	}
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
	self.0 = self.0.wrapping_sub(rhs);
    }
}

impl std::ops::Add<u16> for SequenceId {
    type Output = Self;

    fn add(self, rhs: u16) -> Self::Output {
	Self(self.0.wrapping_add(rhs))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn delta(a: u16, b: u16) -> u16 {
	use SequenceId as Id;

	Id::new(a).delta(Id::new(b))
    }

    #[test]
    fn test_seqid() {
	use SequenceId as Id;

	assert_eq!(delta(    1,     0),     1);
	assert_eq!(delta(    0,     0),     0);
	assert_eq!(delta(    0,     1), 65535);
	assert_eq!(delta(    1, 65535),     2);
	assert_eq!(delta(65535,     1), 65534);

	assert_eq!(Id::new(    0) + 1,     Id::new(    1));
	assert_eq!(Id::new(65535) + 1,     Id::new(    0));
	assert_eq!(Id::new(65535) + 65535, Id::new(65534));

	assert_eq!(Id::new(0x01fe).as_u8_lo(), 0xfe);
	assert_eq!(Id::new(0x0102).as_u8_hi(), 0x01);

	assert_eq!(Id::new(0xfd03).as_u8_lo(), 0x03);
	assert_eq!(Id::new(0xfd03).as_u8_hi(), 0xfd);
    }

    #[test]
    fn test_cmp() {
	use SequenceId as Id;

	assert!(Id::new(0) == Id::new(0));
	assert!(Id::new(0).partial_cmp(&Id::new(0)) == Some(std::cmp::Ordering::Equal));

	assert!(Id::new(0) < Id::new(1));
	assert!(Id::new(0) < Id::new(u16::MAX / 2 - 1));
	assert!(Id::new(0) < Id::new(u16::MAX / 2));
	assert!(Id::new(0) > Id::new(u16::MAX / 2 + 1));
	assert!(Id::new(u16::MAX) < Id::new(0));
    }
}
