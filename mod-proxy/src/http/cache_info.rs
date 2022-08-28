use std::time::Duration;

use super::{Time, HttpHeader};

use crate::{ Result, Error };

#[derive(Debug)]
pub struct CacheInfo {
    pub not_after:	Option<Time>,
    pub modified:	Option<Time>,
    pub etag:		Option<reqwest::header::HeaderValue>,
    pub reftm:		Time,
    pub localtm:	Time,
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

	let reftm = hdrs.as_time(localtm, H::DATE)?.unwrap_or(localtm);

	let not_after = match max_sage.or(max_age) {
	    Some(d)	=> Some(reftm.checked_add(d).ok_or(Error::BadHttpTime)?),
	    None	=> hdrs.as_time(localtm, H::EXPIRES)?,
	};

	Ok(Self {
	    not_after:	not_after,
	    modified:	hdrs.as_time(localtm, H::LAST_MODIFIED)?,
	    etag:	hdrs.get(H::ETAG).cloned(),
	    reftm:	reftm,
	    localtm:	localtm,
	})
    }

    pub fn fill_request(&self, localtm: Time, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
	use reqwest::header as H;

	if let Some(tm) = self.modified {
	    req = req.header(H::IF_MODIFIED_SINCE, httpdate::fmt_http_date(tm.local));
	}

	if let Some(e) = &self.etag {
	    req = req.header(H::IF_NONE_MATCH, e);
	}

	match self.not_after {
	    // TODO: this compares server time (tm) with localtime (localtm)
	    Some(tm) if tm < localtm	=> req = req.header(H::CACHE_CONTROL, "max-age=0"),
	    Some(tm)			=>
		req = req.header(H::CACHE_CONTROL,
				 format!("max-age={}", tm.checked_duration_since(localtm).unwrap().as_secs())),
	    None			=> {},
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

    pub fn is_outdated(&self, reftm: Time, max_lt: Duration) -> bool {
	self.not_after.map(|t| t < reftm).unwrap_or(false) ||
	    reftm.checked_duration_since(self.localtm).map(|d| d > max_lt).unwrap_or(false)
    }
}
