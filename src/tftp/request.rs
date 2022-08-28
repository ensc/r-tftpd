use std::time::Duration;

use super::{ RequestError as E, RequestResult, Mode };

fn try_ranged_from<T, const MIN: u64, const MAX: u64>(s: &[u8]) -> RequestResult<T>
where
    T: TryFrom<u64>,
{
    let mut tmp: u64 = 0;

    for c in s {
	let v = match c {
	    b'0'..=b'9'	=> *c - b'0',
	    c		=> return Err(E::BadDigit(*c)),
	} as u64;

	tmp = tmp
	    .checked_mul(10)
	    .and_then(|t| t.checked_add(v))
	    .ok_or(E::NumberOutOfRange)?;
    }

    if tmp < MIN || tmp > MAX {
	return Err(E::NumberOutOfRange);
    }

    tmp.try_into().map_err(|_| E::NumberOutOfRange)
}

#[derive(Debug, Clone)]
pub struct Request<'a> {
    filename:		&'a[u8],
    pub mode:		Mode,
    pub block_size:	Option<u16>,
    pub timeout:	Option<Duration>,
    pub window_size:	Option<u16>,
    pub tsize:		Option<u64>,
}

pub enum Dir {
    Read,
    Write,
}

impl <'a> Request<'a> {
    pub fn has_options(&self) -> bool {
	self.block_size.is_some() ||
	    self.timeout.is_some() ||
	    self.window_size.is_some() ||
	    self.tsize.is_some()
    }

    pub fn from_slice(data: &'a [u8], dir: Dir) -> RequestResult<Self> {
	if data.is_empty() {
	    return Err(E::TooShort);
	}

	if data[data.len() - 1] != b'\0' {
	    return Err(E::MissingZero);
	}

	let mut iter = data[..data.len() - 1].split(|c| *c == b'\0');

	let filename = iter.next().ok_or(E::MissingFilename)?;
	if filename.is_empty() {
	    return Err(E::MissingFilename);
	}

	let mode = iter.next().ok_or(E::MissingMode)?;
	let mode = Mode::try_from(mode)?;

	let mut res = Self {
	    filename:		filename,
	    mode:		mode,

	    block_size:		None,
	    timeout:		None,
	    window_size:	None,
	    tsize:		None,
	};

	while let Some(v) = iter.next() {
	    use crate::util::ToLower;

	    let name = v.to_lower();
	    let arg = iter.next().ok_or(E::MissingArgument)?;

	    match name.as_slice() {
		b"blksize"	=> res.block_size = Some(try_ranged_from::<u16, 8, 65464>(arg)?),
		b"timeout"	=> res.timeout = Some(Duration::from_secs(try_ranged_from::<u64, 0, 65536>(arg)?)),
		b"tsize"	=> res.tsize = Some(match dir {
		    Dir::Read	=> try_ranged_from::<u64, 0, 0>(arg),
		    Dir::Write	=> try_ranged_from::<u64, 0, 4_294_967_295>(arg),
		}?),
		b"windowsize"	=> res.window_size = Some(try_ranged_from::<u16, 1, 65535>(arg)?),
		_		=> warn!("unsupported {:?}={:?} option", name, arg),
	    }
	}

	Ok(res)
    }

    pub fn get_filename(&self) -> std::path::PathBuf {
	use std::os::unix::ffi::OsStrExt;

	let tmp = std::ffi::OsStr::from_bytes(self.filename);

	tmp.into()
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_range() {
	assert_eq!(try_ranged_from::<u16, 0, 10>(b"000"),     RequestResult::Ok(0));
	assert_eq!(try_ranged_from::<u16, 0, 10>(b"001"),     RequestResult::Ok(1));
	assert_eq!(try_ranged_from::<u16, 0, 10>(b"10"),      RequestResult::Ok(10));
	assert_eq!(try_ranged_from::<u16, 0, 10>(b"010"),     RequestResult::Ok(10));
	assert_eq!(try_ranged_from::<u16, 0, 10>(b"011"),     RequestResult::Err(E::NumberOutOfRange));
	assert_eq!(try_ranged_from::<u16, 1, 10>(b"0"),       RequestResult::Err(E::NumberOutOfRange));
	assert_eq!(try_ranged_from::<u8,  1, 1000>(b"200"),   RequestResult::Ok(200));
	assert_eq!(try_ranged_from::<u8,  1, 1000>(b"300"),   RequestResult::Err(E::NumberOutOfRange));
	assert_eq!(try_ranged_from::<u128, 1, 18446744073709551615>(b"18446744073709551615"),
		   RequestResult::Ok(18446744073709551615));
	assert_eq!(try_ranged_from::<u128, 1, 18446744073709551615>(b"18446744073709551616"),
		   RequestResult::Err(E::NumberOutOfRange));
	assert_eq!(try_ranged_from::<u128, 1, 18446744073709551615>(b"184467440737095516150"),
		   RequestResult::Err(E::NumberOutOfRange));
    }
}
