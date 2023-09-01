## --*- conf-toml -*--
m4_divert(-1)
m4_changequote(`[[', `]]')	# '
m4_changequote([[``]],[['']])
m4_divert(0)

[package]
name = "r-tftpd"
version = "0.2.7"
authors = ["Enrico Scholz <enrico.scholz@sigma-chemnitz.de>"]
edition = "2021"
description = "TFTP server with RFC 7440 windowsize support"
license = "GPL-3.0-or-later"
repository = "https://gitlab-ext.sigma-chemnitz.de/ensc/r-tftpd"
keywords = ["tftp", "rfc7440", "tftp-server"]

[workspace]
members = [
	"mod-proxy",
]

[features]
default = [ m4_ifdef(``CARGO_DEFAULT_FEATURES'',``CARGO_DEFAULT_FEATURES'') ]

proxy = [ "r-tftpd-proxy" ]

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
num-format = { version = "*", features = ["with-system-locale"] }

r-tftpd-proxy = { version = "*", path = "mod-proxy", optional = true }

[dependencies.clap]
version = m4_ifdef(``RUST166_COMPAT'',``"<4.4"'',``"*"'')
features = ["derive", "color", "std"]

[dependencies.clap_lex]
version = m4_ifdef(``RUST166_COMPAT'',``"<0.5.1"'',``"*"'')

[dev-dependencies]
rand = { version = "*", features = ["min_const_gen"] }
tokio = { version = "1", default-features = false, features = ["rt", "time", "net", "macros", "process"] }
tempfile = { version = "*" }

[profile.release]
lto = true
codegen-units = 1
