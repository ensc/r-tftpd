use std::time::Duration;

use super::Request;

pub struct Oack {
    pub block_size:	Option<u16>,
    pub timeout:	Option<Duration>,
    pub window_size:	Option<u16>,
    pub tsize:		Option<u64>,
}

fn append_option<V: Into<u64>>(msg: &mut Vec<u8>, id: &[u8], value: V)
{
    msg.extend(id);
    msg.push(0);
    msg.extend(value.into().to_string().as_bytes());
    msg.push(0);
}

impl Oack {
    pub fn from_request(req: &Request<'_>) -> Self {
	Self {
	    block_size:		req.block_size,
	    timeout:		req.timeout,
	    window_size:	req.window_size,
	    tsize:		req.tsize,
	}
    }

    pub fn update_block_size<F>(&mut self, max_val: u16, update_fn: F)
    where
	F: FnOnce(u16)
    {
	if let Some(sz) = self.block_size {
	    let v = sz.min(max_val);
	    self.block_size = Some(v);
	    update_fn(v)
	}
    }

    pub fn update_window_size<F>(&mut self, max_val: u16, update_fn: F)
    where
	F: FnOnce(u16)
    {
	if let Some(sz) = self.window_size {
	    let v = sz.min(max_val);
	    self.window_size = Some(v);
	    update_fn(v);
	}
    }

    pub fn update_timeout<F>(&mut self, update_fn: F)
    where
	F: FnOnce(Duration)
    {
	if let Some(tm) = self.timeout {
	    update_fn(tm);
	}
    }

    pub fn update_tsize(&mut self, new_sz: Option<u64>)
    {
	if let Some(sz) = self.tsize {
	    assert_eq!(sz, 0);
	    self.tsize = new_sz;
	}
    }

    pub fn fill_buf(self, msg: &mut Vec::<u8>)
    {
	msg.extend([0, 6]);

	self.block_size.map(|sz|  append_option(msg, b"blksize", sz));
	self.window_size.map(|sz| append_option(msg, b"windowsize", sz));
	self.tsize.map(|sz|       append_option(msg, b"tsize", sz));
	self.timeout.map(|to|     append_option(msg, b"timeout", to.as_secs()));
    }
}
