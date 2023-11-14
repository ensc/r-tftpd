use std::time::{SystemTime, Duration, Instant};

#[derive(Copy, Clone, Debug)]
pub struct Time {
    pub local:	SystemTime,
    pub mono:	std::time::Instant,
}

impl Time {
    pub fn now() -> Self {
	Self {
	    local:	SystemTime::now(),
	    mono:	std::time::Instant::now(),
	}
    }
}

impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool {
        self.mono == other.mono
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.mono.partial_cmp(&other.mono)
    }
}

#[derive(Debug)]
pub struct TimeDelta(i64);

impl TimeDelta {
    pub fn from_systemtime(a: SystemTime, b: SystemTime) -> Option<Self> {
	let (delta, sign) = match a < b {
	    true	=> (b.duration_since(a),  1),
	    false	=> (a.duration_since(b), -1),
	};

	delta.ok()
	    .and_then(|t| i64::try_from(t.as_nanos()).ok())
	    .or_else(|| {
		warn!("failed to calculate delta of {a:?} and {b:?}");
		None
	    })
	    .map(|t| match sign {
		s if s < 0	=> -t,
		_		=>  t,
	    })
	    .map(Self)
    }
}

impl std::ops::Add<TimeDelta> for SystemTime {
    type Output = Option<SystemTime>;

    fn add(self, rhs: TimeDelta) -> Self::Output {
	match rhs.0 < 0 {
	    true	=> self.checked_sub(Duration::from_nanos(-rhs.0 as u64)),
	    false	=> self.checked_add(Duration::from_nanos( rhs.0 as u64)),
	}
    }
}

impl std::ops::Sub<TimeDelta> for SystemTime {
    type Output = Option<SystemTime>;

    fn sub(self, rhs: TimeDelta) -> Self::Output {
	match rhs.0 < 0 {
	    true	=> self.checked_add(Duration::from_nanos(-rhs.0 as u64)),
	    false	=> self.checked_sub(Duration::from_nanos( rhs.0 as u64)),
	}
    }
}

impl std::ops::Add<TimeDelta> for Instant {
    type Output = Option<Instant>;

    fn add(self, rhs: TimeDelta) -> Self::Output {
	match rhs.0 < 0 {
	    true	=> self.checked_sub(Duration::from_nanos(-rhs.0 as u64)),
	    false	=> self.checked_add(Duration::from_nanos( rhs.0 as u64)),
	}
    }
}

impl std::ops::Sub<TimeDelta> for Instant {
    type Output = Option<Instant>;

    fn sub(self, rhs: TimeDelta) -> Self::Output {
	match rhs.0 < 0 {
	    true	=> self.checked_add(Duration::from_nanos(-rhs.0 as u64)),
	    false	=> self.checked_sub(Duration::from_nanos( rhs.0 as u64)),
	}
    }
}
