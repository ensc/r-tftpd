impl From<httpdate::Error> for crate::Error {
    fn from(_: httpdate::Error) -> Self {
        Self::BadHttpTime
    }
}
