use std::sync::atomic::Ordering as O;

/// Simple, non-blocking semaphore implementation
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

    pub fn acquire(&self) -> Option<BucketGuard> {
	self.level
	    .fetch_update(O::Relaxed, O::Relaxed, |v| match v {
		0	=> None,
		v	=> Some(v - 1)
	    })
	    .map(|_| BucketGuard(self))
	    .ok()
    }

    fn release(&self) {
	self.level.fetch_add(1, O::Relaxed);
    }
}

pub struct BucketGuard<'a>(&'a Bucket);

impl Drop for BucketGuard<'_> {
    fn drop(&mut self) {
	self.0.release()
    }
}

impl BucketGuard<'_> {
    /// Some syntactic sugar around `drop()`
    pub fn release(_this: Option<Self>) {
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
	    assert!(g0.is_some());

	    let g1 = bucket.acquire();
	    assert_eq!(bucket.level(), 2);
	    assert!(g1.is_some());

	    {
		let g2 = bucket.acquire();
		assert_eq!(bucket.level(), 1);
		assert!(g2.is_some());

		let g3 = bucket.acquire();
		assert_eq!(bucket.level(), 0);
		assert!(g3.is_some());

		{
		    let g4 = bucket.acquire();
		    assert_eq!(bucket.level(), 0);
		    assert!(!g4.is_some());
		}

		assert_eq!(bucket.level(), 0);

		BucketGuard::release(g3);

		assert_eq!(bucket.level(), 1);

		let g5 = bucket.acquire();
		assert_eq!(bucket.level(), 0);
		assert!(g5.is_some());
	    }

	    assert_eq!(bucket.level(), 2);
	}
    }
}
