#![warn(missing_docs)]

//! The fsidx crate scans file system folders to store pathnames and optionally file sizes in database files. For these database files efficient search queries are implemented to locate files.

mod config;
mod filter;
mod find;
mod locate;
mod update;

pub use config::VolumeInfo;
pub use config::{LocateConfig, Mode, Order, Settings, What};
pub use filter::FilterToken;
pub use locate::{locate, LocateError, LocateEvent, Metadata};
pub use update::{update, UpdateSink};
