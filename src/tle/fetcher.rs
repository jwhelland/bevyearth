//! TLE fetching functionality

use crate::tle::types::{FetchChannels, FetchCommand, FetchResultMsg};
use crate::tle::parser::parse_tle_epoch_to_utc;
use chrono::Utc;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

/// Start the background TLE worker thread
pub fn start_tle_worker() -> FetchChannels {
    let (cmd_tx, cmd_rx) = mpsc::channel::<FetchCommand>();
    let (res_tx, res_rx) = mpsc::channel::<FetchResultMsg>();
    
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let client = reqwest::Client::new();

            // Helper: scan arbitrary response for a valid TLE pair, optionally with name
            fn extract_tle_block(body: &str, requested_sat: u32) -> anyhow::Result<(Option<String>, String, String)> {
                let mut lines: Vec<String> = Vec::new();
                for raw in body.lines() {
                    let line = raw.trim_matches(|c| c == '\u{feff}' || c == '\r' || c == '\n' || c == ' '); // trim BOM/CRLF/space
                    if line.is_empty() {
                        continue;
                    }
                    lines.push(line.to_string());
                }
                // find first pair 1/2 with matching sat number
                let sat_fmt = format!("{:05}", requested_sat);
                let mut i = 0usize;
                while i + 1 < lines.len() {
                    let l = &lines[i];
                    let n = if i >= 1 { Some(lines[i - 1].clone()) } else { None };
                    if l.starts_with('1') {
                        let l1 = l;
                        let l2 = &lines[i + 1];
                        if l2.starts_with('2') {
                            let sat_ok = l1.len() >= 7 && l2.len() >= 7 && l1[2..7] == sat_fmt && l2[2..7] == sat_fmt;
                            if sat_ok {
                                // Prefer a text name line immediately before l1 if it is not a TLE line
                                let name = n.filter(|p| !p.starts_with('1') && !p.starts_with('2'));
                                return Ok((name, l1.to_string(), l2.to_string()));
                            }
                        }
                    }
                    i += 1;
                }
                let sample: String = body.lines().take(6).collect::<Vec<_>>().join("\\n");
                anyhow::bail!("No valid TLE pair found for {}. Sample: {}", requested_sat, sample);
            }

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
                            // Debug log full fetch result (status, first lines, and any extracted tuple)
                            println!("[TLE FETCH] norad={} status={} url={} bytes={}...", norad, status, url, body.len());
                            // Attempt parse even if not 2xx, to capture HTML/text bodies for debugging
                            let (name, l1, l2) = extract_tle_block(&body, norad)?;
                            println!("[TLE PARSED] norad={} name={}\\n{}\\n{}", norad, name.clone().unwrap_or_else(|| "None".into()), l1, l2);
                            // If HTTP not success, still bail after logging to surface error to UI
                            if !status.is_success() {
                                anyhow::bail!("HTTP {} after parse", status);
                            }
                            let epoch = parse_tle_epoch_to_utc(&l1).unwrap_or_else(Utc::now);
                            Ok::<_, anyhow::Error>((name, l1, l2, epoch))
                        }
                        .await;
                        match res {
                            Ok((name, line1, line2, epoch_utc)) => {
                                println!("[TLE RESULT] norad={} SUCCESS epoch={}", norad, epoch_utc.to_rfc3339());
                                send(FetchResultMsg::Success { norad, name, line1, line2, epoch_utc })
                            }
                            Err(e) => {
                                eprintln!("[TLE RESULT] norad={} FAILURE: {}", norad, e);
                                send(FetchResultMsg::Failure { norad, error: e.to_string() })
                            }
                        }
                    }
                    FetchCommand::FetchGroup { group } => {
                        let url = format!(
                            "https://celestrak.org/NORAD/elements/gp.php?GROUP={}&FORMAT=TLE",
                            group
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
                            println!("[TLE GROUP FETCH] group={} status={} url={} bytes={}...", group, status, url, body.len());
                            if !status.is_success() {
                                anyhow::bail!("HTTP {} for group fetch", status);
                            }
                            // Parse the body manually to extract TLE lines since sgp4::parse_3les returns Elements
                            // which doesn't preserve the original TLE line format
                            let mut lines: Vec<String> = Vec::new();
                            for raw in body.lines() {
                                let line = raw.trim_matches(|c| c == '\u{feff}' || c == '\r' || c == '\n' || c == ' ');
                                if !line.is_empty() {
                                    lines.push(line.to_string());
                                }
                            }
                            
                            let mut i = 0;
                            while i < lines.len() {
                                // Look for TLE line 1 (starts with '1')
                                if i + 1 < lines.len() && lines[i].starts_with('1') && lines[i + 1].starts_with('2') {
                                    let line1 = &lines[i];
                                    let line2 = &lines[i + 1];
                                    
                                    // Extract NORAD ID from line1 (columns 3-7, 0-based)
                                    let norad = line1.get(2..7)
                                        .and_then(|s| s.trim().parse::<u32>().ok())
                                        .unwrap_or(0);
                                    
                                    // Look for name line before TLE (if exists and is not a TLE line)
                                    let name = if i > 0 && !lines[i-1].starts_with('1') && !lines[i-1].starts_with('2') {
                                        Some(lines[i-1].clone())
                                    } else {
                                        None
                                    };
                                    
                                    let epoch_utc = parse_tle_epoch_to_utc(line1).unwrap_or_else(Utc::now);
                                    println!("[TLE GROUP PARSED] norad={} name={:?}", norad, name);
                                    send(FetchResultMsg::Success {
                                        norad,
                                        name,
                                        line1: line1.clone(),
                                        line2: line2.clone(),
                                        epoch_utc
                                    });
                                    
                                    i += 2; // Skip both TLE lines
                                } else {
                                    i += 1;
                                }
                            }
                            Ok::<_, anyhow::Error>(())
                        }
                        .await;
                        if let Err(e) = res {
                            eprintln!("[TLE GROUP RESULT] group={} FAILURE: {}", group, e);
                            // Optionally, could send a failure for each norad, but here just log
                        }
                    }
                }
            }
        });
    });
    
    FetchChannels { 
        cmd_tx, 
        res_rx: Arc::new(Mutex::new(res_rx)) 
    }
}