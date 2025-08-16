//! Predefined satellite groups for easy selection

/// Predefined satellite groups available on Celestrak
pub const SATELLITE_GROUPS: &[(&str, &str)] = &[
    ("weather", "Weather Satellites"),
    ("noaa", "NOAA Satellites"),
    ("goes", "GOES Satellites"),
    ("resource", "Earth Resources Satellites"),
    ("sarsat", "Search & Rescue Satellites"),
    ("dmc", "Disaster Monitoring Satellites"),
    ("tdrss", "Tracking & Data Relay Satellites"),
    ("argos", "ARGOS Data Collection Satellites"),
    ("planet", "Planet Labs Satellites"),
    ("spire", "Spire Global Satellites"),
    ("globalstar", "Globalstar Satellites"),
    ("iridium", "Iridium Satellites"),
    ("iridium-NEXT", "Iridium NEXT Satellites"),
    ("orbcomm", "Orbcomm Satellites"),
    ("inmarsat", "Inmarsat Satellites"),
    ("ses", "SES Satellites"),
    ("intelsat", "Intelsat Satellites"),
    ("eutelsat", "Eutelsat Satellites"),
    ("telesat", "Telesat Satellites"),
    ("starlink", "Starlink Satellites"),
    ("oneweb", "OneWeb Satellites"),
    ("swarm", "Swarm Technologies Satellites"),
    ("amateur", "Amateur Radio Satellites"),
    ("x-comm", "Experimental Communications"),
    ("other-comm", "Other Communications Satellites"),
    ("satnogs", "SatNOGS Satellites"),
    ("gorizont", "Gorizont Satellites"),
    ("raduga", "Raduga Satellites"),
    ("molniya", "Molniya Satellites"),
    ("gps-ops", "GPS Operational Satellites"),
    ("glo-ops", "GLONASS Operational Satellites"),
    ("galileo", "Galileo Satellites"),
    ("beidou", "Beidou Satellites"),
    ("sbas", "Satellite-Based Augmentation System"),
    ("nnss", "Navy Navigation Satellite System"),
    ("musson", "Russian LEO Navigation Satellites"),
    ("science", "Space & Earth Science Satellites"),
    ("geodetic", "Geodetic Satellites"),
    ("engineering", "Engineering Satellites"),
    ("education", "Education Satellites"),
];

/// Get display name for a group
pub fn get_group_display_name(group: &str) -> &str {
    SATELLITE_GROUPS
        .iter()
        .find(|(key, _)| *key == group)
        .map(|(_, name)| *name)
        .unwrap_or(group)
}

/// Get all group keys
#[allow(dead_code)]
pub fn get_all_groups() -> Vec<&'static str> {
    SATELLITE_GROUPS.iter().map(|(key, _)| *key).collect()
}
