//! TLE processing systems

use bevy::prelude::*;
use crate::tle::types::{FetchChannels, FetchResultMsg, TleData};
use crate::tle::parser::parse_tle_epoch_to_utc;
use crate::satellite::SatelliteStore;

/// System to drain fetch results and build SGP4 propagators
pub fn process_fetch_results_system(
    mut store: ResMut<SatelliteStore>,
    fetch: Option<Res<FetchChannels>>,
) {
    let Some(fetch) = fetch else { return };
    let Ok(guard) = fetch.res_rx.lock() else { return };
    while let Ok(msg) = guard.try_recv() {
        match msg {
            FetchResultMsg::Success {
                norad,
                name,
                line1,
                line2,
                epoch_utc,
            } => {
                println!("[TLE DISPATCH] received SUCCESS for norad={}", norad);
                if let Some(s) = store.items.iter_mut().find(|s| s.norad == norad) {
                    // clear previous error
                    s.error = None;
                    s.name = name.or_else(|| Some(format!("NORAD {}", norad)));
                    let epoch = parse_tle_epoch_to_utc(&line1).unwrap_or(epoch_utc);
                    s.tle = Some(TleData {
                        name: s.name.clone(),
                        line1: line1.clone(),
                        line2: line2.clone(),
                        epoch_utc: epoch,
                    });
                    // Build SGP4 model (sgp4 2.3.0): parse TLE -> Elements -> Constants
                    match sgp4::Elements::from_tle(s.name.clone(), line1.as_bytes(), line2.as_bytes()) {
                        Ok(elements) => match sgp4::Constants::from_elements(&elements) {
                            Ok(constants) => {
                                s.propagator = Some(constants);
                                println!("[SGP4] norad={} constants initialized", norad);
                            }
                            Err(e) => {
                                s.propagator = None;
                                s.error = Some(e.to_string());
                                eprintln!("[SGP4] norad={} constants error: {}", norad, s.error.as_deref().unwrap());
                            }
                        },
                        Err(e) => {
                            s.propagator = None;
                            s.error = Some(e.to_string());
                            eprintln!("[SGP4] norad={} elements error: {}", norad, s.error.as_deref().unwrap());
                        }
                    }
                } else {
                    eprintln!("[TLE DISPATCH] norad={} not found in store", norad);
                }
            }
            FetchResultMsg::Failure { norad, error } => {
                eprintln!("[TLE DISPATCH] received FAILURE for norad={}: {}", norad, error);
                if let Some(s) = store.items.iter_mut().find(|s| s.norad == norad) {
                    // keep existing name if any; record error and clear models
                    s.error = Some(error);
                    s.tle = None;
                    s.propagator = None;
                } else {
                    eprintln!("[TLE DISPATCH] failure for unknown norad={} (not in store)", norad);
                }
            }
        }
    }
}