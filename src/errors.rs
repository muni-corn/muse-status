use std::fmt::{Display, Formatter};
use std::fmt;
use std::num::ParseIntError;
use std::io::Error as IoError;

#[derive(Debug)]
pub enum MuseStatusError {
    Basic(BasicError),
    Update(UpdateError),
    Io(IoError),
    ParseInt(ParseIntError),
    Reqwest(reqwest::Error),
}

impl From<ParseIntError> for MuseStatusError {
    fn from(e: ParseIntError) -> Self {
        Self::ParseInt(e)
    }
}

impl From<IoError> for MuseStatusError {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl From<BasicError> for MuseStatusError {
    fn from(e: BasicError) -> Self {
        Self::Basic(e)
    }
}

impl From<UpdateError> for MuseStatusError {
    fn from(e: UpdateError) -> Self {
        Self::Update(e)
    }
}

impl From<reqwest::Error> for MuseStatusError {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
    }
}

impl Display for MuseStatusError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Basic(b) => b.fmt(f),
            Self::Update(u) => u.fmt(f),
            Self::Io(i) => i.fmt(f),
            Self::ParseInt(p) => p.fmt(f),
            Self::Reqwest(r) => r.fmt(f),
        }
    }
}

#[derive(Debug)]
pub struct BasicError {
    pub message: String
}

impl Display for BasicError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "muse-status encountered an error: {}", self.message)
    }
}

#[derive(Debug)]
pub struct UpdateError {
    pub block_name: String,
    pub message: String,
}

impl Display for UpdateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "couldn't update {}: {}", self.block_name, self.message)
    }
}
