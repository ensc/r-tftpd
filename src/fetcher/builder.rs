use crate::{ Error, Result };
use regex::Regex;
use std::ffi::{ OsString, OsStr };
use std::path::{ PathBuf, Path };

use super::Fetcher;

lazy_static::lazy_static! {
    static ref URI_REGEX: Regex = Regex::new(r"^[a-z]+(\+[a-z]+)?://").unwrap();
}

pub struct Builder<'a> {
    env:	&'a crate::Environment,
}

fn normalize_path(p: &std::path::Path) -> Result<std::path::PathBuf>
{
    let mut res = std::path::PathBuf::new();

    for c in p.components() {
	use std::path::Component as C;

	match c {
	    C::RootDir		=> {},
	    C::CurDir		=> {},
	    C::Normal(p)	=> res.push(p),
	    // TODO: audit log this error
	    _			=> return Err(Error::InvalidPathName),
	}
    }

    assert!(!res.is_absolute());

    Ok(res)
}

#[derive(PartialEq, Debug)]
enum LookupResult {
    Path(PathBuf),
    #[cfg(feature = "proxy")]
    Uri(url::Url),
}

//#[instrument(level = "trace", skip_all, ret)]
fn lookup_path<A, B, C>(root: A, p: B, fallback: Option<C>) -> Result<LookupResult>
where
    A: AsRef<Path>,
    B: AsRef<Path>,
    C: AsRef<OsStr>,
{
    use std::os::unix::ffi::OsStrExt;

    let mut uri: Option<OsString> = None;
    let mut dir = root.as_ref().to_path_buf();
    let path_norm = normalize_path(p.as_ref())?;
    let mut is_dangling = false;

    for c in path_norm.components() {
	uri = match uri {
	    Some(mut u)		=> {
		let tmp = u.as_bytes();

		#[allow(clippy::len_zero)]
		if tmp.len() > 0 && tmp[tmp.len() - 1] != b'/' {
		    u.push(OsStr::from_bytes(b"/"));
		}

		u.push(c);
		Some(u)
	    },

	    None if is_dangling	=> {
		dir = dir.join(c);
		None
	    },

	    None		=> {
		let p = dir.join(c);
		let meta = p.symlink_metadata();

		is_dangling = meta.is_err();

		let uri_raw = if is_dangling || !meta.unwrap().is_symlink() {
		    None
		} else {
		    let data = std::fs::read_link(&p)?.into_os_string();
		    let data_str = data.to_str();

		    match data_str {
			Some(d) if URI_REGEX.is_match(d)	=> Some(data),
			_					=> None,
		    }
		};

		if uri_raw.is_none() {
		    dir = p;
		}

		uri_raw
	    }
	};
    }

    #[allow(clippy::unnecessary_unwrap)]
    if uri.is_none() && fallback.is_some() && !dir.exists() {
	let fallback = fallback.unwrap();
	let mut tmp: OsString = fallback.as_ref().to_os_string();

	tmp.push(path_norm.as_os_str());

	match fallback.as_ref().to_str() {
	    Some(d) if URI_REGEX.is_match(d)	=> uri = Some(tmp),
	    _					=> dir = tmp.into(),
	}
    }

    match uri.map(|u| u.to_str().map(|u| u.parse::<url::Url>())) {
	None			=> Ok(LookupResult::Path(dir)),
	Some(None)		=> Err(Error::StringConversion),
	Some(Some(Err(_)))	=> Err(Error::UriParse),
	#[cfg(feature = "proxy")]
	Some(Some(Ok(u)))	=> Ok(LookupResult::Uri(u)),
	#[cfg(not(feature = "proxy"))]
	Some(Some(Ok(_)))	=> Err(Error::NotImplemented),
    }
}

impl <'a> Builder<'a> {
    pub fn new(env: &'a crate::Environment) -> Self {
	Self {
	    env:	env,
	}
    }

    #[instrument(level = "trace", skip(self), ret)]
    pub fn instanciate(&'a self, p: &std::path::Path) -> Result<super::Fetcher> {
	match lookup_path(&self.env.dir, p, self.env.fallback_uri.as_ref())? {
	    LookupResult::Path(p)	=> Ok(Fetcher::new_file(&p)),
	    #[cfg(feature = "proxy")]
	    LookupResult::Uri(uri)	=> Ok(Fetcher::new_uri(&uri)),
	}
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_normalize() {
	use std::path::Path;

	assert_eq!(normalize_path(Path::new("/a/b/c")).unwrap(),
		   Path::new("a/b/c"));

	assert_eq!(normalize_path(Path::new("////a/b/c")).unwrap(),
		   Path::new("a/b/c"));

	assert_eq!(normalize_path(Path::new("a/b///c")).unwrap(),
		   Path::new("a/b/c"));

	assert_eq!(normalize_path(Path::new("./a/b/.//c")).unwrap(),
		   Path::new("a/b/c"));

	assert!(normalize_path(Path::new("a/b/../c")).is_err());
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn test_lookup() {
	use tempfile::TempDir;
	use std::os::unix::fs::symlink;

	let tmp_dir = TempDir::new().unwrap();
	let tmp_path = tmp_dir.path();

	std::fs::create_dir(tmp_path.join("a")).unwrap();
	std::fs::create_dir(tmp_path.join("b")).unwrap();

	std::fs::File::create(tmp_path.join("b/foo")).unwrap();

	symlink("http://test.example.com/foo",          tmp_path.join("a/link-0")).unwrap();
	symlink("http://test.example.com/bar/",         tmp_path.join("a/link-1")).unwrap();
	symlink("https://test.example.com/foo",         tmp_path.join("a/link-2")).unwrap();
	symlink("https+nocache://test.example.com/foo", tmp_path.join("a/link-3")).unwrap();
	symlink("./http://test.example.com/foo",        tmp_path.join("a/nolink-0")).unwrap();

	let fb_none = None::<OsString>;
	let _fb_some = Some::<OsString>("http://fb.example.com/redir/".into());


	assert_eq!(lookup_path(tmp_path, "/b/foo", fb_none.clone()).unwrap(),
		   LookupResult::Path(tmp_path.join("b/foo")));

	#[cfg(feature = "proxy")]
	{
	    assert_eq!(lookup_path(tmp_path, "/a/link-0", fb_none.clone()).unwrap(),
		       LookupResult::Uri("http://test.example.com/foo".parse().unwrap()));
	    assert_eq!(lookup_path(tmp_path, "/a/link-0/test", fb_none.clone()).unwrap(),
		       LookupResult::Uri("http://test.example.com/foo/test".parse().unwrap()));
	    assert_eq!(lookup_path(tmp_path, "/a/link-3/test", fb_none.clone()).unwrap(),
		       LookupResult::Uri("https+nocache://test.example.com/foo/test".parse().unwrap()));
	}

	assert_eq!(lookup_path(tmp_path, "/a/nolink-0", fb_none.clone()).unwrap(),
		   LookupResult::Path(tmp_path.join("a/nolink-0")));
	assert_eq!(lookup_path(tmp_path, "/a/nolink-0/file", fb_none.clone()).unwrap(),
		   LookupResult::Path(tmp_path.join("a/nolink-0/file")));
    }
}
