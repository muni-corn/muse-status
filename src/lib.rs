//! muse-status needs documentation

#![warn(missing_docs)]
#![deny(unused_variables)]
#![deny(clippy::shadow_unrelated)]

/// The battery block module.
pub mod battery;

/// The brightness block module.
pub mod brightness;

/// The daemon module.
pub mod daemon;

/// The date block module.
pub mod date;

/// The errors module.
pub mod errors;

/// The formatting module.
pub mod format;

/// The network block module.
pub mod network;

/// The mpris block module.
pub mod mpris;

/// The volume block module.
pub mod volume;

/// The utils module.
pub mod utils;

/// The weather block module.
pub mod weather;
