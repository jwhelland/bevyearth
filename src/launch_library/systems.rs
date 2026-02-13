//! Launch Library systems (polling + apply).

use crate::launch_library::fetcher::start_launch_library_worker;
use crate::launch_library::types::{
    LaunchLibraryChannels, LaunchLibraryCommand, LaunchLibraryConfig, LaunchLibraryData,
    LaunchLibraryFeed, LaunchLibraryResult, LaunchLibraryState,
};
use bevy::prelude::*;
use chrono::{DateTime, Duration, Utc};

pub fn setup_launch_library_worker(mut commands: Commands) {
    let channels = start_launch_library_worker();
    println!("[INIT] Launch Library worker started");
    commands.insert_resource(channels);
}

pub fn poll_launch_library(
    config: Res<LaunchLibraryConfig>,
    mut state: ResMut<LaunchLibraryState>,
    channels: Option<Res<LaunchLibraryChannels>>,
) {
    let Some(channels) = channels else { return };

    let now = Utc::now();
    let should_force = state.force_refresh;
    let should_fetch_launches = should_force
        || state
            .last_launch_request
            .map(|t| now.signed_duration_since(t) >= config.refresh_interval)
            .unwrap_or(true);
    let should_fetch_events = should_force
        || state
            .last_event_request
            .map(|t| now.signed_duration_since(t) >= config.refresh_interval)
            .unwrap_or(true);

    if should_fetch_launches {
        let url = build_launches_url(&config, now);
        if let Err(err) = channels
            .cmd_tx
            .send(LaunchLibraryCommand::FetchLaunches { url })
        {
            state.launch_error = Some(format!("Failed to queue launches fetch: {}", err));
            state.is_loading_launches = false;
        } else {
            state.last_launch_request = Some(now);
            state.is_loading_launches = true;
            state.launch_error = None;
        }
    }

    if should_fetch_events {
        let url = build_events_url(&config, now);
        if let Err(err) = channels
            .cmd_tx
            .send(LaunchLibraryCommand::FetchEvents { url })
        {
            state.event_error = Some(format!("Failed to queue events fetch: {}", err));
            state.is_loading_events = false;
        } else {
            state.last_event_request = Some(now);
            state.is_loading_events = true;
            state.event_error = None;
        }
    }

    if state.force_refresh {
        state.force_refresh = false;
    }
}

pub fn apply_launch_library_results(
    mut data: ResMut<LaunchLibraryData>,
    mut state: ResMut<LaunchLibraryState>,
    channels: Option<Res<LaunchLibraryChannels>>,
) {
    let Some(channels) = channels else { return };
    let Ok(guard) = channels.res_rx.lock() else {
        return;
    };

    while let Ok(msg) = guard.try_recv() {
        match msg {
            LaunchLibraryResult::Launches(launches) => {
                data.launches = launches;
                state.last_launch_update = Some(Utc::now());
                state.is_loading_launches = false;
                state.launch_error = None;
            }
            LaunchLibraryResult::Events(events) => {
                data.events = events;
                state.last_event_update = Some(Utc::now());
                state.is_loading_events = false;
                state.event_error = None;
            }
            LaunchLibraryResult::Error { feed, error } => match feed {
                LaunchLibraryFeed::Launches => {
                    state.launch_error = Some(error);
                    state.is_loading_launches = false;
                }
                LaunchLibraryFeed::Events => {
                    state.event_error = Some(error);
                    state.is_loading_events = false;
                }
            },
        }
    }
}

fn build_launches_url(config: &LaunchLibraryConfig, now: DateTime<Utc>) -> String {
    let mut url =
        reqwest::Url::parse(&format!("{}/launches/", config.base_url)).expect("launches url");
    let end = now + Duration::days(config.window_days);
    url.query_pairs_mut()
        .append_pair("net__gte", &now.to_rfc3339())
        .append_pair("net__lte", &end.to_rfc3339())
        .append_pair("ordering", "net")
        .append_pair("limit", &config.limit.to_string())
        .append_pair("mode", "detailed");
    url.to_string()
}

fn build_events_url(config: &LaunchLibraryConfig, now: DateTime<Utc>) -> String {
    let mut url = reqwest::Url::parse(&format!("{}/events/", config.base_url)).expect("events url");
    let end = now + Duration::days(config.window_days);
    url.query_pairs_mut()
        .append_pair("date__gte", &now.to_rfc3339())
        .append_pair("date__lte", &end.to_rfc3339())
        .append_pair("ordering", "date")
        .append_pair("limit", &config.limit.to_string())
        .append_pair("mode", "list");
    url.to_string()
}
