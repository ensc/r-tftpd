use crate::{ Result, Error };
use std::time::SystemTime;
use super::Time;

pub trait HttpHeader {
    fn as_system_time(&self, name: reqwest::header::HeaderName) -> Result<Option<SystemTime>>;
    fn as_time(&self, reftm: Time, name: reqwest::header::HeaderName) -> Result<Option<Time>>;
    fn as_u64(&self, name: reqwest::header::HeaderName) -> Result<Option<u64>>;
}

impl HttpHeader for reqwest::header::HeaderMap {
    fn as_system_time(&self, name: reqwest::header::HeaderName) -> Result<Option<SystemTime>> {
	match self.get(name) {
	    Some(tm)	=> Ok(Some(httpdate::parse_http_date(tm.to_str()
							     .map_err(|_| Error::BadHttpTime)?)?)),
	    None	=> Ok(None),
	}
    }

    fn as_time(&self, reftm: Time, name: reqwest::header::HeaderName) -> Result<Option<Time>> {
	let tm = self.as_system_time(name)?;

	Ok(match tm {
	    Some(t)	=> Some(reftm.relative(t).ok_or(Error::BadHttpTime)?),
	    None	=> None,
	})
    }

    fn as_u64(&self, name: reqwest::header::HeaderName) -> Result<Option<u64>> {
	match self.get(name) {
	    Some(v)	=> Ok(Some(super::as_u64(v.as_bytes())?)),
	    None	=> Ok(None),
	}
    }
}
