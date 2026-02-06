//! Predefined satellite groups for easy selection

/// Predefined satellite groups available on Celestrak
pub const SATELLITE_GROUPS: &[(&str, &str)] = &[
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=geo&FORMAT=TLE",
        "Active Geosynchronous",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=analyst&FORMAT=TLE",
        "Analyst",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=amateur&FORMAT=TLE",
        "Amateur Radio",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=argos&FORMAT=TLE",
        "ARGOS Data Collection",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=beidou&FORMAT=TLE",
        "Beidou",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=cubesat&FORMAT=TLE",
        "CubeSats",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=dmc&FORMAT=TLE",
        "Disaster Monitoring",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=resource&FORMAT=TLE",
        "Earth Resources",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=education&FORMAT=TLE",
        "Education",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=engineering&FORMAT=TLE",
        "Engineering",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=eutelsat&FORMAT=TLE",
        "Eutelsat",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=x-comm&FORMAT=TLE",
        "Experimental Communications",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=galileo&FORMAT=TLE",
        "Galileo",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?SPECIAL=gpz&FORMAT=tle",
        "GEO Protected Zone",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=geodetic&FORMAT=TLE",
        "Geodetic",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=glo-ops&FORMAT=TLE",
        "GLONASS Operational",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=globalstar&FORMAT=TLE",
        "Globalstar",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=goes&FORMAT=TLE",
        "GOES",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=gorizont&FORMAT=TLE",
        "Gorizont",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=gps-ops&FORMAT=TLE",
        "GPS Operational",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=hulianwang&FORMAT=TLE",
        "Hulianwang Digui",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=intelsat&FORMAT=TLE",
        "Intelsat",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=iridium-NEXT&FORMAT=TLE",
        "Iridium NEXT",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=iridium&FORMAT=TLE",
        "Iridium",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=last-30-days&FORMAT=TLE",
        "Last 30 Days Launches",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=movers&FORMAT=TLE",
        "Movers",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=military&FORMAT=TLE",
        " Miscellaneous Military",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=molniya&FORMAT=TLE",
        "Molniya",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=nnss&FORMAT=TLE",
        "Navy Navigation Satellite System",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=noaa&FORMAT=TLE",
        "NOAA",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=oneweb&FORMAT=TLE",
        "OneWeb",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=kuiper&FORMAT=TLE",
        "Kuiper",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=orbcomm&FORMAT=TLE",
        "Orbcomm",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=other-comm&FORMAT=TLE",
        "Other Communications",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=planet&FORMAT=TLE",
        "Planet Labs",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=qianfan&FORMAT=TLE",
        "Qianfan",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=radar&FORMAT=TLE",
        "Radar Calibration",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=raduga&FORMAT=TLE",
        "Raduga",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=musson&FORMAT=TLE",
        "Russian LEO Navigation",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=sarsat&FORMAT=TLE",
        "Search & Rescue",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=satnogs&FORMAT=TLE",
        "SatNOGS",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=sbas&FORMAT=TLE",
        "Satellite-Based Augmentation System",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=science&FORMAT=TLE",
        "Space & Earth Science",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=ses&FORMAT=TLE",
        "SES",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=spire&FORMAT=TLE",
        "Spire Global",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=starlink&FORMAT=TLE",
        "Starlink",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=swarm&FORMAT=TLE",
        "Swarm Technologies",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=tdrss&FORMAT=TLE",
        "Tracking & Data Relay",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=telesat&FORMAT=TLE",
        "Telesat",
    ),
    (
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=weather&FORMAT=TLE",
        "Weather",
    ),
];
