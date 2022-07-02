use std::sync::atomic::Ordering as O;

pub struct Bucket {
    level:	std::sync::atomic::AtomicU32,
}

impl Bucket {
    pub fn new(level: u32) -> Self {
	Self {
	    level:	level.into(),
	}
    }

    #[cfg(test)]
    pub fn level(&self) -> u32 {
	self.level.load(O::Relaxed)
    }

    pub fn acquire(&self) -> BucketGuard {
	let do_release = self.level
	    .fetch_update(O::Relaxed, O::Relaxed, |v| match v {
		0	=> None,
		v	=> Some(v - 1)
	    }).is_ok();

	BucketGuard {
	    bucket:	self,
	    do_release:	do_release,
	}
    }

    fn release(&self) {
	self.level.fetch_add(1, O::Relaxed);
    }
}

pub struct BucketGuard<'a> {
    bucket:	&'a Bucket,
    do_release:	bool,
}

impl Drop for BucketGuard<'_> {
    fn drop(&mut self) {
	if self.do_release {
	    self.bucket.release()
	}
    }
}

impl BucketGuard<'_> {
    pub fn is_ok(&self) -> bool {
	self.do_release
    }

    pub fn release(self) {
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bucket() {
	let bucket = Bucket::new(4);

	assert_eq!(bucket.level(), 4);

	{
	    let g0 = bucket.acquire();
	    assert_eq!(bucket.level(), 3);
	    assert!(g0.is_ok());

	    let g1 = bucket.acquire();
	    assert_eq!(bucket.level(), 2);
	    assert!(g1.is_ok());

	    {
		let g2 = bucket.acquire();
		assert_eq!(bucket.level(), 1);
		assert!(g2.is_ok());

		let g3 = bucket.acquire();
		assert_eq!(bucket.level(), 0);
		assert!(g3.is_ok());

		{
		    let g4 = bucket.acquire();
		    assert_eq!(bucket.level(), 0);
		    assert!(!g4.is_ok());
		}

		assert_eq!(bucket.level(), 0);

		g3.release();

		assert_eq!(bucket.level(), 1);

		let g5 = bucket.acquire();
		assert_eq!(bucket.level(), 0);
		assert!(g5.is_ok());
	    }

	    assert_eq!(bucket.level(), 2);
	}
    }
}
