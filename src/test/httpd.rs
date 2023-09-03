use std::env;
use std::fs::File;
use std::io::Write;
use std::net::Ipv4Addr;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio, Child};
use std::time::Duration;

use tempfile::TempDir;

pub struct Server {
    process:	Option<Child>,
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
const LIGHTTP_WRAP: &str = r#"#!/bin/sh
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

server.modules += ( "mod_expire" )

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

    unsafe fn remove_cloexec(fd: RawFd) -> nix::libc::c_int {
	use nix::libc as libc;
	use libc::FD_CLOEXEC;

	match libc::fcntl(fd, libc::F_GETFD) {
	    e if e < 0			=> e,
	    f if f & FD_CLOEXEC == 0	=> 0,
	    f				=> libc::fcntl(fd, libc::F_SETFD, f & !FD_CLOEXEC),
	}
    }

    unsafe fn dup_nocloexec(old_fd: RawFd, new_fd: RawFd) -> std::io::Result<()> {
	// TODO: this happens only occasionally and causes randomness in our
	// code coverage tests :(
	let rc = match old_fd == new_fd {
	    false	=> nix::libc::dup3(old_fd, new_fd, 0),
	    true	=> Self::remove_cloexec(old_fd),
	};

	if rc < 0 {
	    return Err(std::io::Error::last_os_error());
	}

	Ok(())
    }

    fn create_lighttpd(dir: &Path) -> Self
    {
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

	let mut proc = Command::new("/bin/sh");

	proc
	    .arg(prog)
	    .arg("-D")
	    .arg("-i30")
	    .arg("-f")
	    .arg(conf)
	    .current_dir("/")
	    .env("PATH",
		 "/sbin:/usr/sbin:/usr/local/sbin:".to_owned() + &env::var("PATH").unwrap_or("/bin".to_string()))
	    .env("LISTEN_FDS", "1")
	    .stdin(Stdio::null())
	    .stdout(Stdio::inherit())
	    .stderr(Stdio::inherit());

	let sock_fd = sock.as_raw_fd();

	unsafe {
	    proc.pre_exec(move || {
		Self::dup_nocloexec(sock_fd, 3)
	    })
	};

	let child = proc.spawn().unwrap();

	Self {
	    process:	Some(child),
	    _tmpdir:	tmpdir,
	    host:	host,
	}
    }

    pub fn get_host(&self) -> Option<&str> {
	Some(&self.host)
    }

    pub fn wait_for_ready(&mut self) {
	std::thread::sleep(Duration::from_millis(500));

	match self.process.take() {
	    None		=> panic!("no process"),
	    Some(mut proc)	=> match proc.try_wait() {
		Ok(None)	=> self.process = Some(proc),
		res		=> panic!("lighttpd exited: {res:?}"),
	    }
	}
    }
}

impl std::ops::Drop for Server {
    fn drop(&mut self) {
	if let Some(mut proc) = self.process.take() {
            let _ = proc.kill();
	    let _ = proc.wait();
	}
    }
}
