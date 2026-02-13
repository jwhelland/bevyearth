//! Launch Library fetcher worker.

use crate::launch_library::types::{
    EventSummary, LaunchLibraryCommand, LaunchLibraryFeed, LaunchLibraryResult, LaunchSummary,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::sync::{
    Arc, Mutex,
    mpsc::{self},
};
use std::thread;

pub fn start_launch_library_worker() -> crate::launch_library::types::LaunchLibraryChannels {
    let (cmd_tx, cmd_rx) = mpsc::channel::<LaunchLibraryCommand>();
    let (res_tx, res_rx) = mpsc::channel::<LaunchLibraryResult>();

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let client = reqwest::Client::new();

            while let Ok(cmd) = cmd_rx.recv() {
                let (feed, result) = match cmd {
                    LaunchLibraryCommand::FetchLaunches { url } => {
                        let res = fetch_launches(&client, &url)
                            .await
                            .map(LaunchLibraryResult::Launches);
                        (LaunchLibraryFeed::Launches, res)
                    }
                    LaunchLibraryCommand::FetchEvents { url } => {
                        let res = fetch_events(&client, &url)
                            .await
                            .map(LaunchLibraryResult::Events);
                        (LaunchLibraryFeed::Events, res)
                    }
                };

                let send = |msg| {
                    let _ = res_tx.send(msg);
                };

                match result {
                    Ok(msg) => send(msg),
                    Err(err) => {
                        eprintln!("[LAUNCH LIBRARY] {:?} fetch failed: {}", feed, err);
                        send(LaunchLibraryResult::Error {
                            feed,
                            error: err.to_string(),
                        })
                    }
                }
            }
        });
    });

    crate::launch_library::types::LaunchLibraryChannels {
        cmd_tx,
        res_rx: Arc::new(Mutex::new(res_rx)),
    }
}

async fn fetch_launches(client: &reqwest::Client, url: &str) -> Result<Vec<LaunchSummary>> {
    let body = fetch_body(client, url).await?;
    parse_launches(&body)
}

async fn fetch_events(client: &reqwest::Client, url: &str) -> Result<Vec<EventSummary>> {
    let body = fetch_body(client, url).await?;
    parse_events(&body)
}

async fn fetch_body(client: &reqwest::Client, url: &str) -> Result<String> {
    let resp = client.get(url).send().await?;
    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        anyhow::bail!("HTTP {} for {}", status, url);
    }
    Ok(body)
}

fn parse_launches(body: &str) -> Result<Vec<LaunchSummary>> {
    let value: Value = serde_json::from_str(body)?;
    let items = extract_items(&value);
    let mut launches = Vec::with_capacity(items.len());

    for item in items {
        let name = get_string(item, "name").unwrap_or_else(|| "Unnamed Launch".to_string());
        let net_utc = get_string_ref(item, "net").and_then(parse_datetime);
        let id = get_i64(item, "id");

        let pad = item.get("pad");
        let pad_id = pad.and_then(|p| get_i64(p, "id"));
        let pad_name = pad.and_then(|p| get_string(p, "name"));
        let pad_lat = pad.and_then(|p| get_f64(p, "latitude"));
        let pad_lon = pad.and_then(|p| get_f64(p, "longitude"));
        let pad_location_name = pad.and_then(|p| p.get("location")).and_then(extract_name);
        let launch_location_name = item.get("location").and_then(extract_name);

        let provider_name = item
            .get("launch_service_provider")
            .and_then(|p| get_string(p, "name"))
            .or_else(|| {
                item.get("service_provider")
                    .and_then(|p| get_string(p, "name"))
            });

        let mission = item.get("mission");
        let mission_name = mission.and_then(|m| get_string(m, "name"));
        let orbit_name = mission
            .and_then(|m| m.get("orbit"))
            .and_then(|o| get_string(o, "name"))
            .or_else(|| item.get("orbit").and_then(|o| get_string(o, "name")));

        launches.push(LaunchSummary {
            id,
            name,
            net_utc,
            pad_id,
            pad_name,
            pad_lat,
            pad_lon,
            pad_location_name: pad_location_name.or(launch_location_name),
            provider_name,
            mission_name,
            orbit_name,
        });
    }

    Ok(launches)
}

fn parse_events(body: &str) -> Result<Vec<EventSummary>> {
    let value: Value = serde_json::from_str(body)?;
    let items = extract_items(&value);
    let mut events = Vec::with_capacity(items.len());

    for item in items {
        let name = get_string(item, "name").unwrap_or_else(|| "Unnamed Event".to_string());
        let date_utc = get_string_ref(item, "date")
            .or_else(|| get_string_ref(item, "net"))
            .and_then(parse_datetime);
        let id = get_i64(item, "id");
        let location = item.get("location").and_then(extract_name);
        let type_name = item.get("type").and_then(extract_name);
        let description = get_string(item, "description");

        events.push(EventSummary {
            id,
            name,
            date_utc,
            location,
            type_name,
            description,
        });
    }

    Ok(events)
}

fn extract_items(value: &Value) -> Vec<&Value> {
    if let Some(array) = value.as_array() {
        return array.iter().collect();
    }
    value
        .get("results")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().collect())
        .unwrap_or_default()
}

fn get_string(value: &Value, key: &str) -> Option<String> {
    match value.get(key) {
        Some(Value::String(val)) => Some(val.to_string()),
        Some(other) => other.as_str().map(|s| s.to_string()),
        None => None,
    }
}

fn get_string_ref<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(|v| v.as_str())
}

fn get_i64(value: &Value, key: &str) -> Option<i64> {
    match value.get(key) {
        Some(Value::Number(num)) => num.as_i64(),
        Some(Value::String(val)) => val.parse::<i64>().ok(),
        _ => None,
    }
}

fn get_f64(value: &Value, key: &str) -> Option<f64> {
    match value.get(key) {
        Some(Value::Number(num)) => num.as_f64(),
        Some(Value::String(val)) => val.parse::<f64>().ok(),
        _ => None,
    }
}

fn extract_name(value: &Value) -> Option<String> {
    match value {
        Value::String(val) => Some(val.to_string()),
        Value::Object(map) => map
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

fn parse_datetime(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}
