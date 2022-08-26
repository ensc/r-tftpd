## --*- conf-toml -*--
m4_divert(-1)
m4_changequote(`[[', `]]')	# '
m4_changequote([[``]],[['']])
m4_divert(0)

[package]
name = "r-tftpd"
version = "0.0.1"
authors = ["Enrico Scholz <enrico.scholz@sigma-chemnitz.de>"]
edition = "2021"
description = "TFTP server with RFC 7440 windowsize support"
license = "GPL-3.0-or-later"
repository = "https://gitlab-ext.sigma-chemnitz.de/ensc/r-tftpd"
keywords = ["tftp", "rfc7440", "tftp-server"]

[workspace]

[features]
default = [ m4_ifdef(``CARGO_DEFAULT_FEATURES'',``CARGO_DEFAULT_FEATURES'') ]

proxy = [ "reqwest", "tempfile", "httpdate", "futures-core", "bytes", "bitflags" ]

[dependencies]
tokio = { version = "1", default-features = false, features = ["rt", "time", "net", "macros"] }
thiserror = "*"
lazy_static = "*"
regex = "*"
url = "*"
tracing = "*"
tracing-subscriber = { version = "*", features = ["json", "env-filter"] }
listenfd = "*"
nix = { version = "*", default-features = false, features = ["socket", "uio", "net", "socket"] }
#systemd = { version = "*", default-features = false, features = [] }
clap = { version = "*", features = ["derive", "color", "std"] }
num-format = { version = "*", features = ["with-system-locale"] }

tempfile = { version = "*", optional = true }
httpdate = { version = "*", optional = true }
futures-core = { version = "*", optional = true }
bytes = { version = "*", optional = true }
bitflags = { version = "*", optional = true }

[dependencies.reqwest]
version = "*"
optional = true
default-features = false
features = ["default-tls", "gzip", "brotli", "deflate", "socks"]

[dev-dependencies]
rand = { version = "*", features = ["min_const_gen"] }
tokio = { version = "1", default-features = false, features = ["rt", "time", "net", "macros", "process"] }
tempfile = { version = "*" }

[profile.release]
lto = true
codegen-units = 1
