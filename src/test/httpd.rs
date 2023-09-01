use std::env;
use std::fs::File;
use std::io::Write;
use std::net::Ipv4Addr;
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio, Child};

use tempfile::TempDir;

pub struct Server {
    process:	Child,
    _tmpdir:	TempDir,
    host:	String,
}

const LIGHTTP_PROG: &str = match option_env!("LIGHTTP_PROG") {
    Some(e)	=> e,
    None	=> "lighttpd",
};

// TODO: we have to set LISTEN_PID to the pid after 'fork()'.  But pre_exec()
// does not seem to support updating the environment.  Call 'lighttpd' through
// a wrapper
const LIGHTTP_WRAP: &str = r#"
#!/bin/sh
export LISTEN_PID=$$
exec @LIGHTTP@ "$@"
"#;

const LIGHTTP_CONF: &str = r#"
var.conf_dir = "@TMPDIR@"
var.state_dir = "@TMPDIR@"
var.log_root = "@TMPDIR@"
var.cache_dur = "@TMPDIR@"
var.server_root = "@TMPDIR@"

server.document-root = "@DATADIR@"
server.use-ipv6 = "disable"

server.modules += ( "mod_deflate" )
deflate.allowed-encodings = ("brotli", "gzip", "deflate")

server.systemd-socket-activation = "enable"
"#;

impl Server {
    pub fn create(dir: &Path) -> Option<Self>
    {
	match cfg!(feature = "proxy") {
	    true	=> Some(Self::create_lighttpd(dir)),
	    false	=> None,
	}
    }

    fn create_lighttpd(dir: &Path) -> Self
    {
	// make sure that 'sock' below is not assigned to fd #3 by opening a
	// dummy file which consumes the next open fd.  fds 0-2 should be in
	// use by stdXXX so this is at least #3.
	let _tmp = File::open("/dev/null").unwrap();

	let addr = std::net::SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0);
	let sock = std::net::TcpListener::bind(addr).unwrap();
	let host = format!("localhost:{}", sock.local_addr().unwrap().port());

	let tmpdir = TempDir::new().unwrap();

	let conf = tmpdir.path().join("httpd.conf");
	let prog = tmpdir.path().join("lighttp-wrap");

	File::create(&prog).unwrap()
	    .write_all(LIGHTTP_WRAP
		       .replace("@LIGHTTP@", LIGHTTP_PROG)
		       .as_bytes())
	    .unwrap();

	File::create(&conf).unwrap()
	    .write_all(LIGHTTP_CONF
		       .replace("@TMPDIR@", tmpdir.path().to_str().unwrap())
		       .replace("@DATADIR@", dir.to_str().unwrap())
		       .as_bytes())
	    .unwrap();

	let mut proc = Command::new("sh");

	proc
	    .arg(prog)
	    .arg("-f")
	    .arg(conf)
	    .env("PATH",
		 "/sbin:/usr/sbin:/usr/local/sbin:".to_owned() + &env::var("PATH").unwrap_or("/bin".to_string()))
	    .env("LISTEN_FDS", "1")
	    .stdin(Stdio::null());

	let sock_fd = sock.as_raw_fd();

	// dup3 below relies on that
	assert_ne!(sock_fd, 3);

	unsafe {
	    #[allow(unused_unsafe)]
	    proc.pre_exec(move || {
		let rc = unsafe { nix::libc::dup3(sock_fd, 3, 0) };

		if rc < 0 {
		    return Err(std::io::Error::last_os_error());
		}

		Ok(())
	    })
	};

	let child = proc.spawn().unwrap();

	Self {
	    process:	child,
	    _tmpdir:	tmpdir,
	    host:	host,
	}
    }

    pub fn get_host(&self) -> Option<&str> {
	Some(&self.host)
    }
}

impl std::ops::Drop for Server {
    fn drop(&mut self) {
        let _ = self.process.kill();
	let _ = self.process.wait();
    }
}
