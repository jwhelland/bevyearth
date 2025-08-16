//! Predefined satellite groups for easy selection

/// Predefined satellite groups available on Celestrak
pub const SATELLITE_GROUPS: &[(&str, &str)] = &[
    ("amateur", "Amateur Radio Satellites"),
    ("argos", "ARGOS Data Collection Satellites"),
    ("beidou", "Beidou Satellites"),
    ("dmc", "Disaster Monitoring Satellites"),
    ("education", "Education Satellites"),
    ("engineering", "Engineering Satellites"),
    ("eutelsat", "Eutelsat Satellites"),
    ("galileo", "Galileo Satellites"),
    ("geodetic", "Geodetic Satellites"),
    ("glo-ops", "GLONASS Operational Satellites"),
    ("globalstar", "Globalstar Satellites"),
    ("goes", "GOES Satellites"),
    ("gorizont", "Gorizont Satellites"),
    ("gps-ops", "GPS Operational Satellites"),
    ("inmarsat", "Inmarsat Satellites"),
    ("intelsat", "Intelsat Satellites"),
    ("iridium-NEXT", "Iridium NEXT Satellites"),
    ("iridium", "Iridium Satellites"),
    ("molniya", "Molniya Satellites"),
    ("musson", "Russian LEO Navigation Satellites"),
    ("nnss", "Navy Navigation Satellite System"),
    ("noaa", "NOAA Satellites"),
    ("oneweb", "OneWeb Satellites"),
    ("orbcomm", "Orbcomm Satellites"),
    ("other-comm", "Other Communications Satellites"),
    ("planet", "Planet Labs Satellites"),
    ("raduga", "Raduga Satellites"),
    ("resource", "Earth Resources Satellites"),
    ("sarsat", "Search & Rescue Satellites"),
    ("satnogs", "SatNOGS Satellites"),
    ("sbas", "Satellite-Based Augmentation System"),
    ("science", "Space & Earth Science Satellites"),
    ("ses", "SES Satellites"),
    ("spire", "Spire Global Satellites"),
    ("starlink", "Starlink Satellites"),
    ("swarm", "Swarm Technologies Satellites"),
    ("tdrss", "Tracking & Data Relay Satellites"),
    ("telesat", "Telesat Satellites"),
    ("weather", "Weather Satellites"),
    ("x-comm", "Experimental Communications"),
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
