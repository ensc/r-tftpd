#[derive(Copy, Clone, Debug)]
pub struct Time {
    pub local:	std::time::SystemTime,
    pub mono:	std::time::Instant,
}

impl Time {
    pub fn now() -> Self {
	Self {
	    local:	std::time::SystemTime::now(),
	    mono:	std::time::Instant::now(),
	}
    }

    pub fn checked_duration_since(&self, tm: Time) -> Option<std::time::Duration> {
	self.mono.checked_duration_since(tm.mono)
    }

    pub fn checked_add(&self, duration: std::time::Duration) -> Option<Self> {
	match (self.local.checked_add(duration), self.mono.checked_add(duration)) {
	    (Some(l), Some(m))	=> Some(Self {
		local:	l,
		mono:	m,
	    }),

	    _			=> None,
	}
    }

    pub fn relative(&self, systm: std::time::SystemTime) -> Option<Self> {
	let mono = if self.local < systm {
	    self.mono.checked_add(systm.duration_since(self.local).unwrap())
	} else {
	    self.mono.checked_sub(self.local.duration_since(systm).unwrap())
	}?;

	Some(Self {
	    local:	systm,
	    mono:	mono,
	})
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
