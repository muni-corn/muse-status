use std::fmt::{Display, Formatter};
use std::fmt;
use std::num::ParseIntError;
use std::io::Error as IoError;
use crate::format::color::RGBAParseError;

/// Wraps a number of errors that could be encountered throughout muse-status.
#[derive(Debug)]
pub enum MuseStatusError {
    /// Wraps a BasicError.
    Basic(BasicError),

    /// Wraps an UpdateError.
    Update(UpdateError),

    /// Wraps a std::IoError.
    Io(IoError),

    /// Wraps an error from parsing an integer.
    ParseInt(ParseIntError),

    /// Wraps an error from network requests.
    Reqwest(reqwest::Error),

    /// Wraps an error from parsing colors.
    RGBAParse(RGBAParseError),
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

impl From<RGBAParseError> for MuseStatusError {
    fn from(e: RGBAParseError) -> Self {
        Self::RGBAParse(e)
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
            Self::RGBAParse(r) => r.fmt(f),
        }
    }
}

/// A simple error with a single message.
#[derive(Debug)]
pub struct BasicError {
    /// The message to this error.
    pub message: String
}

impl Display for BasicError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "muse-status encountered an error: {}", self.message)
    }
}

/// UpdateError is returned when a Block fails to update for any reason. The block name and message
/// are given to give the user (or developer) information about where and why an error happened
/// while updating a Block.
#[derive(Debug)]
pub struct UpdateError {
    /// The block name, to be used in the formatted output of the error.
    pub block_name: String,

    /// A message that gives specifics for why this error happened.
    pub message: String,
}

impl Display for UpdateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "couldn't update {}: {}", self.block_name, self.message)
    }
}
