use crate::{ Result, Error };
use std::time::{SystemTime, Instant};
use super::time::TimeDelta;

pub trait HttpHeader {
    fn as_system_time(&self, name: reqwest::header::HeaderName) -> Result<Option<SystemTime>>;
    #[allow(dead_code)]
    fn as_u64(&self, name: reqwest::header::HeaderName) -> Result<Option<u64>>;

    fn as_time_delta(&self, reftm: SystemTime, name: reqwest::header::HeaderName) -> Result<Option<TimeDelta>> {
	let tm = self.as_system_time(name)?;

	Ok(match tm {
	    Some(t)	=> Some(TimeDelta::from_systemtime(reftm, t).ok_or(Error::BadHttpTime)?),
	    None	=> None,
	})
    }

    fn as_instant(&self, now: Instant, reftm: SystemTime, name: reqwest::header::HeaderName) -> Result<Option<Instant>> {
	let delta = self.as_time_delta(reftm, name)?;

	Ok(match delta {
	    Some(d)	=> Some((now + d).ok_or(Error::BadHttpTime)?),
	    None	=> None,
	})
    }
}

impl HttpHeader for reqwest::header::HeaderMap {
    fn as_system_time(&self, name: reqwest::header::HeaderName) -> Result<Option<SystemTime>> {
	match self.get(name) {
	    Some(tm)	=> Ok(Some(httpdate::parse_http_date(tm.to_str()
							     .map_err(|_| Error::BadHttpTime)?)?)),
	    None	=> Ok(None),
	}
    }

    fn as_u64(&self, name: reqwest::header::HeaderName) -> Result<Option<u64>> {
	match self.get(name) {
	    Some(v)	=> Ok(Some(super::as_u64(v.as_bytes())?)),
	    None	=> Ok(None),
	}
    }
}
