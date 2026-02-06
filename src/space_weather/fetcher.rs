//! Space weather fetcher worker.

use crate::space_weather::types::{
    AuroraGrid, AuroraPoint, KpIndex, SpaceWeatherChannels, SpaceWeatherCommand, SpaceWeatherFeed,
    SpaceWeatherResult,
};
use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::sync::{
    Arc, Mutex,
    mpsc::{self},
};
use std::thread;

const OVATION_URL: &str = "https://services.swpc.noaa.gov/json/ovation_aurora_latest.json";
const KP_URL: &str = "https://services.swpc.noaa.gov/products/noaa-planetary-k-index.json";
const MAG_URL: &str = "https://services.swpc.noaa.gov/products/solar-wind/mag-1-day.json";
const PLASMA_URL: &str =
    "https://services.swpc.noaa.gov/products/solar-wind/plasma-1-day.json";

struct JsonTable {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

pub fn start_space_weather_worker() -> SpaceWeatherChannels {
    let (cmd_tx, cmd_rx) = mpsc::channel::<SpaceWeatherCommand>();
    let (res_tx, res_rx) = mpsc::channel::<SpaceWeatherResult>();

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let client = reqwest::Client::new();

            while let Ok(cmd) = cmd_rx.recv() {
                let (feed, res) = match cmd {
                    SpaceWeatherCommand::FetchOvation => (
                        SpaceWeatherFeed::Ovation,
                        fetch_ovation(&client)
                            .await
                            .map(|grid| SpaceWeatherResult::Ovation { grid }),
                    ),
                    SpaceWeatherCommand::FetchKp => (
                        SpaceWeatherFeed::Kp,
                        fetch_kp(&client).await.map(|kp| SpaceWeatherResult::Kp { kp }),
                    ),
                    SpaceWeatherCommand::FetchMag => (
                        SpaceWeatherFeed::Mag,
                        fetch_mag(&client)
                            .await
                            .map(|(bt, bz, timestamp)| SpaceWeatherResult::Mag {
                                bt,
                                bz,
                                timestamp,
                            }),
                    ),
                    SpaceWeatherCommand::FetchPlasma => (
                        SpaceWeatherFeed::Plasma,
                        fetch_plasma(&client)
                            .await
                            .map(|(speed, density, timestamp)| {
                                SpaceWeatherResult::Plasma {
                                    speed,
                                    density,
                                    timestamp,
                                }
                            }),
                    ),
                };

                let send = |msg| {
                    let _ = res_tx.send(msg);
                };

                match res {
                    Ok(msg) => send(msg),
                    Err(err) => {
                        send(SpaceWeatherResult::Error {
                            feed,
                            error: err.to_string(),
                        });
                    }
                }
            }
        });
    });

    SpaceWeatherChannels {
        cmd_tx,
        res_rx: Arc::new(Mutex::new(res_rx)),
    }
}

async fn fetch_ovation(client: &reqwest::Client) -> Result<AuroraGrid> {
    let body = fetch_body(client, OVATION_URL).await?;
    if let Ok(grid) = parse_ovation_object(&body) {
        return Ok(grid);
    }

    let table = parse_json_table(&body)?;
    let mut lat_idx = find_column(
        &table.header,
        &[
            "lat",
            "latitude",
            "lat_deg",
            "latitude_deg",
            "geomagnetic_latitude",
            "magnetic_latitude",
            "mlat",
        ],
    );
    let mut lon_idx = find_column(
        &table.header,
        &[
            "lon",
            "longitude",
            "lon_deg",
            "longitude_deg",
            "geomagnetic_longitude",
            "magnetic_longitude",
            "mlon",
        ],
    );
    let mut value_idx = find_column(
        &table.header,
        &["aurora", "probability", "intensity", "power", "value"],
    );
    let time_idx = find_column(&table.header, &["time_tag", "timestamp", "time"]);

    let mut rows = table.rows.clone();
    if lat_idx.is_none() || lon_idx.is_none() || value_idx.is_none() {
        if header_looks_numeric(&table.header) {
            rows.insert(0, table.header.clone());
        }
        if let Some((lat, lon, value)) = infer_ovation_columns(&rows) {
            lat_idx = Some(lat);
            lon_idx = Some(lon);
            value_idx = Some(value);
        }
    }

    let lat_idx = lat_idx.context("ovation: missing latitude column")?;
    let lon_idx = lon_idx.context("ovation: missing longitude column")?;
    let value_idx = value_idx.context("ovation: missing intensity column")?;

    let mut points = Vec::new();
    let mut max_value = 0.0_f32;

    for row in rows.iter() {
        let Some(lat) = parse_f32(get_cell(row, lat_idx)) else {
            continue;
        };
        let Some(lon) = parse_f32(get_cell(row, lon_idx)) else {
            continue;
        };
        let Some(value) = parse_f32(get_cell(row, value_idx)) else {
            continue;
        };
        if value > max_value {
            max_value = value;
        }
        points.push(AuroraPoint { lat, lon, value });
    }

    let updated_utc = latest_timestamp(&rows, time_idx).or_else(|| Some(Utc::now()));

    Ok(AuroraGrid {
        points,
        grid_values: Vec::new(),
        grid_width: 0,
        grid_height: 0,
        lon_min: 0.0,
        lat_min: 0.0,
        lon_step: 0.0,
        lat_step: 0.0,
        max_value,
        updated_utc,
    })
}

async fn fetch_kp(client: &reqwest::Client) -> Result<KpIndex> {
    let body = fetch_body(client, KP_URL).await?;
    let table = parse_json_table(&body)?;

    let kp_idx = find_column(&table.header, &["kp", "kp_index"])
        .context("kp: missing kp column")?;
    let time_idx = find_column(&table.header, &["time_tag", "timestamp", "time"]);

    let (value, timestamp) = latest_numeric_with_time(&table.rows, kp_idx, time_idx)
        .context("kp: no valid rows")?;

    Ok(KpIndex {
        value: Some(value),
        timestamp,
    })
}

async fn fetch_mag(client: &reqwest::Client) -> Result<(Option<f32>, Option<f32>, Option<DateTime<Utc>>)> {
    let body = fetch_body(client, MAG_URL).await?;
    let table = parse_json_table(&body)?;

    let bt_idx = find_column(&table.header, &["bt", "bt_gsm"]);
    let bz_idx = find_column(&table.header, &["bz_gsm", "bz", "bz_gse"]);
    let time_idx = find_column(&table.header, &["time_tag", "timestamp", "time"]);

    let bt = bt_idx.and_then(|idx| latest_numeric(&table.rows, idx));
    let bz = bz_idx.and_then(|idx| latest_numeric(&table.rows, idx));
    let timestamp = latest_timestamp(&table.rows, time_idx);

    if bt.is_none() && bz.is_none() {
        anyhow::bail!("mag: missing bt/bz values");
    }

    Ok((bt, bz, timestamp))
}

async fn fetch_plasma(
    client: &reqwest::Client,
) -> Result<(Option<f32>, Option<f32>, Option<DateTime<Utc>>)> {
    let body = fetch_body(client, PLASMA_URL).await?;
    let table = parse_json_table(&body)?;

    let speed_idx = find_column(&table.header, &["speed", "proton_speed"]);
    let density_idx = find_column(&table.header, &["density", "proton_density"]);
    let time_idx = find_column(&table.header, &["time_tag", "timestamp", "time"]);

    let speed = speed_idx.and_then(|idx| latest_numeric(&table.rows, idx));
    let density = density_idx.and_then(|idx| latest_numeric(&table.rows, idx));
    let timestamp = latest_timestamp(&table.rows, time_idx);

    if speed.is_none() && density.is_none() {
        anyhow::bail!("plasma: missing speed/density values");
    }

    Ok((speed, density, timestamp))
}

async fn fetch_body(client: &reqwest::Client, url: &str) -> Result<String> {
    let resp = client
        .get(url)
        .header("accept", "application/json")
        .send()
        .await
        .context("request failed")?;
    let status = resp.status();
    let body = resp.text().await.context("read response")?;
    if !status.is_success() {
        anyhow::bail!("http {} for {}", status, url);
    }
    Ok(body)
}

fn parse_json_table(body: &str) -> Result<JsonTable> {
    let value: Value = serde_json::from_str(body).context("invalid json")?;
    match value {
        Value::Array(items) => parse_items_array(&items),
        Value::Object(obj) => {
            if let Some(message) = extract_error_message(&obj) {
                anyhow::bail!("{}", message);
            }
            if let Some(items) = extract_array_from_object(&obj) {
                return parse_items_array(items);
            }
            let mut keys: Vec<String> = obj.keys().cloned().collect();
            keys.sort();
            anyhow::bail!("expected json array (object keys: {})", keys.join(", "));
        }
        Value::String(text) => {
            let trimmed = text.trim();
            let snippet = if trimmed.len() > 120 {
                format!("{}...", &trimmed[..120])
            } else {
                trimmed.to_string()
            };
            anyhow::bail!("expected json array (string: {})", snippet);
        }
        _ => anyhow::bail!("expected json array"),
    }
}

fn parse_items_array(items: &[Value]) -> Result<JsonTable> {
    if items.is_empty() {
        anyhow::bail!("empty json table");
    }
    if let Some(first) = items.first() {
        if let Value::Array(_) = first {
            return parse_array_rows(items);
        }
        if let Value::Object(_) = first {
            return parse_object_rows(items);
        }
    }
    anyhow::bail!("unsupported table shape");
}

fn extract_error_message(obj: &serde_json::Map<String, Value>) -> Option<String> {
    for key in ["error", "message", "detail", "status_message", "title"] {
        if let Some(Value::String(val)) = obj.get(key) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn extract_array_from_object(obj: &serde_json::Map<String, Value>) -> Option<&[Value]> {
    for key in ["data", "values", "rows", "table", "records", "items"] {
        if let Some(Value::Array(items)) = obj.get(key) {
            return Some(items);
        }
    }
    let mut array_val: Option<&[Value]> = None;
    for value in obj.values() {
        if let Value::Array(items) = value {
            if array_val.is_some() {
                return None;
            }
            array_val = Some(items);
        }
    }
    array_val
}

fn parse_array_rows(items: &[Value]) -> Result<JsonTable> {
    let header_vals = items
        .first()
        .and_then(|row| row.as_array())
        .context("missing header row")?;
    let header: Vec<String> = header_vals
        .iter()
        .map(|v| value_to_string(v).unwrap_or_default())
        .collect();

    let mut rows = Vec::new();
    for row_val in items.iter().skip(1) {
        let Some(arr) = row_val.as_array() else { continue };
        let row: Vec<String> = arr
            .iter()
            .map(|v| value_to_string(v).unwrap_or_default())
            .collect();
        rows.push(row);
    }

    Ok(JsonTable { header, rows })
}

fn parse_object_rows(items: &[Value]) -> Result<JsonTable> {
    let Some(Value::Object(first)) = items.first() else {
        anyhow::bail!("missing object rows");
    };
    let mut header: Vec<String> = first.keys().cloned().collect();
    header.sort();

    let mut rows = Vec::new();
    for row_val in items.iter() {
        let Some(obj) = row_val.as_object() else { continue };
        let mut row = Vec::with_capacity(header.len());
        for key in header.iter() {
            let cell = obj.get(key).and_then(value_to_string).unwrap_or_default();
            row.push(cell);
        }
        rows.push(row);
    }

    Ok(JsonTable { header, rows })
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(val) => Some(val.clone()),
        Value::Number(num) => Some(num.to_string()),
        Value::Bool(val) => Some(val.to_string()),
        _ => None,
    }
}

fn normalize_key(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-', '_', '/'], "")
}

fn find_column(header: &[String], candidates: &[&str]) -> Option<usize> {
    let normalized: Vec<String> = header.iter().map(|h| normalize_key(h)).collect();
    for (idx, name) in normalized.iter().enumerate() {
        for candidate in candidates {
            let needle = normalize_key(candidate);
            if name == &needle || name.contains(&needle) {
                return Some(idx);
            }
        }
    }
    None
}

fn get_cell<'a>(row: &'a [String], idx: usize) -> Option<&'a str> {
    row.get(idx)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "null")
}

fn parse_f32(value: Option<&str>) -> Option<f32> {
    value?.parse::<f32>().ok()
}

fn collect_candidate_triples(
    value: &Value,
    lonlat: &mut Vec<(f32, f32, f32)>,
    latlon: &mut Vec<(f32, f32, f32)>,
) {
    match value {
        Value::Array(items) => {
            if items.len() >= 3 {
                if let (Some(a), Some(b), Some(c)) = (
                    items.get(0).and_then(|v| v.as_f64()),
                    items.get(1).and_then(|v| v.as_f64()),
                    items.get(2).and_then(|v| v.as_f64()),
                ) {
                    let a = a as f32;
                    let b = b as f32;
                    let c = c as f32;
                    if is_lon(a) && is_lat(b) {
                        lonlat.push((a, b, c));
                    }
                    if is_lat(a) && is_lon(b) {
                        latlon.push((a, b, c));
                    }
                    return;
                }
            }
            for item in items {
                collect_candidate_triples(item, lonlat, latlon);
            }
        }
        Value::Object(map) => {
            for val in map.values() {
                collect_candidate_triples(val, lonlat, latlon);
            }
        }
        _ => {}
    }
}

fn is_lat(value: f32) -> bool {
    (-90.0..=90.0).contains(&value)
}

fn is_lon(value: f32) -> bool {
    (-180.0..=360.0).contains(&value)
}

fn parse_ovation_object(body: &str) -> Result<AuroraGrid> {
    let value: Value = serde_json::from_str(body).context("invalid json")?;
    let Value::Object(obj) = value else {
        anyhow::bail!("ovation: expected json object");
    };

    let mut lonlat = Vec::new();
    let mut latlon = Vec::new();
    if let Some(candidate_root) = obj
        .get("coordinates")
        .or_else(|| obj.get("features"))
        .or_else(|| obj.get("data"))
        .or_else(|| obj.get("values"))
        .or_else(|| obj.get("rows"))
        .or_else(|| obj.get("records"))
        .or_else(|| obj.get("items"))
    {
        collect_candidate_triples(candidate_root, &mut lonlat, &mut latlon);
    } else {
        for value in obj.values() {
            collect_candidate_triples(value, &mut lonlat, &mut latlon);
        }
    }

    let (triples, used_lonlat) = if lonlat.len() >= latlon.len() {
        (lonlat, true)
    } else {
        (latlon, false)
    };

    if triples.is_empty() {
        anyhow::bail!("ovation: empty coordinates");
    }

    let mut raw_points: Vec<(i32, i32, f32)> = Vec::new();
    let mut lon_keys: BTreeSet<i32> = BTreeSet::new();
    let mut lat_keys: BTreeSet<i32> = BTreeSet::new();
    let mut max_value = 0.0_f32;

    for (a, b, value) in triples {
        let (lon, lat) = if used_lonlat { (a, b) } else { (b, a) };
        let lon_key = scaled_key(lon);
        let lat_key = scaled_key(lat);
        lon_keys.insert(lon_key);
        lat_keys.insert(lat_key);
        raw_points.push((lon_key, lat_key, value));
        if value > max_value {
            max_value = value;
        }
    }

    if lon_keys.is_empty() || lat_keys.is_empty() {
        anyhow::bail!("ovation: empty coordinates");
    }

    let lon_values: Vec<i32> = lon_keys.into_iter().collect();
    let lat_values: Vec<i32> = lat_keys.into_iter().collect();
    let lon_index = build_index_map(&lon_values);
    let lat_index = build_index_map(&lat_values);

    let grid_width = lon_values.len();
    let grid_height = lat_values.len();
    let mut grid_values = vec![0.0_f32; grid_width * grid_height];

    for (lon_key, lat_key, value) in raw_points {
        if let (Some(&x), Some(&y)) = (lon_index.get(&lon_key), lat_index.get(&lat_key)) {
            let idx = y * grid_width + x;
            if value > grid_values[idx] {
                grid_values[idx] = value;
            }
        }
    }

    let lon_min = (lon_values[0] as f32) / 1000.0;
    let lat_min = (lat_values[0] as f32) / 1000.0;
    let lon_step = step_from_keys(&lon_values).unwrap_or(1.0);
    let lat_step = step_from_keys(&lat_values).unwrap_or(1.0);

    let updated_utc = ovation_timestamp(&obj).or_else(|| Some(Utc::now()));

    Ok(AuroraGrid {
        points: Vec::new(),
        grid_values,
        grid_width,
        grid_height,
        lon_min,
        lat_min,
        lon_step,
        lat_step,
        max_value,
        updated_utc,
    })
}

fn ovation_timestamp(obj: &serde_json::Map<String, Value>) -> Option<DateTime<Utc>> {
    for key in [
        "Forecast Time",
        "Observation Time",
        "forecast_time",
        "observation_time",
        "time_tag",
        "timestamp",
        "time",
    ] {
        if let Some(Value::String(value)) = obj.get(key) {
            if let Some(ts) = parse_timestamp(value) {
                return Some(ts);
            }
        }
    }
    None
}


fn scaled_key(value: f32) -> i32 {
    (value * 1000.0).round() as i32
}

fn build_index_map(values: &[i32]) -> HashMap<i32, usize> {
    values
        .iter()
        .enumerate()
        .map(|(idx, value)| (*value, idx))
        .collect()
}

fn step_from_keys(values: &[i32]) -> Option<f32> {
    if values.len() < 2 {
        return None;
    }
    let mut min_step = i32::MAX;
    for window in values.windows(2) {
        let step = window[1] - window[0];
        if step > 0 && step < min_step {
            min_step = step;
        }
    }
    if min_step == i32::MAX {
        None
    } else {
        Some((min_step as f32) / 1000.0)
    }
}

fn header_looks_numeric(header: &[String]) -> bool {
    if header.is_empty() {
        return false;
    }
    let numeric = header
        .iter()
        .filter(|cell| parse_f32(Some(cell.as_str())).is_some())
        .count();
    numeric >= header.len().saturating_sub(1).max(1)
}

fn infer_ovation_columns(rows: &[Vec<String>]) -> Option<(usize, usize, usize)> {
    let row_count = rows.len();
    if row_count == 0 {
        return None;
    }
    let col_count = rows.iter().map(|row| row.len()).max().unwrap_or(0);
    if col_count == 0 {
        return None;
    }

    let min_required = ((row_count as f32) * 0.5).ceil() as usize;
    let min_required = min_required.max(1);
    let mut numeric_counts = vec![0usize; col_count];
    let mut mins = vec![f32::INFINITY; col_count];
    let mut maxs = vec![f32::NEG_INFINITY; col_count];

    for row in rows.iter() {
        for (idx, cell) in row.iter().enumerate() {
            if let Some(value) = parse_f32(Some(cell.as_str())) {
                numeric_counts[idx] += 1;
                if value < mins[idx] {
                    mins[idx] = value;
                }
                if value > maxs[idx] {
                    maxs[idx] = value;
                }
            }
        }
    }

    let mut lat_candidates = Vec::new();
    let mut lon_candidates = Vec::new();
    let mut value_candidates = Vec::new();

    for idx in 0..col_count {
        if numeric_counts[idx] < min_required {
            continue;
        }
        let min = mins[idx];
        let max = maxs[idx];
        if min.is_finite() && max.is_finite() {
            if min >= -90.0 && max <= 90.0 {
                lat_candidates.push(idx);
            }
            if min >= -180.0 && max <= 360.0 {
                lon_candidates.push(idx);
            }
            if max > 0.0 {
                value_candidates.push(idx);
            }
        }
    }

    if lat_candidates.is_empty() || lon_candidates.is_empty() {
        return None;
    }
    let lat_idx = *lat_candidates.first()?;
    let lon_idx = *lon_candidates.iter().find(|idx| **idx != lat_idx)?;

    let mut value_idx = value_candidates
        .iter()
        .find(|idx| **idx != lat_idx && **idx != lon_idx)
        .copied();

    if value_idx.is_none() && col_count == 3 {
        for idx in 0..col_count {
            if idx != lat_idx && idx != lon_idx {
                value_idx = Some(idx);
                break;
            }
        }
    }

    value_idx.map(|value| (lat_idx, lon_idx, value))
}

fn latest_numeric(rows: &[Vec<String>], idx: usize) -> Option<f32> {
    rows.iter()
        .rev()
        .find_map(|row| parse_f32(get_cell(row, idx)))
}

fn latest_numeric_with_time(
    rows: &[Vec<String>],
    idx: usize,
    time_idx: Option<usize>,
) -> Option<(f32, Option<DateTime<Utc>>)> {
    for row in rows.iter().rev() {
        if let Some(value) = parse_f32(get_cell(row, idx)) {
            let timestamp = time_idx.and_then(|t_idx| {
                get_cell(row, t_idx).and_then(|value| parse_timestamp(value))
            });
            return Some((value, timestamp));
        }
    }
    None
}

fn latest_timestamp(rows: &[Vec<String>], time_idx: Option<usize>) -> Option<DateTime<Utc>> {
    let t_idx = time_idx?;
    for row in rows.iter().rev() {
        if let Some(ts) = get_cell(row, t_idx).and_then(parse_timestamp) {
            return Some(ts);
        }
    }
    None
}

fn parse_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    let value = raw.trim();
    if value.is_empty() || value == "null" {
        return None;
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Some(Utc.from_utc_datetime(&dt));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f") {
        return Some(Utc.from_utc_datetime(&dt));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn parse_table_array_rows() {
        let body = r#"[["time_tag","kp"],["2024-01-01 00:00:00","2.33"]]"#;
        let table = parse_json_table(body).unwrap();
        assert_eq!(table.header, vec!["time_tag", "kp"]);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0][1], "2.33");
    }

    #[test]
    fn parse_kp_latest() {
        let body = r#"[["time_tag","kp"],["2024-01-01 00:00:00","1.0"],["2024-01-01 03:00:00","2.67"]]"#;
        let table = parse_json_table(body).unwrap();
        let kp_idx = find_column(&table.header, &["kp"]).unwrap();
        let time_idx = find_column(&table.header, &["time_tag"]);
        let (value, _) = latest_numeric_with_time(&table.rows, kp_idx, time_idx).unwrap();
        assert!((value - 2.67).abs() < 1e-4);
    }

    #[test]
    fn parse_ovation_points() {
        let body = r#"[["lat","lon","aurora"],["65.0","-150.0","42"],["66.0","-151.0","0"]]"#;
        let table = parse_json_table(body).unwrap();
        let lat_idx = find_column(&table.header, &["lat"]).unwrap();
        let lon_idx = find_column(&table.header, &["lon"]).unwrap();
        let value_idx = find_column(&table.header, &["aurora"]).unwrap();
        let mut points = Vec::new();
        for row in table.rows.iter() {
            let lat = parse_f32(get_cell(row, lat_idx)).unwrap();
            let lon = parse_f32(get_cell(row, lon_idx)).unwrap();
            let value = parse_f32(get_cell(row, value_idx)).unwrap();
            points.push(AuroraPoint { lat, lon, value });
        }
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].lat, 65.0);
    }

    #[test]
    fn parse_ovation_object_coordinates_grid() {
        let body = r#"{
            "Forecast Time": "2024-01-01 00:00:00",
            "coordinates": [
                [0, -90, 1.0],
                [1, -90, 2.0],
                [0, -89, 3.0],
                [1, -89, 4.0]
            ]
        }"#;
        let grid = parse_ovation_object(body).unwrap();
        assert_eq!(grid.grid_width, 2);
        assert_eq!(grid.grid_height, 2);
        assert_eq!(grid.grid_values.len(), 4);
        assert!((grid.max_value - 4.0).abs() < 1e-4);
        assert!((grid.lon_min - 0.0).abs() < 1e-4);
        assert!((grid.lat_min + 90.0).abs() < 1e-4);
        assert!((grid.lon_step - 1.0).abs() < 1e-4);
        assert!((grid.lat_step - 1.0).abs() < 1e-4);
        assert!((grid.grid_values[0] - 1.0).abs() < 1e-4);
        assert!((grid.grid_values[1] - 2.0).abs() < 1e-4);
        assert!((grid.grid_values[2] - 3.0).abs() < 1e-4);
        assert!((grid.grid_values[3] - 4.0).abs() < 1e-4);
        let expected = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(grid.updated_utc, Some(expected));
    }

    #[test]
    fn parse_ovation_geojson_like() {
        let body = r#"{
            "features": [
                { "geometry": { "coordinates": [10, 50, 5.0] } },
                { "geometry": { "coordinates": [11, 50, 6.0] } }
            ]
        }"#;
        let grid = parse_ovation_object(body).unwrap();
        assert_eq!(grid.grid_width, 2);
        assert_eq!(grid.grid_height, 1);
        assert!((grid.max_value - 6.0).abs() < 1e-4);
    }

    #[test]
    fn parse_ovation_fixture_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push("ovation_aurora_latest.json");
        let body = fs::read_to_string(path).expect("read ovation fixture");
        let grid = parse_ovation_object(&body).unwrap();
        assert_eq!(grid.grid_width, 360);
        assert_eq!(grid.grid_height, 181);
        assert_eq!(grid.grid_values.len(), grid.grid_width * grid.grid_height);
        assert!(grid.max_value >= 0.0);
    }
}
