use crate::{ Result, Error };
use crate::fetcher::Fetcher;
use crate::tftp::SequenceId;

use super::Datagram;

enum Data<'a> {
    Owned(Vec<u8>),
    Ref(Option<&'a[u8]>),
}

impl Data<'_>
{
    pub fn alloc(sz: usize) -> Self {
	let mut data = Vec::with_capacity(sz);

	#[allow(clippy::uninit_vec)]
	unsafe { data.set_len(sz); }

	Self::Owned(data)
    }
}

struct Block<'a> {
    data:	Data<'a>,
    len:	u16,
    blksz:	u16,
}

impl <'a> Block<'a> {
    pub fn new_owned(size: u16) -> Self {
	Self {
	    data:	Data::alloc(size as usize),
	    blksz:	size,
	    len:	0,
	}
    }

    pub fn new_ref(size: u16) -> Self {
	Self {
	    data:	Data::Ref(None),
	    blksz:	size,
	    len:	0,
	}
    }

    pub fn get_blksize(&self) -> u16 {
	self.blksz
    }

    pub fn init(&mut self) -> &mut Self {
	self.len = 0;
	self
    }

    pub fn set_len(&mut self, sz: usize) {
	assert!(sz <= self.blksz as usize);
	self.len = u16::try_from(sz).unwrap();
    }

    pub fn get_data(&self) -> &[u8] {
	match &self.data {
	    Data::Owned(d)	=> &d[0..self.len as usize],
	    Data::Ref(Some(d))	=> &d[0..self.len as usize],
	    Data::Ref(None)	=> panic!("Data::Ref is None"),
	}
    }

    pub async fn fill<'b>(&mut self, fetcher: &'b mut Fetcher) -> Result<usize>
//    where
//	'b: 'a
    {
	let sz = match &mut self.data {
	    Data::Owned(d)	=> fetcher.read(d).await?,
	    Data::Ref(_)	=> {
		let data = fetcher.read_mmap(self.get_blksize() as usize)?;

		self.data = unsafe { std::mem::transmute(Data::Ref(Some(data))) };
		data.len()
	    }
	};

	self.set_len(sz);

	Ok(sz)
    }
}

#[derive(Default)]
struct BlockInfo {
    seq:	SequenceId,
    idx:	u16,
}

pub struct Xfer<'a> {
    start:	BlockInfo,
    active_sz:	u16,
    blocks:	Vec<Block<'a>>,
    is_eof:	bool,
}

impl <'a> Xfer<'a> {
    pub fn new<'b>(fetcher: &'b Fetcher, blk_size: u16, window_size: u16) -> Self
    where
	'a: 'b
    {
	assert!(window_size > 0);
	assert!(window_size < u16::MAX);

	let window_size = window_size as usize;

	let mut blocks = Vec::with_capacity(window_size);

	for _ in 0..window_size {
	    match fetcher.is_mmaped() {
		true	=> blocks.push(Block::new_ref(blk_size)),
		false	=> blocks.push(Block::new_owned(blk_size)),
	    }
	}

	Self {
	    start:	BlockInfo::default(),
	    active_sz:	0,
	    blocks:	blocks,
	    is_eof:	false,
	}
    }

    fn window_size(&self) -> u16
    {
	self.blocks.len() as u16
    }

    fn get_rel_block(&self, idx: u16) -> Option<(SequenceId, &Block)>
    {
	if idx >= self.active_sz {
	    return None;
	}

	let mut p = self.start.idx + idx;

	if p >= self.window_size() {
	    p -= self.window_size();
	}

	Some((self.start.seq + idx, &self.blocks[p as usize]))
    }

    fn alloc_block(&mut self) -> Option<&mut Block<'a>>
    {
	if self.active_sz >= self.window_size() {
	    return None;
	}

	let mut p = self.start.idx + self.active_sz;

	if p >= self.window_size() {
	    p -= self.window_size();
	}

	self.active_sz += 1;

	let block = &mut self.blocks[p as usize];

	block.init();

	Some(block)
    }

    fn free_blocks(&mut self, blk_id: SequenceId) -> Result<()>
    {
	let delta = if self.active_sz == 0 {
	    0_u16
	} else {
	    blk_id.delta(self.start.seq)
	};

	#[allow(clippy::comparison_chain)]
	if delta == self.active_sz {
	    trace!("all active blocks consumed");
	    self.start.idx = 0;
	    self.start.seq = blk_id;
	    self.active_sz = 0;
	} else if delta > self.active_sz {
	    return Err(Error::Protocol("blk-id out of window"));
	} else {
	    trace!("freeing {} blocks", delta);
	    self.start.idx = (self.start.idx + delta) % self.window_size();
	    self.start.seq += delta;
	    self.active_sz -= delta;
	}

	Ok(())
    }

    pub async fn fill_window<'b>(&mut self, blk_id: SequenceId, fetcher: &'b mut Fetcher) -> Result<()>
//    where
//	'b: 'a
    {
	assert!(self.active_sz <= self.window_size());

	trace!("filling {:?} in {:?}@{}+{}", blk_id, self.start.seq, self.start.idx, self.active_sz);

	self.free_blocks(blk_id)?;

	if self.active_sz > 0 {
	    debug!("retransmitting {:?}+", blk_id);
	}

	while self.active_sz < self.window_size() && !self.is_eof {
	    let block = self.alloc_block().unwrap();

	    let sz = if fetcher.is_eof() {
		block.set_len(0);
		0
	    } else {
		block.fill(fetcher).await?
	    };

	    if sz < block.get_blksize() as usize {
		self.is_eof = true;
	    }

	    debug!("read {}; active_sz={}", sz, self.active_sz);
	}

	Ok(())
    }

    pub fn is_eof(&self) -> bool
    {
	self.is_eof && self.active_sz == 0
    }

    pub fn iter(&'a self) -> XferIterator<'a>
    {
	XferIterator {
	    xfer: self,
	    pos: 0,
	}
    }
}

pub struct XferIterator<'a>
{
    xfer: &'a Xfer<'a>,
    pos: u16,
}

impl <'a> Iterator for XferIterator<'a> {
    type Item = Datagram<'a>;

    fn next(&mut self) -> Option<Self::Item>
    {
	let (seq, block) = self.xfer.get_rel_block(self.pos)?;

	self.pos += 1;

	Some(Datagram::Data(seq, block.get_data()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn verify_data(xfer: &Xfer, start_idx: SequenceId, cnt: u16)
    {
	let end_idx = start_idx + cnt;

	for (idx, d) in xfer.iter().enumerate() {
	    println!("idx={}, d={:?}", idx, d);
	    match d {
		Datagram::Data(id, data)	=> {
		    assert_eq!(id, start_idx + idx as u16);

		    match id.as_u16() {
			23	=> assert_eq!(data, &[ 0,  1]),
			24	=> assert_eq!(data, &[ 2,  3]),
			25	=> assert_eq!(data, &[ 4,  5]),
			26	=> assert_eq!(data, &[ 6,  7]),
			27	=> assert_eq!(data, &[ 8,  9]),
			28	=> assert_eq!(data, &[10, 11]),
			29	=> assert_eq!(data, &[12, 13]),
			30	=> assert_eq!(data, &[14, 15]),
			31	=> assert_eq!(data, &[]),

			50	=> assert_eq!(data, &[ 0,  1]),
			51	=> assert_eq!(data, &[ 2 ]),
			_	=> unreachable!(),
		    }
		},

		_	=> unreachable!(),
	    }
	}

	assert_eq!(xfer.iter().count(), cnt as usize);
    }

    #[tokio::test]
    async fn test_0() {
	tracing_subscriber::fmt::init();

	let mut f = Fetcher::new_memory(&[0, 1,   2,  3,   4,  5,   6,  7,
					  8, 9,  10, 11,  12, 13,  14, 15]);

	let mut xfer = Xfer::new(&f, 2, 3);

	assert!(!xfer.is_eof());

	let mut seq = SequenceId::new(23);

	info!("23/0 + 3");
	xfer.fill_window(seq, &mut f).await.expect("fill_window(0) failed");
	verify_data(&xfer, seq, 3);
	assert!(!xfer.is_eof());

	info!("25/2 + 3; last buffer of previous transfer was lost");
	seq += 2;		// 25
	xfer.fill_window(seq, &mut f).await.expect("fill_window(+2) failed");
	verify_data(&xfer, seq, 3);
	assert!(!xfer.is_eof());

	info!("24; error");
	seq -= 1;		// 24
	xfer.fill_window(seq, &mut f).await.expect_err("out-of-window blkid succeeded");
	seq += 1;		// 25
	verify_data(&xfer, seq, 3);
	assert!(!xfer.is_eof());

	info!("29; error");
	seq += 4;		// 29
	xfer.fill_window(seq, &mut f).await.expect_err("out-of-window blkid succeeded");
	seq -= 4;		// 25
	verify_data(&xfer, seq, 3);
	assert!(!xfer.is_eof());

	seq += 3;		// 28
	xfer.fill_window(seq, &mut f).await.expect("fill_window(+3) failed");
	verify_data(&xfer, seq, 3);
	assert!(!xfer.is_eof());

	seq += 2;		// 30
	xfer.fill_window(seq, &mut f).await.expect("fill_window(+3) failed");
	verify_data(&xfer, seq, 2);
	assert!(!xfer.is_eof());

	seq += 2;		// 32
	xfer.fill_window(seq, &mut f).await.expect("fill_window(+3) failed");
	verify_data(&xfer, seq, 0);
	assert!(xfer.is_eof());
    }

    #[tokio::test]
    async fn test_1() {
	let mut f = Fetcher::new_memory(&[0, 1, 2]);

	let mut xfer = Xfer::new(&f, 2, 3);

	assert!(!xfer.is_eof());

	let mut seq = SequenceId::new(50);
	xfer.fill_window(seq, &mut f).await.expect("fill_window(0) failed");
	verify_data(&xfer, seq, 2);
	assert!(!xfer.is_eof());

	seq += 1;		// 51
	xfer.fill_window(seq, &mut f).await.expect("fill_window(0) failed");
	verify_data(&xfer, seq, 1);
	assert!(!xfer.is_eof());

	seq += 1;		// 52
	xfer.fill_window(seq, &mut f).await.expect("fill_window(0) failed");
	verify_data(&xfer, seq, 0);
	assert!(xfer.is_eof());
    }
}
