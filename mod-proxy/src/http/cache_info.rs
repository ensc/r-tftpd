use std::time::{Duration, Instant, SystemTime};

use crate::{ Result, Error };

use super::{ HttpHeader, Time };

#[derive(Debug)]
pub struct CacheInfo {
    pub not_after:	Option<Instant>,
    // given in remote time
    pub modified:	Option<SystemTime>,
    pub etag:		Option<reqwest::header::HeaderValue>,

    // the time when resoure was loaded
    pub local_time:	Instant,
}

impl CacheInfo {
    pub fn new(localtm: Time, hdrs: &reqwest::header::HeaderMap) -> Result<Self>
    {
	use reqwest::header as H;
	use super::CacheControlIterator as I;
	use super::CacheControl as C;

	let mut max_age = None;
	let mut max_sage = None;

	for cache in I::new(hdrs.get_all(H::CACHE_CONTROL)) {
	    match cache {
		Ok(C::MaxAge(d))	=> max_age  = Some(d),
		Ok(C::SMaxAge(d))	=> max_sage = Some(d),
		Ok(C::NoCache)		=> max_sage = Some(Duration::ZERO),
		Err(_)			=> warn!("bad cache-control from server"),
		_			=> { },
	    }
	}

	let remote_tm = hdrs.as_system_time(H::DATE)?.unwrap_or(localtm.local);

	let not_after = match max_sage.or(max_age) {
	    Some(d)	=> Some(localtm.mono.checked_add(d).ok_or(Error::BadHttpTime)?),
	    None	=> hdrs.as_instant(localtm.mono, remote_tm, H::EXPIRES)?,
	};

	Ok(Self {
	    not_after:		not_after,
	    modified:		hdrs.as_system_time(H::LAST_MODIFIED)?,
	    etag:		hdrs.get(H::ETAG).cloned(),
	    local_time:		localtm.mono,
	})
    }

    pub fn fill_request(&self, now: Instant, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
	use reqwest::header as H;

	if let Some(tm) = self.modified {
	    req = req.header(H::IF_MODIFIED_SINCE, httpdate::fmt_http_date(tm));
	}

	if let Some(e) = &self.etag {
	    req = req.header(H::IF_NONE_MATCH, e);
	}

	if let Some(tm) = self.not_after {
	    let delta = match tm < now {
		true	=> 0,
		false	=> tm.checked_duration_since(now).unwrap().as_secs(),
	    };

	    req = req.header(H::CACHE_CONTROL, format!("max-age={delta}"));
	}

	req
    }

    pub fn update(self, localtm: Time, hdrs: &reqwest::header::HeaderMap) -> Result<Self>
    {
	let tmp = Self::new(localtm, hdrs)?;

	Ok(Self {
	    not_after:	tmp.not_after.or(self.not_after),
	    modified:	tmp.modified.or(self.modified),
	    etag:	tmp.etag.or(self.etag),
	    ..tmp
	})
    }

    pub fn is_outdated(&self, reftm: Instant, max_lt: Duration) -> bool {
	self.not_after.map(|t| t < reftm).unwrap_or(false) ||
	    reftm.checked_duration_since(self.local_time).map(|d| d > max_lt).unwrap_or(false)
    }
}
