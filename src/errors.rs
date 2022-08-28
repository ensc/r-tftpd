use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Nix(#[from] nix::Error),

    #[cfg(feature = "proxy")]
    #[error(transparent)]
    Proxy(#[from] r_tftpd_proxy::Error),

    #[error(transparent)]
    RequestError(#[from] crate::tftp::RequestError),

    #[error("invalid pathname")]
    InvalidPathName,

    #[error("string conversion error")]
    StringConversion,

    #[error("failed to parse uri")]
    UriParse,

    #[error("file is missing")]
    FileMissing,

    #[error("internal error: {0}")]
    Internal(&'static str),

    #[error("timeout")]
    Timeout,

    #[error("bad ack package")]
    BadAck,

    #[error("generic protocol error: {0}")]
    Protocol(&'static str),

    #[error("operation not implemented")]
    NotImplemented,

    #[error("too much clients")]
    TooMuchClients,
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Self::Io(e) => Self::Io(e.kind().into()),
            Self::Nix(arg0) => Self::Nix(*arg0),
            Self::RequestError(arg0) => Self::RequestError(arg0.clone()),
            Self::InvalidPathName => Self::InvalidPathName,
            Self::UriParse => Self::UriParse,
            Self::FileMissing => Self::FileMissing,
            Self::Internal(arg0) => Self::Internal(arg0),
            Self::Timeout => Self::Timeout,
            Self::BadAck => Self::BadAck,
            Self::Protocol(arg0) => Self::Protocol(arg0),
            Self::NotImplemented => Self::NotImplemented,
            Self::TooMuchClients => Self::TooMuchClients,
            Self::StringConversion => Self::StringConversion,

	    #[cfg(feature = "proxy")]
            Self::Proxy(arg0) => Self::Proxy(arg0.clone()),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
