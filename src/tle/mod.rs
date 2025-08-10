//! TLE (Two-Line Element) data management module
//! 
//! This module handles TLE fetching, parsing, and data structures for satellite
//! orbital elements from external sources like Celestrak.

pub mod fetcher;
pub mod parser;
pub mod systems;
pub mod types;

pub use types::{TleData, FetchCommand, FetchResultMsg, FetchChannels};
pub use fetcher::start_tle_worker;
pub use parser::parse_tle_epoch_to_utc;
pub use systems::process_fetch_results_system;