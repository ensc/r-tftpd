use super::{ RequestError as E, RequestResult };

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    NetAscii,
    Octet,
    Mail,
}

impl TryFrom<&[u8]> for Mode {
    type Error = E;

    fn try_from(s: &[u8]) -> RequestResult<Self> {
	use crate::util::ToLower;

        match s.to_lower().as_slice() {
	    b"netascii"	=> Ok(Self::NetAscii),
	    b"octet"	=> Ok(Self::Octet),
	    b"binary"	=> Ok(Self::Octet), // legacy name
	    b"mail"	=> Ok(Self::Mail),
	    _m		=> Err(E::BadMode),
	}
    }
}
