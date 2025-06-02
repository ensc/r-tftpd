## --*- conf-toml -*--
m4_divert(-1)
m4_changequote(`[[', `]]')	# '
m4_changequote([[``]],[['']])
m4_divert(0)

[package]
name = "r-tftpd"
version = "0.3.5"
authors = ["Enrico Scholz <enrico.scholz@sigma-chemnitz.de>"]
edition = "2021"
description = "TFTP server with RFC 7440 windowsize support"
license = "GPL-3.0-or-later"
homepage = "https://github.com/ensc/r-tftpd"
repository = "https://gitlab-ext.sigma-chemnitz.de/ensc/r-tftpd"
keywords = ["tftp", "rfc7440", "tftp-server"]

[workspace]
members = [
	"mod-proxy",
]

[features]
default = [ m4_ifdef(``CARGO_DEFAULT_FEATURES'',``CARGO_DEFAULT_FEATURES'') ]

proxy = [ "r-tftpd-proxy" ]
legacy_rust_179 = []

[dependencies]
tokio = { version = "1", default-features = false, features = ["rt", "time", "net", "macros", "signal"] }
thiserror = "*"
lazy_static = "*"
regex = "*"
url = "*"
tracing = "*"
tracing-subscriber = { version = "*", features = ["json", "env-filter"] }
listenfd = "*"
nix = { version = "*", default-features = false, features = ["socket", "uio", "net", "socket"] }
#systemd = { version = "*", default-features = false, features = [] }
num-format = { version = "*", features = ["with-system-locale"] }

r-tftpd-proxy = { version = "*", path = "mod-proxy", optional = true }

[dependencies.clap]
version = "*"
features = ["derive", "color", "std"]

[dev-dependencies]
rand = { version = "*", features = ["min_const_gen"] }
tokio = { version = "1", default-features = false, features = ["rt", "time", "net", "macros", "process"] }
tempfile = { version = "*" }

[profile.release]
lto = true
codegen-units = 1
