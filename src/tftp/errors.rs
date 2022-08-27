#[derive(thiserror::Error, Debug, Eq, PartialEq, Clone)]
pub enum RequestError {
    #[error("datagram too short")]
    TooShort,
    #[error("bad op code ({})", {0})]
    BadOpCode(u16),
    #[error("filename missing")]
    MissingFilename,
    #[error("mode missing")]
    MissingMode,
    #[error("unsupported mode")]
    BadMode,
    #[error("not a digit ({})", {0})]
    BadDigit(u8),
    #[error("number out of range")]
    NumberOutOfRange,
    #[error("missing argument")]
    MissingArgument,
    #[error("datagram without trailing zero")]
    MissingZero,
    #[error("write operation not implemented")]
    WriteUnsupported,
    #[error("operation not supported")]
    OperationUnsupported,
    #[error("transfer mode not supported; only 'octet' is supported")]
    ModeUnsupported,
    #[error("malformed ACK")]
    MalformedAck,
}

pub type RequestResult<T> = std::result::Result<T, RequestError>;
