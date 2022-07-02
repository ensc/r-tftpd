#[derive(Debug)]
pub enum Fetcher {
    File(Box<super::File>),
    Memory(Box<super::Memory>),
}

impl Fetcher {
    pub fn new_file(path: &std::path::Path) -> Self {
	Self::File(Box::new(super::file::File::new(path)))
    }

    #[cfg(test)]
    pub fn new_memory(buf: &[u8]) -> Self {
	Self::Memory(Box::new(super::memory::Memory::new(buf)))
    }

    pub fn is_mmaped(&self) -> bool {
	match self {
	    Self::File(f)	=> f.is_mmaped(),
	    Self::Memory(_)	=> true,
	}
    }

    pub fn open(&mut self) -> crate::Result<()> {
	match self {
	    Self::File(f)	=> f.open(),
	    Self::Memory(m)	=> m.open(),
	}
    }

    pub fn get_size(&self) -> Option<u64> {
	match self {
	    Self::File(f)	=> f.get_size(),
	    Self::Memory(m)	=> m.get_size(),
	}
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize>
    {
	match self {
	    Self::File(f)	=> f.read(buf).await,
	    Self::Memory(m)	=> m.read(buf).await,
	}
    }

    pub fn read_mmap(&mut self, cnt: usize) -> crate::Result<&[u8]>
    {
	match self {
	    Self::File(f)	=> f.read_mmap(cnt),
	    Self::Memory(m)	=> m.read_mmap(cnt),
	}
    }

    pub fn is_eof(&self) -> bool
    {
	match self {
	    Self::File(f)	=> f.is_eof(),
	    Self::Memory(m)	=> m.is_eof(),
	}
    }
}
