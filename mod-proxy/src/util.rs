pub struct PrettyDumpWrap<'a, T: PrettyDump>(&'a T);

impl <T: PrettyDump> std::fmt::Display for PrettyDumpWrap<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.pretty_dump(f)
    }
}

impl <'a, T: PrettyDump> PrettyDumpWrap<'a, T> {
    pub fn new(o: &'a T) -> Self {
	Self(o)
    }
}

pub fn pretty_dump_wrap<T: PrettyDump>(o: &T) -> PrettyDumpWrap<'_, T> {
    PrettyDumpWrap::new(o)
}

pub trait PrettyDump {
    fn pretty_dump(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl PrettyDump for std::time::SystemTime {
    fn pretty_dump(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	let tm = self.duration_since(Self::UNIX_EPOCH).unwrap();

	// tv_secs part in duration uses 31 so as_secs_f32() will not have a
	// precision; we could use as_secs_f64() here but using fixed point
	// arithmentic gives more accurate results


	f.write_fmt(format_args!("{}.{:03}", tm.as_secs(), tm.subsec_millis()))
    }
}

impl PrettyDump for std::time::Instant {
    // transform an 'Instant' to 'SystemTime': calculate duration of an
    // 'Instant' until now and subtract it from the current 'SystemTime'
    fn pretty_dump(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	let now_instant = Self::now();
	let now_system = std::time::SystemTime::now();

	let tm = if now_instant > *self {
	    now_system.checked_sub(now_instant.duration_since(*self))
	} else {
	    now_system.checked_add(self.duration_since(now_instant))
	}.ok_or(std::fmt::Error)?;

	tm.pretty_dump(f)
    }
}

impl <T: PrettyDump> PrettyDump for Option<T> {
    fn pretty_dump(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
	    None	=> f.write_str("n/a"),
	    Some(v)	=> v.pretty_dump(f)
	}
    }
}

impl crate::util::PrettyDump for reqwest::Response {
    fn pretty_dump(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} {}", self.status(), self.url()))?;

	if let Some(sz) = self.content_length() {
	    f.write_fmt(format_args!(" ({sz})"))?;
	}

	Ok(())
    }
}

impl crate::util::PrettyDump for reqwest::header::HeaderValue {
    fn pretty_dump(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	f.write_str(self.to_str().map_err(|_| std::fmt::Error)?)
    }
}

impl crate::util::PrettyDump for u64 {
    fn pretty_dump(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self}"))
    }
}

impl crate::util::PrettyDump for std::fs::File {
    fn pretty_dump(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	use std::os::fd::AsRawFd;

        f.write_fmt(format_args!("fd={}", self.as_raw_fd()))
    }
}
