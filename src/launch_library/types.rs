use bevy::prelude::*;
use chrono::{DateTime, Duration, Utc};
use std::sync::{
    Arc, Mutex,
    mpsc::{Receiver, Sender},
};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct LaunchSummary {
    pub id: Option<i64>,
    pub name: String,
    pub net_utc: Option<DateTime<Utc>>,
    pub pad_id: Option<i64>,
    pub pad_name: Option<String>,
    pub pad_lat: Option<f64>,
    pub pad_lon: Option<f64>,
    pub pad_location_name: Option<String>,
    pub provider_name: Option<String>,
    pub mission_name: Option<String>,
    pub orbit_name: Option<String>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct EventSummary {
    pub id: Option<i64>,
    pub name: String,
    pub date_utc: Option<DateTime<Utc>>,
    pub location: Option<String>,
    pub type_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchLibraryFeed {
    Launches,
    Events,
}

#[derive(Resource, Debug)]
pub struct LaunchLibraryConfig {
    pub base_url: String,
    pub limit: usize,
    pub window_days: i64,
    pub refresh_interval: Duration,
    pub show_pad_markers: bool,
}

impl Default for LaunchLibraryConfig {
    fn default() -> Self {
        Self {
            base_url: "https://ll.thespacedevs.com/2.3.0".to_string(),
            limit: 10,
            window_days: 30,
            refresh_interval: Duration::minutes(30),
            show_pad_markers: true,
        }
    }
}

#[derive(Resource, Debug, Default)]
pub struct LaunchLibraryState {
    pub last_launch_request: Option<DateTime<Utc>>,
    pub last_event_request: Option<DateTime<Utc>>,
    pub last_launch_update: Option<DateTime<Utc>>,
    pub last_event_update: Option<DateTime<Utc>>,
    pub is_loading_launches: bool,
    pub is_loading_events: bool,
    pub launch_error: Option<String>,
    pub event_error: Option<String>,
    pub force_refresh: bool,
}

#[derive(Resource, Debug, Default)]
pub struct LaunchLibraryData {
    pub launches: Vec<LaunchSummary>,
    pub events: Vec<EventSummary>,
}

#[derive(Resource)]
pub struct LaunchLibraryChannels {
    pub cmd_tx: Sender<LaunchLibraryCommand>,
    pub res_rx: Arc<Mutex<Receiver<LaunchLibraryResult>>>,
}

pub enum LaunchLibraryCommand {
    FetchLaunches { url: String },
    FetchEvents { url: String },
}

pub enum LaunchLibraryResult {
    Launches(Vec<LaunchSummary>),
    Events(Vec<EventSummary>),
    Error { feed: LaunchLibraryFeed, error: String },
}
