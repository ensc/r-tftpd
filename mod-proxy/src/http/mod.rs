mod cache_control;
mod header;
mod errors;
mod cache_info;
mod time;

#[path = "multi-header.rs"]
mod multi_header;

pub use cache_info::CacheInfo;
pub use cache_control::{ CacheControl,
			 Iterator as CacheControlIterator };
pub use header::HttpHeader;

pub use time::Time;

pub fn to_lower(s: &[u8]) -> std::borrow::Cow<'_, [u8]>
{
    let mut tmp = std::borrow::Cow::Borrowed(s);

    for (pos, c) in s.iter().enumerate() {
	#[allow(clippy::single_match)]
	match c {
	    b'A'..=b'Z'	=> tmp.to_mut()[pos] = c + 32,
	    _		=> {},
	}
    }

    tmp
}

pub fn as_u64(s: &[u8]) -> crate::Result<u64> {
    use crate::Error;

    let mut res: u64 = 0;

    for c in s {
	res = res.checked_mul(10).ok_or(Error::StringConversion)?;

	match c {
	    b'0'..=b'9'	=> res = res.checked_add((c - b'0') as u64).ok_or(Error::StringConversion)?,
	    _		=> return Err(Error::StringConversion),
	}
    }

    Ok(res)
}

fn duration_from_s(s: &[u8]) -> crate::Result<std::time::Duration> {
    Ok(std::time::Duration::from_secs(as_u64(s)?))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_to_lower() {
	assert_eq!(to_lower(b"abc").as_ref(), b"abc");
	assert_eq!(to_lower(b"Abc").as_ref(), b"abc");
	assert_eq!(to_lower(b"012").as_ref(), b"012");
    }
}
