use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Nix(#[from] nix::Error),

    #[error(transparent)]
    HttpError(#[from] reqwest::Error),

    // helper because reqwest::Error does not implement Clone
    #[error("http error: {0}")]
    HttpErrorStr(String),

    #[error("request failed with status {0}")]
    HttpStatus(reqwest::StatusCode),

    #[error("bad http time")]
    BadHttpTime,

    #[error("string conversion error")]
    StringConversion,

    #[error("internal error: {0}")]
    Internal(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Self::Io(e) => Self::Io(e.kind().into()),
            Self::Nix(arg0) => Self::Nix(*arg0),
            Self::HttpError(arg0) => Self::HttpErrorStr(format!("{}", arg0)),
            Self::HttpErrorStr(arg0) => Self::HttpErrorStr(arg0.clone()),
	    Self::HttpStatus(s) => Self::HttpStatus(*s),
            Self::BadHttpTime => Self::BadHttpTime,
            Self::StringConversion => Self::StringConversion,
            Self::Internal(arg0) => Self::Internal(arg0),
	}
    }
}
