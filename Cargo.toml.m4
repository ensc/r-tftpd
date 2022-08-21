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

[workspace]

[features]
default = [ m4_ifdef(``CARGO_DEFAULT_FEATURES'',``CARGO_DEFAULT_FEATURES'') ]

proxy = [ "dep:reqwest" ]

[dependencies]
tokio = { version = "1", default-features = false, features = ["rt", "time", "net", "macros"] }
thiserror = "*"
lazy_static = "*"
regex = "*"
http = "*"
tracing = "*"
tracing-subscriber = { version = "*", features = ["json"] }
listenfd = "*"
nix = { version = "*", default-features = false, features = ["socket", "uio", "net", "socket"] }
#systemd = { version = "*", default-features = false, features = [] }
clap = { version = "*", features = ["derive", "color", "std"] }
num-format = { version = "*", features = ["with-system-locale"] }

[dependencies.reqwest]
version = "*"
optional = true
default-features = false
features = ["default-tls", "gzip", "brotli", "deflate", "stream", "socks"]

[dev-dependencies]
rand = { version = "0.8.*", features = ["min_const_gen"] }
tempfile = "*"
tokio = { version = "1", default-features = false, features = ["rt", "time", "net", "macros", "process"] }

[profile.release]
lto = true
codegen-units = 1
