use std::mem::MaybeUninit;

use crate::{ Error, Result };
use crate::util::AsInit;

use super::{ Cache, CacheEntry, CacheEntryData };

#[derive(Debug)]
pub struct Uri {
    uri:	url::Url,
    size:	Option<u64>,
    cache:	Option<CacheEntry>,
    pos:	u64,
    is_eof:	bool,
}

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    struct Flags: u8 {
	const NO_CACHE = 1;
	const NO_COMPRESS = 2;
    }
}

impl Uri {
    pub fn new(uri: &url::Url) -> Self {
	Self {
	    uri:	uri.clone(),
	    size:	None,
	    cache:	None,
	    pos:	0,
	    is_eof:	false,
	}
    }

    async fn open_cached(&self, entry: &mut CacheEntryData, flags: Flags) -> Result<()>
    {
	use reqwest::header as H;
	use reqwest::StatusCode as S;

	if entry.is_running() {
	    return Ok(());
	}

	let client = Cache::get_client();

	let mut req = entry
	    .fill_request(client.get(entry.key.clone()));

	if flags.contains(Flags::NO_COMPRESS) {
	    req = req.header(H::ACCEPT_ENCODING, "identity");
	}

	let req = req.build()?;

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

    fn get_uri(&self) -> (std::borrow::Cow<'_, url::Url>, Flags) {
	use std::borrow::Cow;

	let mut res = Cow::Borrowed(&self.uri);
	let mut flags = Flags::empty();

	let (scheme, has_xtra) = {
	    let mut i = self.uri.scheme().split('+');

	    let scheme = i.next().unwrap();
	    let mut has_xtra = false;

	    for x in i {
		has_xtra = true;

		match x {
		    "nocache"		=> flags |= Flags::NO_CACHE,
		    "nocompress"	=> flags |= Flags::NO_COMPRESS,
		    s			=> warn!("unsupported scheme modifier {}", s),
		}
	    }

	    (scheme, has_xtra)
	};

	if has_xtra {
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
	}

	(res, flags)
    }

    pub async fn open(&mut self) -> Result<()>
    {
	let (uri, flags) = self.get_uri();
	let entry = match flags {
	    f if f.contains(Flags::NO_CACHE)	=> Cache::create(&uri),
	    _					=> Cache::lookup_or_create(&uri),
	};

	{
	    let mut e_locked = entry.write().await;

	    self.open_cached(&mut e_locked, flags).await?;
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
    pub async fn read<'a>(&mut self, buf: &'a mut [MaybeUninit<u8>]) -> Result<&'a [u8]>
    {
	assert!(!self.is_eof);

	let mut entry = self.cache.as_ref().unwrap().write().await;

	let len = buf.len();
	let mut pos = 0;

	while pos < len {
	    let sz = entry.read_some(self.pos, &mut buf[pos..len]).await?.len();

	    if sz == 0 {
		self.is_eof = true;
		break;
	    }

	    pos += sz;
	    self.pos += sz as u64;
	}

	Ok(unsafe { buf[..pos].assume_init() })
    }

    pub fn is_eof(&self) -> bool
    {
	self.is_eof
    }
}
