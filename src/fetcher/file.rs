use crate::{ Error, Result };
use crate::util::AsInit;

use std::io::Read;
use std::mem::MaybeUninit;

#[derive(Debug)]
pub struct File {
    path:	std::path::PathBuf,
    file:	Option<std::fs::File>,
    is_eof:	bool,
}

impl File {
    pub fn new(path: &std::path::Path) -> Self {
	Self {
	    path:	path.into(),
	    file:	None,
	    is_eof:	false,
	}
    }

    pub fn open(&mut self) -> Result<()> {
	if self.file.is_some() {
	    return Err(Error::Internal("file already opened"));
	}

	self.file = match std::fs::File::open(&self.path) {
	    Err(e) if e.kind() == std::io::ErrorKind::NotFound	=>
                return Err(Error::FileMissing(self.path.clone().into())),
	    Err(e)	=> return Err(Error::Io(e)),
	    Ok(f)	=> Some(f),
	};

	Ok(())
    }

    pub fn is_mmaped(&self) -> bool {
	false
    }

    pub fn get_size(&self) -> Option<u64> {
	let file = self.file.as_ref().unwrap();

	file.metadata().ok().map(|v| v.len())
    }

    pub async fn read<'a>(&mut self, buf: &'a mut [MaybeUninit<u8>]) -> crate::Result<&'a [u8]>
    {
	assert!(!self.is_eof());

	let mut file = self.file.as_ref().unwrap();
	let mut len = buf.len();
	let mut pos = 0;

        let buf = unsafe { buf.assume_init() };

	while len > 0 {
	    let sz = file.read(&mut buf[pos..])?;

	    if sz == 0 {
		trace!("eof reached");
		self.is_eof = true;
		break;
	    }

	    len -= sz;
	    pos += sz;
	}

	Ok(&buf[..pos])
    }

    pub fn read_mmap(&mut self, _cnt: usize) -> crate::Result<&[u8]>
    {
	Err(Error::Internal("File::read_mmap() not implemented"))
    }

    pub fn is_eof(&self) -> bool
    {
	self.is_eof
    }
}
