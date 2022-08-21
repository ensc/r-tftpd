use std::os::unix::prelude::{AsRawFd};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
// use chrono::{ NaiveDateTime, Utc };
use std::time::{ Duration };

use super::http;
use http::Time;

use crate::{ Result, Error };

lazy_static::lazy_static!{
    static ref CACHE: std::sync::RwLock<CacheImpl> = std::sync::RwLock::new(CacheImpl::new());
}

#[derive(Clone, Copy, Debug, Default)]
struct Stats {
    pub tm:		Duration,
}

impl Stats {
    pub async fn chunk(&mut self, response: &mut reqwest::Response) -> reqwest::Result<Option<bytes::Bytes>>
    {
	let start = std::time::Instant::now();
	let chunk = response.chunk().await;
	self.tm += start.elapsed();

	chunk
    }
}

#[derive(Debug)]
enum State {
    None,

    Error(&'static str),

    Init {
	response:	reqwest::Response,
    },

    HaveMeta {
	response:	reqwest::Response,
	cache_info:	http::CacheInfo,
	file_size:	Option<u64>,
	stats:		Stats,
    },

    Downloading {
	response:	reqwest::Response,
	cache_info:	http::CacheInfo,
	file_size:	Option<u64>,
	file:		std::fs::File,
	file_pos:	u64,
	stats:		Stats,
    },

    Complete {
	cache_info:	http::CacheInfo,
	file:		std::fs::File,
	file_size:	u64,
    },

    Refresh {
	response:	reqwest::Response,
	cache_info:	http::CacheInfo,
	file:		std::fs::File,
	file_size:	u64,
    },
}

impl State {
    pub fn take(&mut self, hint: &'static str) -> Self {
	std::mem::replace(self, State::Error(hint))
    }

    pub fn is_none(&self) -> bool {
	matches!(self, Self::None)
    }

    pub fn is_init(&self) -> bool {
	matches!(self, Self::Init { .. })
    }

    pub fn is_error(&self) -> bool {
	matches!(self, Self::Error(_))
    }

    pub fn is_refresh(&self) -> bool {
	matches!(self, Self::Refresh { .. })
    }

    pub fn is_have_meta(&self) -> bool {
	matches!(self, Self::HaveMeta { .. })
    }

    pub fn is_downloading(&self) -> bool {
	matches!(self, Self::Downloading { .. })
    }

    pub fn is_complete(&self) -> bool {
	matches!(self, Self::Complete { .. })
    }

    pub fn get_file_size(&self) -> Option<u64> {
	match self {
	    Self::None |
	    Self::Init { .. }	=> None,

	    Self::HaveMeta { file_size, .. }	=> *file_size,
	    Self::Downloading { file_size, .. }	=> *file_size,

	    Self::Complete { file_size, .. }	=> Some(*file_size),
	    Self::Refresh { file_size, .. }	=> Some(*file_size),

	    Self::Error(hint)	=> panic!("get_file_size called in error state ({})", hint),
	}
    }

    pub fn get_cache_info(&self) -> Option<&http::CacheInfo> {
	match self {
	    State::HaveMeta { cache_info, .. } |
	    State::Downloading { cache_info, .. } |
	    State::Complete { cache_info, .. }	=> Some(cache_info),
	    State::Refresh { cache_info, .. }	=> Some(cache_info),

	    Self::Error(hint)	=> panic!("get_file_size called in error state ({})", hint),
	    _			=> None,
	}
    }

    fn read_file(file: &std::fs::File, ofs: u64, buf: &mut [u8], max: u64) -> Result<usize> {
	use nix::libc;

	assert!(max > ofs);

	let len = (buf.len() as u64).min(max - ofs) as usize;
	let buf_ptr = buf.as_mut_ptr() as *mut libc::c_void;

	// TODO: this would be nice, but does not work because we can not get
	// a mutable reference to 'file'
	//file.flush()?;

	let rc = unsafe { libc::pread(file.as_raw_fd(), buf_ptr, len, ofs as i64) };

	if rc < 0 {
	    return Err(std::io::Error::last_os_error().into());
	}

	Ok(len)
    }

    pub fn read(&self, ofs: u64, buf: &mut [u8]) -> Result<Option<usize>> {
	match &self {
	    State::Downloading { file, file_pos, .. } if ofs < *file_pos	=> {
		Self::read_file(file, ofs, buf, *file_pos)
	    },

	    State::Complete { file, file_size, .. } if ofs < *file_size		=> {
		Self::read_file(file, ofs, buf, *file_size)
	    }

	    State::Complete { file_size, .. } if ofs == *file_size	=> Ok(0),

	    State::Complete { file_size, .. } if ofs >= *file_size	=>
		Err(Error::Internal("file out-of-bound read")),

	    _	=> return Ok(None)
	}.map(|sz| Some(sz as usize))
    }

    pub fn is_outdated(&self, reftm: Time) -> bool {
	match self {
	    Self::None |
	    Self::Error(_) |
	    Self::Init { .. }	=> true,

	    Self::HaveMeta { cache_info, .. } |
	    Self::Downloading { cache_info, .. } |
	    Self::Complete { cache_info, .. } |
	    Self::Refresh { cache_info, .. }	=> cache_info.is_outdated(reftm),
	}
    }
}

#[derive(Debug)]
pub struct EntryData {
    pub key:		url::Url,
    state:		State,
    reftm:		Time,
}

impl EntryData {
    pub fn new(url: &url::Url) -> Self {
	Self {
	    key:		url.clone(),
	    state:		State::None,
	    reftm:		Time::now(),
	}
    }

    pub fn is_complete(&self) -> bool {
	self.state.is_complete()
    }

    pub fn is_error(&self) -> bool {
	self.state.is_error()
    }

    pub fn is_running(&self) -> bool {
	self.state.is_have_meta() || self.state.is_downloading()
    }

    pub fn update_localtm(&mut self) {
	self.reftm = Time::now();
    }

    pub fn set_response(&mut self, response: reqwest::Response) {
	self.state = match self.state.take("set_respone") {
	    State::None |
	    State::Error(_)	=> State::Init { response },

	    State::Complete { cache_info, file, file_size } |
	    State::Refresh { cache_info, file, file_size, .. } => State::Refresh {
		cache_info:	cache_info,
		file:		file,
		file_size:	file_size,
		response:	response,
	    },

	    s			=> panic!("unexpected state {:?}", s),
	}
    }

    pub fn is_outdated(&self, reftm: Time) -> bool {
	self.state.is_outdated(reftm)
    }

    pub async fn fill_meta(&mut self) -> Result<()> {
	if !self.state.is_init() && !self.state.is_none() && !self.state.is_refresh() {
	    return Ok(());
	}

	self.state = match self.state.take("fill_meta") {
	    State::None			=> panic!("unexpected state"),

	    State::Init{ response }	=> {
		let hdrs = response.headers();

		State::HaveMeta {
		    cache_info:	http::CacheInfo::new(self.reftm, hdrs)?,
		    file_size:	response.content_length(),
		    response:	response,
		    stats:	Stats::default(),
		}
	    },

	    State::Refresh { file, file_size, response, cache_info }	=> {
		let hdrs = response.headers();

		State::Complete {
		    cache_info:	cache_info.update(self.reftm, hdrs)?,
		    file:	file,
		    file_size:	file_size,
		}
	    },

	    _				=> unreachable!(),
	};

	Ok(())
    }

    fn signal_complete(&self, stats: Stats) {
	if let State::Complete { file_size, .. } = self.state {
	    info!("downloaded {} with {} bytes in {}ms", self.key, file_size, stats.tm.as_millis());
	}
    }

    #[instrument(level = "trace")]
    pub async fn get_filesize(&mut self) -> Result<u64> {
	use std::io::Write;

	if let Some(sz) = self.state.get_file_size() {
	    return Ok(sz);
	}

	match self.state.take("get_filesize") {
	    State::HaveMeta { mut response, file_size: None, mut stats, cache_info }	=> {
		let mut file = Cache::new_file()?;
		let mut pos = 0;


		while let Some(chunk) = stats.chunk(&mut response).await? {
		    pos += chunk.len() as u64;
		    file.write_all(&chunk)?;
		}

		self.state = State::Complete {
		    file:	file,
		    file_size:	pos,
		    cache_info:	cache_info,
		};

		self.signal_complete(stats);

		Ok(pos)
	    },

	    State::Downloading { mut response, mut file, file_pos, file_size: None, mut stats, cache_info } => {
		let mut pos = file_pos;

		while let Some(chunk) = stats.chunk(&mut response).await? {
		    pos += chunk.len() as u64;
		    file.write_all(&chunk)?;
		}

		self.state = State::Complete {
		    file:	file,
		    file_size:	pos,
		    cache_info:	cache_info,
		};

		self.signal_complete(stats);

		Ok(pos)
	    }

	    s		=> panic!("unexpected state: {:?}", s),
	}
    }

    pub fn fill_request(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
	match self.state.get_cache_info() {
	    Some(info)	=> info.fill_request(self.reftm, req),
	    None	=> req,
	}
    }

    pub fn matches(&self, etag: Option<&str>) -> bool {
	let cache_info = self.state.get_cache_info();

	match cache_info.and_then(|c| c.not_after) {
	    Some(t) if t < Time::now()			=> return false,
	    _						=> {},
	}

	let self_etag = match cache_info {
	    Some(c)	=> c.etag.as_ref(),
	    None	=> None,
	};

	match (self_etag, etag) {
	    (Some(a), Some(b)) if a == b		=> {},
	    (None, None)				=> {},
	    _						=> return false,
	}

	true
    }

    pub fn invalidate(&mut self)
    {
	match &self.state {
	    State::Refresh { .. }	=> self.state = State::None,
	    State::Complete { .. }	=> self.state = State::None,
	    _				=> {},
	}
    }

    pub async fn read_some(&mut self, ofs: u64, buf: &mut [u8]) -> Result<usize>
    {
	use std::io::Write;

	trace!("state={:?}, ofs={}, #buf={}", self.state, ofs, buf.len());

	async fn fetch(response: &mut reqwest::Response, file: &mut std::fs::File,
		       buf: &mut [u8], stats: &mut Stats) -> Result<(usize, usize)> {
	    match stats.chunk(response).await? {
		Some(data)	=> {
		    let len = buf.len().min(data.len());

		    buf[0..len].clone_from_slice(&data.as_ref()[0..len]);
		    file.write_all(&data)?;

		    // TODO: it would be better to do this in State::read_file()
		    file.flush()?;

		    Ok((len, data.len()))
		},

		None		=> Ok((0, 0))
	    }
	}

	if self.state.is_init() {
	    self.fill_meta().await?;
	}

	if let Some(sz) = self.state.read(ofs, buf)? {
	    return Ok(sz);
	}

	match self.state.take("read_some") {
	    State::HaveMeta { mut response, cache_info, file_size, mut stats }	=> {
		let mut file = Cache::new_file()?;

		let res = fetch(&mut response, &mut file, buf, &mut stats).await?;

		self.state = match res {
		    (_, 0)	=> State::Complete {
			cache_info:	cache_info,
			file:		file,
			file_size:	0,
		    },

		    (_, sz)	=> State::Downloading {
			response:	response,
			cache_info:	cache_info,
			file_size:	file_size,
			file:		file,
			file_pos:	sz as u64,
			stats:		stats,
		    }
		};

		self.signal_complete(stats);

		Ok(res.0)
	    },

	    // catched by 'self.state.read()' above
	    State::Downloading { file_pos, .. } if ofs < file_pos	=> unreachable!(),

	    State::Downloading { mut response, cache_info, file_size, mut file, file_pos, mut stats } => {
		let res = fetch(&mut response, &mut file, buf, &mut stats).await?;

		self.state = match res {
		    (_, 0)	=> State::Complete {
			cache_info:	cache_info,
			file:		file,
			file_size:	file_pos,
		    },

		    (_, sz)	=> State::Downloading {
			response:	response,
			cache_info:	cache_info,
			file_size:	file_size,
			file:		file,
			file_pos:	file_pos + (sz as u64),
			stats:		stats,
		    }
		};

		self.signal_complete(stats);

		Ok(res.0)
	    }

	    s		=> panic!("unexpected state: {:?}", s),
	}

    }
}

pub type Entry = Arc<RwLock<EntryData>>;

struct CacheImpl {
    tmpdir:	std::path::PathBuf,
    entries:	HashMap<url::Url, Entry>,
    client:	Arc<reqwest::Client>,
    is_dirty:	bool,
}

pub enum LookupResult {
    Found(Entry),
    Missing,
}

impl CacheImpl {
    fn new() -> Self {
	Self {
	    tmpdir:	std::env::temp_dir(),
	    entries:	HashMap::new(),
	    client:	Arc::new(reqwest::Client::new()),
	    is_dirty:	false,
	}
    }

    pub fn get_client(&self) -> Arc<reqwest::Client> {
	self.client.clone()
    }

    pub fn lookup_or_create(&mut self, key: &url::Url) -> Entry {
	match self.entries.get(key) {
	    Some(v)	=> {
		self.is_dirty = true;
		v.clone()
	    },
	    None	=> self.create(key),
	}
    }

    pub fn create(&mut self, key: &url::Url) -> Entry {
	Entry::new(RwLock::new(EntryData::new(key)))
    }

    pub fn replace(&mut self, key: &url::Url, entry: &Entry) {
	self.is_dirty = true;
	self.entries.insert(key.clone(), entry.clone());
    }

    pub fn remove(&mut self, key: &url::Url) {
	self.is_dirty = true;
	self.entries.remove(key);
    }

    pub fn run_gc(&mut self) {
	if !self.is_dirty {
	    return;
	}

	self.is_dirty = false;

	let mut outdated = Vec::new();
	let now = Time::now();

	for (key, e) in &self.entries {
	    let entry = match e.try_read() {
		Ok(e)	=> e,
		_	=> continue,
	    };

	    if entry.is_outdated(now) {
		outdated.push(key.clone());
	    }
	}

	for e in outdated {
	    self.remove(&e);
	}
    }
}

pub struct Cache();

impl Cache {
    #[instrument(level = "trace")]
    pub fn instanciate(tmpdir: &std::path::Path) {
	let mut cache = CACHE.write().unwrap();

	trace!("tmpdir={:?}", tmpdir);

	cache.tmpdir = tmpdir.into();
    }

    #[instrument(level = "trace", ret)]
    pub fn lookup_or_create(key: &url::Url) -> Entry {
	let mut cache = CACHE.write().unwrap();

	cache.lookup_or_create(key)
    }

    #[instrument(level = "trace", ret)]
    pub fn create(key: &url::Url) -> Entry {
	let mut cache = CACHE.write().unwrap();

	cache.create(key)
    }

    #[instrument(level = "trace", ret)]
    pub fn replace(key: &url::Url, entry: &Entry) {
	let mut cache = CACHE.write().unwrap();

	cache.replace(key, entry)
    }

    #[instrument(level = "trace", ret)]
    pub fn remove(key: &url::Url) {
	let mut cache = CACHE.write().unwrap();

	cache.remove(key)
    }

    pub fn get_client() -> Arc<reqwest::Client> {
	let cache = CACHE.read().unwrap();

	cache.get_client()
    }

    pub fn new_file() -> Result<std::fs::File> {
	let cache = CACHE.read().unwrap();

	Ok(tempfile::tempfile_in(&cache.tmpdir)?)
    }
}
