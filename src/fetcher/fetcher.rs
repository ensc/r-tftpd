#[derive(Debug)]
pub enum Fetcher {
    File(Box<super::File>),
    Memory(Box<super::Memory>),
    #[cfg(feature = "proxy")]
    Uri(Box<super::Uri>),
}

impl Fetcher {
    #[instrument(level = "trace")]
    pub fn new_file(path: &std::path::Path) -> Self {
	Self::File(Box::new(super::file::File::new(path)))
    }

    #[cfg(feature = "proxy")]
    #[instrument(level = "trace")]
    pub fn new_uri(uri: &url::Url) -> Self {
	Self::Uri(Box::new(super::Uri::new(uri)))
    }

    #[cfg(test)]
    pub fn new_memory(buf: &[u8]) -> Self {
	Self::Memory(Box::new(super::memory::Memory::new(buf)))
    }

    pub fn is_mmaped(&self) -> bool {
	match self {
	    Self::File(f)	=> f.is_mmaped(),
	    Self::Memory(_)	=> true,
	    #[cfg(feature = "proxy")]
	    Self::Uri(_)	=> false,
	}
    }

    #[instrument(level = "trace")]
    pub async fn open(&mut self) -> crate::Result<()> {
	match self {
	    Self::File(f)	=> f.open(),
	    Self::Memory(m)	=> m.open(),
	    #[cfg(feature = "proxy")]
	    Self::Uri(u)	=> Ok(u.open().await?),
	}
    }

    #[instrument(level = "trace", ret)]
    pub async fn get_size(&self) -> Option<u64> {
	match self {
	    Self::File(f)	=> f.get_size(),
	    Self::Memory(m)	=> m.get_size(),
	    #[cfg(feature = "proxy")]
	    Self::Uri(u)	=> u.get_size().await,
	}
    }

    //#[instrument(level = "trace", skip(buf), ret)]
    pub async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize>
    {
	match self {
	    Self::File(f)	=> f.read(buf).await,
	    Self::Memory(m)	=> m.read(buf).await,
	    #[cfg(feature = "proxy")]
	    Self::Uri(u)	=> Ok(u.read(buf).await?),
	}
    }

    #[instrument(level = "trace")]
    pub fn read_mmap(&mut self, cnt: usize) -> crate::Result<&[u8]>
    {
	match self {
	    Self::File(f)	=> f.read_mmap(cnt),
	    Self::Memory(m)	=> m.read_mmap(cnt),
	    #[cfg(feature = "proxy")]
	    Self::Uri(_)	=> unimplemented!(),
	}
    }

    pub fn is_eof(&self) -> bool
    {
	match self {
	    Self::File(f)	=> f.is_eof(),
	    Self::Memory(m)	=> m.is_eof(),
	    #[cfg(feature = "proxy")]
	    Self::Uri(u)	=> u.is_eof(),
	}
    }
}
