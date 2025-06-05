use std::mem::MaybeUninit;

use crate::Result;
use crate::util::CopyInit;

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

    pub async fn read<'a>(&mut self, buf: &'a mut [MaybeUninit<u8>]) -> crate::Result<&'a [u8]>
    {
	let sz = buf.len().min(self.buf.len() - self.pos);

	let buf = buf[0..sz].write_copy_of_slice_x(&self.buf[self.pos..self.pos + sz]);

	self.pos += sz;

	Ok(buf)
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
