//! TLE data types and communication structures

use bevy::prelude::*;
use chrono::{DateTime, Utc};
use std::sync::{
    Arc, Mutex,
    mpsc::{Receiver, Sender},
};

/// TLE data structure
#[derive(Clone)]
pub struct TleData {
    pub epoch_utc: DateTime<Utc>,
}

/// Commands for the TLE fetcher worker thread
#[derive(Debug)]
pub enum FetchCommand {
    /// Fetch a single satellite by NORAD ID
    Fetch(u32),
    /// Fetch all satellites in a Celestrak group (e.g., "weather")
    FetchGroup { group: String },
}

/// Results from the TLE fetcher worker thread
#[derive(Debug)]
pub enum FetchResultMsg {
    Success {
        norad: u32,
        name: Option<String>,
        line1: String,
        line2: String,
        epoch_utc: DateTime<Utc>,
        group: Option<String>,
    },
    Failure {
        norad: u32,
        error: String,
    },
    GroupDone {
        group: String,
        count: usize,
    },
    GroupFailure {
        group: String,
        error: String,
    },
}

/// Resource containing channels for communicating with the TLE worker thread
#[derive(Resource)]
pub struct FetchChannels {
    pub cmd_tx: Sender<FetchCommand>,
    pub res_rx: Arc<Mutex<Receiver<FetchResultMsg>>>,
}

/// Configuration for TLE disk caching
#[derive(Resource, Clone)]
pub struct TleCacheConfig {
    /// Whether disk caching is enabled
    pub enabled: bool,
    /// Number of days before a cached TLE is considered expired
    pub expiration_days: i64,
    /// Enable verbose logging of cache hits/misses
    pub verbose_logging: bool,
}

impl Default for TleCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            expiration_days: 7,
            verbose_logging: false,
        }
    }
}
