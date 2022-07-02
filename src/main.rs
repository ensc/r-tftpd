#![allow(clippy::redundant_field_names)]
#![allow(dead_code)]
//#![allow(unused_variables)]

#[macro_use]
extern crate tracing;

mod tftp;
pub mod errors;
pub mod util;
pub mod fetcher;

use std::sync::Arc;
use util::{ UdpSocket, UdpRecvInfo, SocketAddr, Bucket };

use tftp::{ Session, SessionStats };

pub use errors::{ Error, Result };


pub struct Environment {
    dir:		std::path::PathBuf,
    fallback_uri:	Option<std::ffi::OsString>,
    max_block_size:	u16,
    max_window_size:	u16,
    max_connections:	u32,
    timeout:		std::time::Duration,
}

struct SpeedInfo {
    duration:		std::time::Duration,
    stats:		SessionStats,
}

impl SpeedInfo {
    pub fn new(now: std::time::Instant, stats: SessionStats) -> Self {
	Self {
	    duration:	now.elapsed(),
	    stats:	stats,
	}
    }
}

impl std::fmt::Display for SpeedInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	write!(f, "{}ms", self.duration.as_millis())
    }
}

use tracing::field::Empty;

#[instrument(skip_all,
	     fields(remote = Empty,
		    local = Empty,
		    filename = Empty,
		    filesize = Empty,
		    op = Empty))]
async fn handle_request(env: std::sync::Arc<Environment>,
			info: UdpRecvInfo,
			req: Vec<u8>,
			bucket: Arc<Bucket>)
{
    let instant = std::time::Instant::now();
    let session = Session::new(&env, info.remote, info.local).await;

    if let Err(e) = session {
	warn!("failed to create tftp session: {:?}", e);
	return;
    }

    let session = session.unwrap();

    let b = bucket.acquire();

    let res = match b.is_ok() {
	false	=> session.do_reject().await,
	true	=> session.run(req).await
    };

    match res {
	    Ok(stats)	=> info!("completed in {}", SpeedInfo::new(instant, stats)),
	    Err(e)	=> error!("request failed: {:?}", e),
    };
}

async fn run_tftpd_loop(env: std::sync::Arc<Environment>, sock: UdpSocket) -> Result<()> {
    let mut buf = vec![0u8; 1500];

    let bucket = Arc::new(Bucket::new(env.max_connections));

    loop {
	let info = sock.recvmsg(&mut buf).await?;
	let request = Vec::from(&buf[..info.size]);

	tokio::task::spawn(handle_request(env.clone(), info,
					  request, bucket.clone()));
    }
}

#[tokio::main(flavor = "current_thread")]
async fn run(env: Environment, mut sock: UdpSocket) -> Result<()> {
    sock.init_async_fd()?;
    sock.set_request_pktinfo()?;

    run_tftpd_loop(std::sync::Arc::new(env), sock).await?;

    Ok(())
}

use clap::Parser;

#[derive(clap::Parser, Debug)]
struct CliOpts {
    #[clap(short, long, help("use systemd fd propagation"), value_parser)]
    systemd:		bool,

    #[clap(short, long, value_parser, help("port to listen on"), default_value("69"))]
    port:		u16,

    #[clap(short, long, value_parser, help("ip address to listen on"),
	   value_name("IP"), default_value("::"))]
    listen:		std::net::IpAddr,

    #[clap(short, long, value_parser, help("maximum number of connections"),
	   value_name("NUM"), default_value("64"))]
    max_connections:	u32,

    #[clap(short, long, value_parser, help("timeout in seconds during tftp transfers"),
	   default_value("3"))]
    timeout:		f32,

    #[clap(short, long, value_parser, value_name("URI"), help("fallback uri"))]
    fallback:		Option<String>,
}

fn main() {
    tracing_subscriber::fmt::init();

    let args = CliOpts::parse();

    let env = Environment {
	dir:			".".into(),
	fallback_uri:		args.fallback.map(|s| s.into()),
	max_block_size:		1500,
	max_window_size:	64,
	max_connections:	args.max_connections,
	timeout:		std::time::Duration::from_secs_f32(args.timeout),
    };

    let fd = match args.systemd {
	true	=> listenfd::ListenFd::from_env()
	    .take_raw_fd(0)
	    .unwrap(),
	false	=> None
    };

    let sock = match fd {
	None		=> {
	    let addr = SocketAddr::new(args.listen, args.port);
	    UdpSocket::bind_noasync(addr)
	},

	Some(fd)	=> UdpSocket::from_raw(fd),
    }.expect("failed to bind socket");

    run(env, sock).unwrap();
}
