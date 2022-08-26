use lazy_static::__Deref;

use std::time::Duration;
use crate::{ Error, Result };

#[derive(Debug, PartialEq, Eq)]
pub enum CacheControl {
    MaxAge(Duration),
    SMaxAge(Duration),
    NoCache,
    MustRevalidate,
    ProxyRevalidate,
    Private,
    Public,
    Immutable,
    Other,
}

pub type Iterator<'a> = super::multi_header::MultiHeaderIterator<'a, CacheControl>;

impl TryFrom<(&[u8], Option<&[u8]>)> for CacheControl {
    type Error = Error;

    fn try_from((key, val): (&[u8], Option<&[u8]>)) -> Result<Self> {
	use super::duration_from_s as tm_s;

	let key = super::to_lower(key);

	match key.deref() {
	    b"max-age"		=> Ok(Self::MaxAge(tm_s(val.ok_or(Error::StringConversion)?)?)),
	    b"s-maxage"		=> Ok(Self::SMaxAge(tm_s(val.ok_or(Error::StringConversion)?)?)),
	    b"no-cache"		=> Ok(Self::NoCache),
	    b"must-revalidate"	=> Ok(Self::MustRevalidate),
	    b"proxy-revalidate"	=> Ok(Self::ProxyRevalidate),
	    b"private"		=> Ok(Self::Private),
	    b"public"		=> Ok(Self::Public),
	    b"immutable"	=> Ok(Self::Immutable),
	    k			=> {
		debug!("unsupported cache-control {:?}", k);
		Ok(Self::Other)
	    }
	}
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_00() {
	use super::CacheControl as C;
	use reqwest::header as H;
	use reqwest::header::HeaderMap;

	let mut map = HeaderMap::new();

	map.append(H::CACHE_CONTROL, "max-age=23".parse().unwrap());
	map.append(H::CACHE_CONTROL, "s-maxage=42,no-cache".parse().unwrap());
	map.append(H::CACHE_CONTROL, "must-ReValidate".parse().unwrap());
	map.append(H::CACHE_CONTROL, "PROXY-revalidate".parse().unwrap());
	map.append(H::CACHE_CONTROL, "private,PUBLIC".parse().unwrap());
	map.append(H::CACHE_CONTROL, "immutable".parse().unwrap());
	map.append(H::CACHE_CONTROL, "xxx-unsupported".parse().unwrap());

	let mut step = 0;
	for i in Iterator::new(map.get_all(H::CACHE_CONTROL)) {
	    match step {
		0	=> assert_eq!(i.unwrap(), C::MaxAge(Duration::from_secs(23))),
		1	=> assert_eq!(i.unwrap(), C::SMaxAge(Duration::from_secs(42))),
		2	=> assert_eq!(i.unwrap(), C::NoCache),
		3	=> assert_eq!(i.unwrap(), C::MustRevalidate),
		4	=> assert_eq!(i.unwrap(), C::ProxyRevalidate),
		5	=> assert_eq!(i.unwrap(), C::Private),
		6	=> assert_eq!(i.unwrap(), C::Public),
		7	=> assert_eq!(i.unwrap(), C::Immutable),
		8	=> assert_eq!(i.unwrap(), C::Other),
		_	=> panic!("too much results"),
	    }

	    step += 1;
	}

	assert_eq!(step, 9);
    }
}
