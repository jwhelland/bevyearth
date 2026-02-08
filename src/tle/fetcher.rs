// TLE fetching functionality

use crate::tle::parser::parse_tle_epoch_to_utc;
use crate::tle::types::{FetchChannels, FetchCommand, FetchResultMsg};
use chrono::Utc;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

/// Parsed TLE entry with optional name, line pair, NORAD ID, and epoch.
struct TleEntry {
    name: Option<String>,
    line1: String,
    line2: String,
    norad: u32,
}

/// Clean a TLE response body: strip BOM, CRLF, leading/trailing whitespace,
/// and drop empty lines.
fn clean_tle_lines(body: &str) -> Vec<String> {
    body.lines()
        .map(|raw| {
            raw.trim_matches(|c| c == '\u{feff}' || c == '\r' || c == '\n' || c == ' ')
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .collect()
}

/// Iterate over cleaned TLE lines and yield all valid (line1, line2) pairs
/// with optional preceding name line and extracted NORAD ID.
fn parse_tle_pairs(lines: &[String]) -> Vec<TleEntry> {
    let mut entries = Vec::new();
    let mut i = 0;
    while i + 1 < lines.len() {
        if lines[i].starts_with('1') && lines[i + 1].starts_with('2') {
            let line1 = &lines[i];
            let line2 = &lines[i + 1];

            let norad = line1
                .get(2..7)
                .and_then(|s| s.trim().parse::<u32>().ok())
                .unwrap_or(0);

            // A non-TLE line immediately before line1 is the satellite name
            let name = if i > 0 && !lines[i - 1].starts_with('1') && !lines[i - 1].starts_with('2')
            {
                Some(lines[i - 1].clone())
            } else {
                None
            };

            entries.push(TleEntry {
                name,
                line1: line1.clone(),
                line2: line2.clone(),
                norad,
            });

            i += 2; // Skip both TLE lines
        } else {
            i += 1;
        }
    }
    entries
}

/// Start the background TLE worker thread
pub fn start_tle_worker() -> FetchChannels {
    let (cmd_tx, cmd_rx) = mpsc::channel::<FetchCommand>();
    let (res_tx, res_rx) = mpsc::channel::<FetchResultMsg>();

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let client = reqwest::Client::new();

            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    FetchCommand::Fetch(norad) => {
                        let url = format!(
                            "https://celestrak.org/NORAD/elements/gp.php?CATNR={}&FORMAT=TLE",
                            norad
                        );
                        let send = |m| {
                            let _ = res_tx.send(m);
                        };
                        let res = async {
                            let resp = client
                                .get(&url)
                                .header("accept", "text/plain")
                                .send()
                                .await?;
                            let status = resp.status();
                            let body = resp.text().await?;

                            // Attempt parse even if not 2xx, to capture HTML/text bodies for debugging
                            let lines = clean_tle_lines(&body);
                            let pairs = parse_tle_pairs(&lines);
                            let sat_fmt = format!("{:05}", norad);
                            let entry = pairs
                                .into_iter()
                                .find(|e| {
                                    e.line1.len() >= 7
                                        && e.line2.len() >= 7
                                        && e.line1[2..7] == sat_fmt
                                        && e.line2[2..7] == sat_fmt
                                })
                                .ok_or_else(|| {
                                    let sample: String =
                                        body.lines().take(6).collect::<Vec<_>>().join("\n");
                                    anyhow::anyhow!(
                                        "No valid TLE pair found for {}. Sample: {}",
                                        norad,
                                        sample
                                    )
                                })?;

                            // If HTTP not success, still bail after logging to surface error to UI
                            if !status.is_success() {
                                anyhow::bail!("HTTP {} after parse", status);
                            }
                            let epoch =
                                parse_tle_epoch_to_utc(&entry.line1).unwrap_or_else(Utc::now);
                            Ok::<_, anyhow::Error>((entry.name, entry.line1, entry.line2, epoch))
                        }
                        .await;
                        match res {
                            Ok((name, line1, line2, epoch_utc)) => {
                                println!(
                                    "[TLE RESULT] norad={} SUCCESS epoch={}",
                                    norad,
                                    epoch_utc.to_rfc3339()
                                );
                                send(FetchResultMsg::Success {
                                    norad,
                                    name,
                                    line1,
                                    line2,
                                    epoch_utc,
                                    group: None,
                                })
                            }
                            Err(e) => {
                                eprintln!("[TLE RESULT] norad={} FAILURE: {}", norad, e);
                                send(FetchResultMsg::Failure {
                                    norad,
                                    error: e.to_string(),
                                })
                            }
                        }
                    }
                    FetchCommand::FetchGroup { group } => {
                        let send = |m| {
                            let _ = res_tx.send(m);
                        };
                        let group_name = group.clone();
                        let res = async {
                            let resp = client
                                .get(&group)
                                .header("accept", "text/plain")
                                .send()
                                .await?;
                            let status = resp.status();
                            let body = resp.text().await?;

                            if !status.is_success() {
                                anyhow::bail!("HTTP {} for group fetch", status);
                            }

                            let lines = clean_tle_lines(&body);
                            let entries = parse_tle_pairs(&lines);
                            let count = entries.len();

                            for entry in entries {
                                let epoch_utc =
                                    parse_tle_epoch_to_utc(&entry.line1).unwrap_or_else(Utc::now);
                                println!(
                                    "[TLE GROUP PARSED] norad={} name={:?}",
                                    entry.norad, entry.name
                                );
                                send(FetchResultMsg::Success {
                                    norad: entry.norad,
                                    name: entry.name,
                                    line1: entry.line1,
                                    line2: entry.line2,
                                    epoch_utc,
                                    group: Some(group_name.clone()),
                                });
                            }
                            Ok::<_, anyhow::Error>(count)
                        }
                        .await;
                        match res {
                            Ok(count) => {
                                println!(
                                    "[TLE GROUP RESULT] group={} SUCCESS count={}",
                                    group_name, count
                                );
                                send(FetchResultMsg::GroupDone {
                                    group: group_name,
                                    count,
                                });
                            }
                            Err(e) => {
                                eprintln!("[TLE GROUP RESULT] group={} FAILURE: {}", group_name, e);
                                send(FetchResultMsg::GroupFailure {
                                    group: group_name,
                                    error: e.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        });
    });

    FetchChannels {
        cmd_tx,
        res_rx: Arc::new(Mutex::new(res_rx)),
    }
}
