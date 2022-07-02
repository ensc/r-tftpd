use crate::{ Result };

#[derive(Debug)]
pub struct Memory {
    buf:	Vec<u8>,
    pos:	usize,
}

impl Memory {
    pub fn new(data: &[u8]) -> Self {
	Self {
	    buf:	Vec::from(data),
	    pos:	0,
	}
    }

    pub fn open(&mut self) -> Result<()> {
	Ok(())
    }

    pub fn get_size(&self) -> Option<u64> {
	Some(self.buf.len() as u64)
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize>
    {
	let sz = buf.len().min(self.buf.len() - self.pos);

	buf[0..sz].clone_from_slice(&self.buf[self.pos..self.pos + sz]);

	self.pos += sz;

	Ok(sz)
    }

    pub fn read_mmap(&mut self, sz: usize) -> crate::Result<&[u8]>
    {
	let sz = sz.min(self.buf.len() - self.pos);
	let pos = self.pos;

	self.pos += sz;

	trace!("pos={}, sz={}", pos, sz);

	Ok(&self.buf[pos..pos + sz])
    }

    pub fn is_eof(&self) -> bool
    {
	self.pos == self.buf.len()
    }
}
