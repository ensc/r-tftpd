#![allow(clippy::redundant_field_names)]
//#![allow(dead_code)]
//#![allow(unused_variables)]

#[macro_use]
extern crate tracing;

mod tftp;
pub mod errors;
pub mod util;
pub mod fetcher;

use std::os::fd::{OwnedFd, FromRawFd};
use std::sync::Arc;
use std::time::Duration;
use util::{ UdpSocket, UdpRecvInfo, SocketAddr, Bucket, ToFormatted };

use tftp::{ Session, SessionStats };

pub use errors::{ Error, Result };

#[cfg(test)]
mod test;

pub struct Environment {
    dir:		std::path::PathBuf,
    cache_dir:		std::path::PathBuf,
    fallback_uri:	Option<std::ffi::OsString>,
    max_block_size:	u16,
    max_window_size:	u16,
    max_connections:	u32,
    timeout:		Duration,
    no_rfc2347:		bool,
    wrq_devnull:	bool,

    #[cfg(feature = "proxy")]
    allow_uri:		bool,
}

impl Environment {
    pub fn allow_uri(&self) -> bool {
	#[cfg(feature = "proxy")]
	return self.allow_uri;

        #[cfg(not(feature = "proxy"))]
	false
    }
}

struct SpeedInfo<'a> {
    duration:		Duration,
    stats:		&'a SessionStats,
}

impl <'a> SpeedInfo<'a> {
    pub fn new(now: std::time::Instant, stats: &'a SessionStats) -> Self {
	Self {
	    duration:	now.elapsed(),
	    stats:	stats,
	}
    }
}

impl std::fmt::Display for SpeedInfo<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	write!(f, "duration={} ms", self.duration.as_millis().to_formatted())?;

	match self.stats.speed_bit_per_s(self.duration) {
	    None			=> Ok(()),
	    Some((speed_f, speed_n)) if speed_f == speed_n	=>
		write!(f, " => total={} bytes/s",
		       (speed_f as u64).to_formatted()),

	    Some((_, speed_n)) if !self.stats.is_complete	=>
		write!(f, " => net={} bytes/s", (speed_n as u64).to_formatted()),

	    Some((speed_f, speed_n))	=>
		write!(f, " => file={} bytes/s, net={} bytes/s",
		       (speed_f as u64).to_formatted(),
		       (speed_n as u64).to_formatted()),
	}
    }
}

async fn sigusr1_handler(mut stream: tokio::signal::unix::Signal)
{
    loop {
	stream.recv().await;
	debug!("got SIGUSR1");

	#[cfg(feature = "proxy")]
	fetcher::Cache::dump().await;
    }
}

async fn sigusr2_handler(mut stream: tokio::signal::unix::Signal)
{
    loop {
	stream.recv().await;
	debug!("got SIGUSR2");

	#[cfg(feature = "proxy")]
	fetcher::Cache::clear().await;
    }
}

fn init_sighandlers() -> Result<()>
{
    use tokio::signal::unix::{ signal, SignalKind };

    let stream = signal(SignalKind::from_raw(nix::libc::SIGUSR1))?;
    tokio::spawn(sigusr1_handler(stream));

    let stream = signal(SignalKind::from_raw(nix::libc::SIGUSR2))?;
    tokio::spawn(sigusr2_handler(stream));

    Ok(())
}

use tracing::field::Empty;

#[instrument(skip_all,
	     fields(id = id,
		    remote = Empty,
		    local = Empty,
		    filename = Empty,
		    op = Empty))]
async fn handle_request(env: std::sync::Arc<Environment>,
			id: u64,
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

    let res = match b.is_some() {
	false	=> session.do_reject().await,
	true	=> session.run(req).await
    };

    match res {
	Ok(stats)	=> {
	    info!(parent: tracing::Span::none(),
		  "conn#{}: {}, {}", id, &stats, SpeedInfo::new(instant, &stats))
	},
	Err(e)	=> error!("request failed: {:?}", e),
    };
}

async fn run_tftpd_loop(env: std::sync::Arc<Environment>, sock: UdpSocket) -> Result<()> {
    let mut buf = vec![0u8; 1500];

    let bucket = Arc::new(Bucket::new(env.max_connections));
    let mut num = 0;

    loop {
	let info = sock.recvmsg(&mut buf).await?;
	let request = Vec::from(&buf[..info.size]);

	#[cfg(test)]
	if &request[0..2] == b"QQ" {
	    break Ok(());
	}

	tokio::task::spawn(handle_request(env.clone(), num, info,
					  request, bucket.clone()));

	num += 1;
    }
}

enum Either<T: Sized, U: Sized> {
    A(T),
    B(U),
}

async fn run(env: Environment, info: Either<SocketAddr, OwnedFd>) -> Result<()> {
    #[cfg(feature = "proxy")]
    {
	#[allow(clippy::identity_op)]
	let gc_props = fetcher::CacheGcProperties {
	    max_elements:	50,
	    max_lifetime:	Duration::from_secs(1 * 3600),
	    sleep:		Duration::from_secs(30),
	};

	fetcher::Cache::instanciate(&env.cache_dir, gc_props);
    }


    // UdpSocket creation must happen with active Tokio runtime
    let mut sock = match info {
	Either::A(addr)	=> UdpSocket::bind(&addr),
	Either::B(fd)	=> UdpSocket::from_raw(fd),
    }?;

    sock.set_nonblocking()?;
    sock.set_request_pktinfo()?;

    init_sighandlers()?;

    run_tftpd_loop(std::sync::Arc::new(env), sock).await
}

#[tokio::main(flavor = "current_thread")]
async fn tokio_main(env: Environment, info: Either<SocketAddr, OwnedFd>) -> Result<()> {
    run(env, info).await
}

use clap::Parser;

#[derive(clap::ValueEnum)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LogFormat {
    Default,
    Compact,
    Full,
    Json,
}

#[derive(clap::Parser, Debug)]
#[clap(author, version, about)]
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

    #[clap(short('L'), long, value_parser, value_name("FMT"), help("log format"),
	   default_value("default"))]
    log_format:		LogFormat,

    #[clap(short('C'), long, value_parser, value_name("DIR"),
	   help("directory used for cache files"))]
    cache_dir:		Option<String>,

    #[clap(long, help("disable RFC 2347 (OACK) support; only useful for testing some clients"),
	   value_parser)]
    no_rfc2347:		bool,

    #[clap(long, help("accept WRQ but throw it away; only useful for testing some clients"),
	   value_parser)]
    wrq_devnull:	bool,

    #[cfg(feature = "proxy")]
    #[clap(long, help("disable proxy support"), value_parser)]
    disable_proxy:	bool,
}

fn main() {
    let mut args = CliOpts::parse();

    if args.log_format == LogFormat::Default {
	args.log_format = if args.systemd {
	    // when running under systemd, do not emit the timestamp because
	    // output is usually recorded in the journal.  Accuracy in journal
	    // should suffice for most usecases.

	    LogFormat::Compact
	} else {
	    LogFormat::Full
	}
    }

    let fmt = tracing_subscriber::fmt()
	.with_env_filter(tracing_subscriber::EnvFilter::from_default_env());

    match args.log_format {
	LogFormat::Compact		=> fmt.without_time().init(),
	LogFormat::Json			=> fmt.json().init(),
	LogFormat::Full			=> fmt.init(),
	LogFormat::Default		=> unreachable!(),
    }

    let env = Environment {
	dir:			".".into(),
	cache_dir:		args.cache_dir.map(|s| s.into()).unwrap_or_else(std::env::temp_dir),
	fallback_uri:		args.fallback.map(|s| s.into()),
	max_block_size:		1500,
	max_window_size:	64,
	max_connections:	args.max_connections,
	timeout:		Duration::from_secs_f32(args.timeout),
	no_rfc2347:		args.no_rfc2347,
	wrq_devnull:		args.wrq_devnull,

	#[cfg(feature = "proxy")]
	allow_uri:		!args.disable_proxy,
    };

    let fd = match args.systemd {
	true	=> listenfd::ListenFd::from_env()
	    .take_raw_fd(0)
	    .unwrap()
	    .map(|fd| unsafe { OwnedFd::from_raw_fd(fd) }),
	false	=> None
    };

    let info = match fd {
	None		=> Either::A(SocketAddr::new(args.listen, args.port)),
	Some(fd)	=> Either::B(fd),
    };

    tokio_main(env, info).unwrap();
}
