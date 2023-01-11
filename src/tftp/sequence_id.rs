const SEQUENCE_SIZE: u32 = 65536;

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
    pub fn new(v: u16) -> Self {
	Self(v)
    }

    pub fn delta(self, other: Self) -> u16 {
	match SEQUENCE_SIZE.is_power_of_two() {
	    true	=> self.0.wrapping_sub(other.0),

	    false if self.0 < other.0	=> u16::try_from(SEQUENCE_SIZE - (other.0 as u32) +
							 (self.0 as u32)).unwrap(),
	    false			=> self.0 - other.0
	}
    }

    pub fn add(self, other: Self) -> Self {
	Self(match SEQUENCE_SIZE.is_power_of_two() {
	    true	=> self.0.wrapping_add(other.0),
	    false	=> (((self.0 as u32) + (other.0 as u32)) % SEQUENCE_SIZE) as u16,
	})
    }

    pub fn as_u16(self) -> u16 {
	self.0
    }

    pub fn as_slice(self) -> [u8;2] {
	[(self.0 >> 8) as u8, (self.0 & 0xff) as u8]
    }

    #[inline]
    pub fn as_u8_hi(self) -> u8 {
	((self.0 >> 8) & 0xff) as u8

    }

    #[allow(clippy::identity_op)]
    #[inline]
    pub fn as_u8_lo(self) -> u8 {
	((self.0 >> 0) & 0xff) as u8
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
	self.0 = match SEQUENCE_SIZE.is_power_of_two() {
	    true	=> self.0.wrapping_sub(rhs),
	    false	=> ((SEQUENCE_SIZE + self.0 as u32 - rhs as u32) % SEQUENCE_SIZE) as u16,
	}
    }
}

impl std::ops::Add<u16> for SequenceId {
    type Output = Self;

    fn add(self, rhs: u16) -> Self::Output {
	Self(match SEQUENCE_SIZE.is_power_of_two() {
	    true	=> self.0.wrapping_add(rhs),
	    false	=> (((self.0 as u32) + (rhs as u32)) % SEQUENCE_SIZE) as u16,
	})
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

	assert_eq!(Id::new(    0).add(Id::new(1)),     Id::new(    1));
	assert_eq!(Id::new(65535).add(Id::new(1)),     Id::new(    0));
	assert_eq!(Id::new(65535).add(Id::new(65535)), Id::new(65534));

	assert_eq!(Id::new(0x01fe).as_u8_lo(), 0xfe);
	assert_eq!(Id::new(0x0102).as_u8_hi(), 0x01);

	assert_eq!(Id::new(0xfd03).as_u8_lo(), 0x03);
	assert_eq!(Id::new(0xfd03).as_u8_hi(), 0xfd);
    }
}
