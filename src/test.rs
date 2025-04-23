mod httpd;

use std::io::Write;
use std::path::Path;

use nix::libc::{kill, getpid};

use super::*;

fn create_file(p: &std::path::Path, fname: &str, mut sz: usize) -> std::io::Result<()>
{
    let mut f = std::fs::File::create(p.join(fname))?;

    while sz > 0 {
	let buf = rand::random::<[u8; 4096]>();
	let w_sz = buf.len().min(sz);

	f.write_all(&buf[0..w_sz])?;

	sz -= w_sz;
    }

    Ok(())
}

fn abort_server(addr: std::net::SocketAddr)
{
    let mut l_addr = addr;
    l_addr.set_port(0);

    let sock = std::net::UdpSocket::bind(l_addr).unwrap();
    sock.send_to(b"QQ", addr).unwrap();
}

#[derive(Debug)]
enum FileSpec {
    Content(&'static str, usize),
    Link(&'static str, &'static str, &'static str),

    TestPlain(&'static str),
    #[allow(dead_code)]
    TestProxy(&'static str, &'static str, &'static str),
}

impl FileSpec {
    pub fn create(&self, dir: &Path, host: Option<&str>) -> std::io::Result<()> {
	match (self, host) {
	    (Self::Content(name, sz), _)	=> create_file(dir, name, *sz),
	    (Self::Link(name, dst, proto), Some(host))	=>
		std::os::unix::fs::symlink(format!("http{proto}://{host}/{dst}"), dir.join(name)),
	    (Self::Link(_, _, _), _)		=> Ok(()),

	    (Self::TestPlain(_), _) |
	    (Self::TestProxy(_, _, _), _)	=> Ok(()),
	}
    }

    pub const fn is_available(&self) -> bool {
	match self {
	    Self::Content(_, _) |
	    Self::TestPlain(_)		=> true,
	    Self::Link(_, _, _) |
	    Self::TestProxy(_, _, _)	=> cfg!(feature = "proxy"),
	}
    }

    pub const fn get_file_name(&self) -> &str {
	match self {
	    Self::Content(name, _) |
	    Self::Link(name, _, _) |
	    Self::TestPlain(name) |
	    Self::TestProxy(name, _, _)		=> name,
	}
    }

    pub const fn get_reference(&self) -> &str {
	match self {
	    Self::Content(name, _) |
	    Self::Link(_, name, _) |
	    Self::TestPlain(name) |
	    Self::TestProxy(_, name, _)		=> name,
	}
    }
}

const FILES: &[FileSpec] = &[
    FileSpec::Content("input_0",            0),
    FileSpec::Content("input_511",        511),
    FileSpec::Content("input_512",        512),
    FileSpec::Content("input_513",        513),
    FileSpec::Content("input_100000",  100000),

    FileSpec::Link("proxy_0",      "input_0",      ""),
    FileSpec::Link("proxy_511",    "input_511",    "+nocache"),
    FileSpec::Link("proxy_511_0",  "input_511",    "+nocache"),
    FileSpec::Link("proxy_511_1",  "input_511",    ""),
    FileSpec::Link("proxy_511_2",  "input_511",    ""),
    FileSpec::Link("proxy_511_3",  "input_511",    "+nocache"),
    FileSpec::Link("proxy_512",    "input_512",    "+nocompress"),
    FileSpec::Link("proxy_513",    "input_513",    "+nocache+nocompress"),
    FileSpec::Link("proxy_100000", "input_100000", ""),

    FileSpec::TestPlain("input_513"),
    FileSpec::TestPlain("input_513"),

    FileSpec::TestProxy("proxy_511",    "input_511",    "+nocache"),
    FileSpec::TestProxy("proxy_511",    "input_511",    ""),
    FileSpec::TestProxy("proxy_511",    "input_511",    "+nocache"),
    FileSpec::TestProxy("proxy_511",    "input_511",    ""),
    FileSpec::TestProxy("proxy_511",    "input_511",    ""),
];

async fn run_test(ip: std::net::IpAddr)
{
    use tokio::time::timeout;
    use tokio::process::Command;
    use tempfile::TempDir;
    use std::process::Stdio;

    let dir = TempDir::new().unwrap();

    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
	.join("scripts/run-tftp");

    let mut http_server = httpd::Server::create(dir.path());

    for f in FILES.iter().filter(|f| f.is_available()) {
	f.create(dir.path(), http_server.as_ref().and_then(|s| s.get_host())).unwrap()
    }

    let env = Environment {
	dir:			dir.path().into(),
	cache_dir:		std::env::temp_dir(),
	fallback_uri:		None,
	max_block_size:		1500,
	max_window_size:	64,
	max_connections:	1,
	timeout:		Duration::from_secs(3),
	no_rfc2347:		false,
	wrq_devnull:		true,

	#[cfg(feature = "proxy")]
	allow_uri:		true,
    };

    let addr = std::net::SocketAddr::new(ip, 0);
    let listen = std::net::UdpSocket::bind(addr).unwrap();
    let addr = listen.local_addr().unwrap();

    #[allow(clippy::option_map_unit_fn)]
    {
	http_server.as_mut().map(|s| s.wait_for_ready());
    }

    let h_server = tokio::task::spawn(timeout(Duration::from_secs(5),
					      run(env, Either::B(listen.into()))));
    let mut instance = 0;
    let mut do_abort = false;

    loop {
	for f in FILES.iter().filter(|f| f.is_available()) {
	    debug!("running {f:?} test");

	    let client = Command::new(&script)
		.arg(addr.ip().to_string())
		.arg(addr.port().to_string())
		.arg(f.get_file_name())
		.arg(f.get_reference())
		.arg(instance.to_string())
		.stdin(Stdio::null())
		.current_dir(dir.path())
		.output()
		.await
		.expect("run-tftp failed");

	    if !client.stderr.is_empty() {
		warn!("run-tftp stderr:\n{}", String::from_utf8_lossy(&client.stderr));
	    }

	    if !client.stdout.is_empty() {
		debug!("run-tftp stdout:\n{}", String::from_utf8_lossy(&client.stdout));
	    }

	    match client.status.code() {
		Some(0)		=> {},
		Some(23)	=> {
		    println!("run-tftp #{} skipped", instance);
		    break;
		},
		Some(42)	=> {
		    do_abort = true;
		    break;
		}
		_		=> panic!("run-tftpd failed: {:?}", client),
	    }

	    unsafe { kill(getpid(), nix::libc::SIGUSR1) };
	}

	if do_abort {
	    abort_server(addr);
	    break;
	}

	unsafe { kill(getpid(), nix::libc::SIGUSR2) };

	instance += 1
    }

    h_server.await
	.expect("tftp server timed out")
	.expect("tftp server failed")
	.unwrap();

    #[cfg(feature = "proxy")]
    crate::fetcher::Cache::close().await;
}

// switching tokio runtime between tests breaks the Cache singleton
lazy_static::lazy_static! {
    static ref TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::new(());
}

static LOG_LOCK: std::sync::Mutex<bool> = std::sync::Mutex::new(false);

pub fn init_logging() {
    let mut log = LOG_LOCK.lock().unwrap();

    if !*log {
	tracing_subscriber::fmt()
	    .with_env_filter(tracing_subscriber::EnvFilter::from_env("RUST_TEST_LOG"))
	    .with_writer(tracing_subscriber::fmt::writer::TestWriter::new())
	    .init();

	*log = true;
    }
}

#[tokio::test]
async fn test_ipv4() {
    let _g = TEST_LOCK.lock().await;

    init_logging();

    run_test(std::net::Ipv4Addr::LOCALHOST.into()).await;
}

#[tokio::test]
async fn test_ipv6() {
    let _g = TEST_LOCK.lock().await;

    init_logging();

    run_test(std::net::Ipv6Addr::LOCALHOST.into()).await;
}
