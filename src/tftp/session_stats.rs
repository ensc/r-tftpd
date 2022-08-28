use crate::util::ToFormatted;

#[derive(Copy, Clone, Default, Debug)]
pub enum Direction {
    Wrq,
    #[default]
    Rrq,
}

impl Direction {
    pub fn as_arrow(&self) -> &'static str {
	match self {
	    Self::Wrq	=> "<=",
	    Self::Rrq	=> "=>",
	}
    }
}

#[derive(Default, Debug)]
pub struct Stats {
    pub direction:	Direction,
    pub filesize:	u64,
    pub xmitsz:		u64,
    pub retries:	u32,
    pub wastedsz:	u64,
    pub num_timeouts:	u32,
    pub window_size:	u16,
    pub block_size:	u16,
    pub filename:	String,
    pub remote_ip:	String,
    pub local_ip:	String,
    pub is_complete:	bool,
}

impl Stats {
    pub fn has_errors(&self) -> bool {
	self.filesize != self.xmitsz ||
	    self.retries != 0 ||
	    self.wastedsz != 0 ||
	    self.num_timeouts != 0
    }

    pub fn speed_bit_per_s(&self, duration: std::time::Duration) -> Option<(f32, f32)> {
	if duration.is_zero() {
	    return None;
	}

	Some(((self.filesize as f64 / duration.as_secs_f64()) as f32,
	      (self.xmitsz as f64 / duration.as_secs_f64()) as f32))
    }
}

impl std::fmt::Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	write!(f, "\"{}\" ({} {} {}, {}x {})", self.filename,
	       self.local_ip, self.direction.as_arrow(), self.remote_ip,
	       self.window_size, self.block_size)?;

	match self.direction {
	    Direction::Rrq	=> {
		write!(f, " {} bytes", self.filesize.to_formatted())?;

		if self.has_errors() {
		    write!(f, ", sent={} ({} retries, {} blocks wasted, {} timeouts)",
			   self.xmitsz.to_formatted(),
			   self.retries, self.wastedsz.to_formatted(),
			   self.num_timeouts)?
		}
	    },

	    Direction::Wrq	=> {
		write!(f, " {} bytes", self.xmitsz.to_formatted())?;

		if self.has_errors() {
		    write!(f, " ({} retries, {} blocks wasted, {} timeouts)",
			   self.retries, self.wastedsz.to_formatted(),
			   self.num_timeouts)?
		}
	    }
	}

	Ok(())
    }
}
