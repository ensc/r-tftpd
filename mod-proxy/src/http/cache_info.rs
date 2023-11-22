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

impl crate::util::PrettyDump for CacheInfo {
    fn pretty_dump(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	use crate::util::pretty_dump_wrap as pretty;

        f.write_fmt(format_args!("[not_after={}, modified={}, etag={}, tm={}]",
				 pretty(&self.not_after),
				 pretty(&self.modified),
				 pretty(&self.etag),
				 pretty(&self.local_time)))
    }
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

    pub fn get_expiration_tm(&self, max_lt: Duration) -> Instant {
	match (self.not_after, self.local_time + max_lt) {
	    (None, tm)		=> tm,
	    (Some(a), b)	=> a.min(b),
	}
    }

    pub fn is_outdated(&self, reftm: Instant, max_lt: Duration) -> bool {
	self.get_expiration_tm(max_lt) <= reftm
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_outdated() {
	use reqwest::header as H;
	use reqwest::header::HeaderMap;

	// Sun May 16 01:00:00 CET 1971
	let now = Time {
	    local:	SystemTime::UNIX_EPOCH + Duration::from_secs(500 * 24 * 3600),
	    mono:	Instant::now(),
	};

	let tm_20 = now.mono + Duration::from_secs(20);
	let tm_50 = now.mono + Duration::from_secs(50);
	let tm_1d = now.mono + Duration::from_secs(24 * 3600);

	let e = {
	    let mut map = HeaderMap::new();

	    map.append(H::DATE, "Mon, 24 May 1971 00:00:00 GMT".parse().unwrap());

	    CacheInfo::new(now, &map).unwrap()
	};

	//println!("e={e:?}");

	assert_eq!(e.get_expiration_tm(Duration::from_secs(10)),
		   e.local_time + Duration::from_secs(10));
	assert!(!e.is_outdated(now.mono,  Duration::from_secs(10)));
	assert!( e.is_outdated(tm_20,     Duration::from_secs(10)));

	//

	let e = {
	    let mut map = HeaderMap::new();

	    map.append(H::CACHE_CONTROL, "max-age=23".parse().unwrap());
	    map.append(H::DATE, "Mon, 24 May 1971 00:00:00 GMT".parse().unwrap());

	    CacheInfo::new(now, &map).unwrap()
	};

	//println!("e={e:?}");

	assert_eq!(e.get_expiration_tm(Duration::from_secs(100)),
		   now.mono + Duration::from_secs(23));
	assert!(!e.is_outdated(now.mono, Duration::from_secs(100)));
	assert!(!e.is_outdated(tm_20,    Duration::from_secs(100)));
	assert!( e.is_outdated(tm_50,    Duration::from_secs(100)));

	//

	let e = {
	    let mut map = HeaderMap::new();

	    map.append(H::EXPIRES, "Mon, 24 May 1971 12:00:00 GMT".parse().unwrap());
	    map.append(H::DATE, "Mon, 24 May 1971 00:00:00 GMT".parse().unwrap());

	    CacheInfo::new(now, &map).unwrap()
	};

	//println!("e={e:?}");

	assert_eq!(e.get_expiration_tm(Duration::from_secs(100_000)),
		   e.local_time + Duration::from_secs(12 * 3600));
	assert_eq!(e.get_expiration_tm(Duration::from_secs(100)),
		   e.local_time + Duration::from_secs(100));

	assert!(!e.is_outdated(now.mono, Duration::from_secs(100_000)));
	assert!(!e.is_outdated(tm_20,    Duration::from_secs(100_000)));
	assert!( e.is_outdated(tm_1d,    Duration::from_secs(100_000)));

	let e = {
	    let mut map = HeaderMap::new();

	    map.append(H::LAST_MODIFIED, "Sun, 23 May 1971 00:00:00 GMT".parse().unwrap());
	    map.append(H::DATE, "Mon, 24 May 1971 00:00:00 GMT".parse().unwrap());

	    CacheInfo::new(now, &map).unwrap()
	};

	//println!("e={e:?}");

	assert_eq!(e.get_expiration_tm(Duration::from_secs(100_000)),
		   e.local_time + Duration::from_secs(100_000));

	assert!(!e.is_outdated(now.mono, Duration::from_secs(100_000)));
    }
}
