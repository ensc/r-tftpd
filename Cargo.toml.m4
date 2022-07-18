## --*- conf-toml -*--

[package]
name = "r-tftpd"
version = "0.0.1"
authors = ["Enrico Scholz <enrico.scholz@sigma-chemnitz.de>"]
edition = "2021"

[workspace]

[dependencies]
tokio = { version = "1", default-features = false, features = ["rt", "time", "net", "macros"] }
reqwest = { version = "*", default-features = false, features = ["default-tls", "gzip", "brotli", "deflate", "stream", "socks"] }
thiserror = "*"
lazy_static = "*"
regex = "*"
http = "*"
tracing = "*"
tracing-subscriber = "*"
listenfd = "*"
libc = "*"
nix = { version = "*", default-features = false, features = ["socket", "uio", "net", "socket"] }
#systemd = { version = "*", default-features = false, features = [] }
clap = { version = "*", features = ["derive", "color", "std"] }
num-format = { version = "*", features = ["with-system-locale"] }

[dev-dependencies]
tempdir = "*"

[profile.release]
lto = true
codegen-units = 1
