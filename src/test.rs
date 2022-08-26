use std::{os::unix::prelude::AsRawFd, io::Write};

use super::*;

const FILE_NAME: &str = "input";

fn create_file(p: &std::path::Path, fname: &str, mut sz: usize)
{
    let mut f = std::fs::File::create(p.join(fname)).unwrap();

    while sz > 0 {
	let buf = rand::random::<[u8; 4096]>();
	let w_sz = buf.len().min(sz);

	f.write_all(&buf[0..w_sz]).unwrap();

	sz -= w_sz;
    }
}

fn abort_server(addr: std::net::SocketAddr)
{
    let mut l_addr = addr;
    l_addr.set_port(0);

    let sock = std::net::UdpSocket::bind(l_addr).unwrap();
    sock.send_to(b"QQ", addr).unwrap();
}

struct FileSpec(&'static str, usize);

const FILES: &[FileSpec] = &[
    FileSpec("input_0",            0),
    FileSpec("input_511",        511),
    FileSpec("input_512",        512),
    FileSpec("input_513",        513),
    FileSpec("input_100000",  100000),
];

async fn run_test(ip: std::net::IpAddr)
{
    use tokio::time::timeout;
    use tokio::process::Command;
    use tempfile::TempDir;
    use std::process::{Stdio};

    let dir = TempDir::new().unwrap();

    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
	.join("scripts/run-tftp");

    for f in FILES {
	create_file(dir.path(), f.0, f.1);
    }

    let env = Environment {
	dir:			dir.path().into(),
	cache_dir:		std::env::temp_dir(),
	fallback_uri:		None,
	max_block_size:		1500,
	max_window_size:	64,
	max_connections:	1,
	timeout:		Duration::from_secs(3),
	no_rfc2374:		false,
	wrq_devnull:		true,

	#[cfg(feature = "proxy")]
	allow_uri:		true,
    };

    let addr = std::net::SocketAddr::new(ip, 0);
    let listen = std::net::UdpSocket::bind(addr).unwrap();
    let addr = listen.local_addr().unwrap();

    let h_server = tokio::task::spawn(timeout(Duration::from_secs(5),
					      run(env, Either::B(listen.as_raw_fd()))));
    let mut instance = 0;
    let mut do_abort = false;

    loop {
	for f in FILES {
	    let client = Command::new(&script)
		.arg(addr.ip().to_string())
		.arg(addr.port().to_string())
		.arg(f.0)
		.arg(instance.to_string())
		.stdin(Stdio::null())
		.current_dir(dir.path())
		.spawn()
		.expect("failed to start run-tftp")
		.wait()
		.await
		.expect("run-tftp failed");

	    match client.code() {
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
	}

	if do_abort {
	    abort_server(addr);
	    break;
	}

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
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tokio::test]
async fn test_ipv4() {
    let _g = TEST_LOCK.lock().unwrap();

    run_test(std::net::Ipv4Addr::LOCALHOST.into()).await;
}

#[tokio::test]
async fn test_ipv6() {
    let _g = TEST_LOCK.lock().unwrap();

    run_test(std::net::Ipv6Addr::LOCALHOST.into()).await;
}
