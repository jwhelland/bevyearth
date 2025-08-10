//! TLE data types and communication structures

use bevy::prelude::*;
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex, mpsc::{Receiver, Sender}};

/// TLE data structure
#[derive(Clone)]
pub struct TleData {
    pub name: Option<String>,
    pub line1: String,
    pub line2: String,
    pub epoch_utc: DateTime<Utc>,
}

/// Commands for the TLE fetcher worker thread
#[derive(Debug)]
pub enum FetchCommand {
    Fetch(u32),
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
    },
    Failure { 
        norad: u32, 
        error: String 
    },
}

/// Resource containing channels for communicating with the TLE worker thread
#[derive(Resource)]
pub struct FetchChannels {
    pub cmd_tx: Sender<FetchCommand>,
    pub res_rx: Arc<Mutex<Receiver<FetchResultMsg>>>,
}