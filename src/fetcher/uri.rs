use crate::{ Result, Error };

use super::{ Cache, CacheEntry, CacheEntryData };

#[derive(Debug)]
pub struct Uri {
    uri:	url::Url,
    size:	Option<u64>,
    cache:	Option<CacheEntry>,
    pos:	u64,
    is_eof:	bool,
}

impl Uri {
    pub(crate) fn new(uri: &url::Url) -> Self {
	Self {
	    uri:	uri.clone(),
	    size:	None,
	    cache:	None,
	    pos:	0,
	    is_eof:	false,
	}
    }

    async fn open_cached(&self, entry: &mut CacheEntryData) -> Result<()>
    {
	use reqwest::StatusCode as S;

	if entry.is_running() {
	    return Ok(());
	}

	let client = Cache::get_client();

	let req = entry
	    .fill_request(client.get(entry.key.clone()))
	    .build()?;

	entry.update_localtm();

	let resp = client.execute(req).await?;

	match resp.status() {
	    S::NOT_MODIFIED	=> {
		entry.set_response(resp);
		entry.fill_meta().await?;
	    },
	    S::OK		=> {
		entry.invalidate();
		entry.set_response(resp);
		entry.fill_meta().await?;
	    },
	    s			=> {
		return Err(Error::HttpStatus(s));
	    }
	}

	Ok(())
    }

    fn get_uri(&self) -> (std::borrow::Cow<'_, url::Url>, bool) {
	use std::borrow::Cow;

	let mut res = Cow::Borrowed(&self.uri);

	let (scheme,xtra) = {
	    let mut i = self.uri.scheme().splitn(2, '+');

	    (i.next().unwrap(), i.next())
	};

	error!("scheme={}, xtra={:?}", scheme, xtra);

	match xtra {
	    Some("nocache")	=> {
		match res.to_mut().set_scheme(scheme) {
		    Ok(_)	=> { },
		    Err(_)	=> {
			// 'url' crate does not allow rewriting non-standard
			// 'http+nocached' to standard 'http' scheme
			let uri = scheme.to_string() + &res.as_str()[self.uri.scheme().len()..];
			let uri = url::Url::parse(&uri).unwrap();

			res = Cow::Owned(uri);
		    }
		};

		error!("{:?}", res);
		(res, true)
	    },

	    Some(xtra)		=> {
		warn!("unsupported xtra scheme param {}", xtra);
		(res, false)
	    },

	    None		=> (res, false)
	}
    }

    pub async fn open(&mut self) -> Result<()>
    {
	let entry = match self.get_uri() {
	    (uri, false)	=> Cache::lookup_or_create(&uri),
	    (uri, true)		=> Cache::create(&uri),
	};

	{
	    let mut e_locked = entry.write().await;

	    self.open_cached(&mut e_locked).await?;
	    self.size = Some(e_locked.get_filesize().await?);

	    if e_locked.is_error() {
		Cache::remove(&e_locked.key);
	    }
	};

	{
	    let e_locked = entry.read().await;

	    Cache::replace(&e_locked.key, &entry)
	};

	self.cache = Some(entry);

	Ok(())
    }

    pub async fn get_size(&self) -> Option<u64>
    {
	self.size
    }

    //#[instrument(level = "trace", skip_all, ret)]
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize>
    {
	assert!(!self.is_eof);

	let mut entry = self.cache.as_ref().unwrap().write().await;

	let len = buf.len();
	let mut pos = 0;

	while pos < len {
	    let sz = entry.read_some(self.pos, &mut buf[pos..len]).await?;

	    if sz == 0 {
		self.is_eof = true;
		break;
	    }

	    pos += sz;
	    self.pos += sz as u64;
	}

	Ok(pos)
    }

    pub fn is_eof(&self) -> bool
    {
	self.is_eof
    }
}
