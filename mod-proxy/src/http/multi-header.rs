use reqwest::header::HeaderValue;
use crate::Result;

pub struct MultiHeaderIterator<'a, T> {
    values: reqwest::header::ValueIter<'a, HeaderValue>,
    header: Option<&'a [u8]>,
    pos: usize,
    _m: std::marker::PhantomData<T>,
}

impl <'a, T> MultiHeaderIterator<'a, T> {
    pub fn new(headers: reqwest::header::GetAll<'a, HeaderValue>) -> Self {
	let mut tmp = headers.into_iter();
	let hdr = tmp.next().map(|v| v.as_bytes());

	Self {
	    values:	tmp,
	    header:	hdr,
	    pos:	0,
	    _m:		std::marker::PhantomData,
	}
    }

    fn split<'b>(hdr: &'b [u8], pos: &mut usize) -> Option<(&'b [u8], Option<&'b [u8]>)> {
	let mut q = None;	// start of data
	let mut p = None;	// end of data
	let mut d = None;	// delim
	let mut e = hdr.len();	//

	for (i, c) in hdr.iter().enumerate().skip(*pos) {
	    let c = match c {
		// normalize whitespace
		b' ' | b'\t'		=> b' ',
		c			=> *c,
	    };

	    if q.is_none() {
		match c {
		    b' ' | b','		=> {},
		    _			=> q = Some(i),
		}
	    } else {
		match c {
		    b'=' if d.is_none()	=> d = Some(i),
		    b' '		=> p = p.or(Some(i)),
		    b','		=> {
			e = i + 1;
			p = p.or(Some(i));
			break;
		    }
		    _			=> p = None,
		}
	    }
	}

	let p = p.unwrap_or(e);

	*pos = e;

	match (q, d) {
	    (None, _)		=> None,
	    (Some(q), None)	=> Some((&hdr[q..p], None)),
	    (Some(q), Some(d))	=> Some((&hdr[q..d], Some(&hdr[d + 1..p]))),
	}
    }
}

impl <'a, T> std::iter::Iterator for MultiHeaderIterator<'a, T>
where
    T: TryFrom<(&'a [u8], Option<&'a [u8]>), Error = crate::Error>
{
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
	let mut hdr = self.header?;
	let mut pos = self.pos;

	let res = 'exit: loop {
	    match Self::split(hdr, &mut pos) {
		None	=> {
		    pos = 0;
		    match self.values.next().map(|v| v.as_bytes()) {
			None	=> break 'exit None,
			Some(h)	=> hdr = h,
		    };
		},

		Some(v)	=> break Some(v),
	    }
	};

	if res.is_some() {
	    self.header = Some(hdr);
	    self.pos    = pos;
	} else {
	    self.header = None;
	    self.pos    = 0;
	}

	res.map(|v| v.try_into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Error;

    #[derive(Debug, Eq, PartialEq)]
    struct Info {
	a: u32,
	b: Option<u32>,
    }

    impl TryFrom<(&[u8], Option<&[u8]>)> for Info {
	type Error = Error;

	fn try_from((key, val): (&[u8], Option<&[u8]>)) -> Result<Self> {
	    Ok(Info {
		a: String::from_utf8_lossy(key).parse().map_err(|_| Error::StringConversion)?,
		b: match val.map(|v| String::from_utf8_lossy(v).parse().map_err(|_| Error::StringConversion)) {
		    Some(Err(e))	=> return Err(e),
		    Some(Ok(v))		=> Some(v),
		    None		=> None,
		}
	    })
	}
    }

    impl PartialEq<(u32,)> for Info {
        fn eq(&self, rhs: &(u32,)) -> bool {
            self.a == rhs.0 && self.b.is_none()
	}
    }

    impl PartialEq<(u32, u32)> for Info {
        fn eq(&self, rhs: &(u32, u32)) -> bool {
            self.a == rhs.0 && self.b == Some(rhs.1)
	}
    }

    type InfoIterator<'a> = MultiHeaderIterator<'a, Info>;

    #[test]
    fn test_00() {
	use reqwest::header::HeaderMap;

	let mut map = HeaderMap::new();
	map.append("foo", "0".parse().unwrap());
	map.append("foo", "10,11,12".parse().unwrap());
	map.append("foo", ",,,20,,,  21  ,,,  22,,,23  ,,,".parse().unwrap());
	map.append("foo", "30=1,31=, 32=2,33=3 ,34=".parse().unwrap());

	let mut step = 0;
	for i in InfoIterator::new(map.get_all("foo")) {
	    match step {
		0	=> assert_eq!(i.unwrap(), (0,)),
		1	=> assert_eq!(i.unwrap(), (10,)),
		2	=> assert_eq!(i.unwrap(), (11,)),
		3	=> assert_eq!(i.unwrap(), (12,)),
		4	=> assert_eq!(i.unwrap(), (20,)),
		5	=> assert_eq!(i.unwrap(), (21,)),
		6	=> assert_eq!(i.unwrap(), (22,)),
		7	=> assert_eq!(i.unwrap(), (23,)),
		8	=> assert_eq!(i.unwrap(), (30, 1)),
		9	=> assert!(i.is_err()),
		10	=> assert_eq!(i.unwrap(), (32, 2)),
		11	=> assert_eq!(i.unwrap(), (33, 3)),
		12	=> assert!(i.is_err()),
		_	=> panic!("too much results"),
	    }

	    step += 1;
	}

	assert_eq!(step, 13);
    }
}
