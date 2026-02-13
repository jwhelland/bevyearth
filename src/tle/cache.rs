//! TLE disk caching module
//!
//! Provides persistent caching of TLE data to disk, reducing network requests
//! and enabling offline operation for recently-viewed satellites.

use chrono::{DateTime, Duration, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Serialized cache entry stored as JSON on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTle {
    pub norad: u32,
    pub name: Option<String>,
    pub line1: String,
    pub line2: String,
    pub epoch_utc: DateTime<Utc>,
    pub cached_at: DateTime<Utc>,
}

/// TLE disk cache manager
pub struct TleCache {
    cache_dir: PathBuf,
    expiration_days: i64,
}

impl TleCache {
    /// Create a new TLE cache with the specified expiration threshold in days
    ///
    /// Resolves platform-specific cache directory:
    /// - macOS: ~/Library/Caches/bevyearth/tle/
    /// - Linux: ~/.cache/bevyearth/tle/
    /// - Windows: %LOCALAPPDATA%\bevyearth\tle\
    ///
    /// Returns an error if cache directory cannot be resolved or created.
    pub fn new(expiration_days: i64) -> Result<Self, anyhow::Error> {
        let proj_dirs = ProjectDirs::from("", "", "bevyearth")
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve cache directory"))?;

        let cache_dir = proj_dirs.cache_dir().join("tle");
        Self::new_in_dir(cache_dir, expiration_days)
    }

    /// Create a new TLE cache rooted at a specific directory
    ///
    /// This is primarily intended for tests or custom setups where the
    /// platform cache directory is not writable.
    pub fn new_in_dir(cache_dir: PathBuf, expiration_days: i64) -> Result<Self, anyhow::Error> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            cache_dir,
            expiration_days,
        })
    }

    /// Read a cached TLE entry from disk by NORAD ID
    ///
    /// Returns Ok(None) if the cache file doesn't exist (cache miss).
    /// Returns Err if file exists but cannot be read or parsed.
    pub fn read(&self, norad: u32) -> Result<Option<CachedTle>, anyhow::Error> {
        let path = self.cache_path(norad);

        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&path)?;
        let cached: CachedTle = serde_json::from_str(&contents)?;

        Ok(Some(cached))
    }

    /// Write a TLE entry to disk cache
    ///
    /// Creates or overwrites the cache file for the given NORAD ID.
    pub fn write(&self, entry: &CachedTle) -> Result<(), anyhow::Error> {
        let path = self.cache_path(entry.norad);
        let contents = serde_json::to_string_pretty(entry)?;
        fs::write(&path, contents)?;
        Ok(())
    }

    /// Check if a cached TLE entry is still valid based on its epoch
    ///
    /// Returns true if the TLE epoch is within the expiration threshold,
    /// false if it has expired and should be re-fetched.
    pub fn is_valid(&self, entry: &CachedTle) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(entry.epoch_utc);
        age < Duration::days(self.expiration_days)
    }

    /// Get the file path for a cached TLE entry
    fn cache_path(&self, norad: u32) -> PathBuf {
        self.cache_dir.join(format!("{}.json", norad))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(test_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bevyearth-tle-cache-{}-{}-{}",
            test_name,
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn test_cache_validation() {
        let cache_dir = unique_temp_dir("validation");
        let cache = TleCache::new_in_dir(cache_dir, 7).expect("Failed to create cache");

        // Valid entry (recent epoch)
        let valid_entry = CachedTle {
            norad: 25544,
            name: Some("ISS (ZARYA)".to_string()),
            line1: "1 25544U 98067A   26044.51782528".to_string(),
            line2: "2 25544  51.6416 247.4627 0006703".to_string(),
            epoch_utc: Utc::now() - Duration::days(3),
            cached_at: Utc::now(),
        };
        assert!(cache.is_valid(&valid_entry));

        // Expired entry (old epoch)
        let expired_entry = CachedTle {
            norad: 25544,
            name: Some("ISS (ZARYA)".to_string()),
            line1: "1 25544U 98067A   26044.51782528".to_string(),
            line2: "2 25544  51.6416 247.4627 0006703".to_string(),
            epoch_utc: Utc::now() - Duration::days(10),
            cached_at: Utc::now(),
        };
        assert!(!cache.is_valid(&expired_entry));
    }

    #[test]
    fn test_cache_write_and_read() {
        let cache_dir = unique_temp_dir("write_and_read");
        let cache = TleCache::new_in_dir(cache_dir, 7).expect("Failed to create cache");

        // Create a test TLE entry
        let test_entry = CachedTle {
            norad: 99999,
            name: Some("TEST SATELLITE".to_string()),
            line1: "1 99999U 24001A   26044.51782528  .00000000  00000-0  00000-0 0  9999"
                .to_string(),
            line2: "2 99999  51.6416 247.4627 0006703 290.1234  69.8765 15.48919393123456"
                .to_string(),
            epoch_utc: Utc::now(),
            cached_at: Utc::now(),
        };

        // Write to cache
        cache.write(&test_entry).expect("Failed to write to cache");

        // Read from cache
        let cached = cache
            .read(99999)
            .expect("Failed to read from cache")
            .expect("Cache entry not found");

        // Verify data matches
        assert_eq!(cached.norad, 99999);
        assert_eq!(cached.name, Some("TEST SATELLITE".to_string()));
        assert_eq!(cached.line1, test_entry.line1);
        assert_eq!(cached.line2, test_entry.line2);

        // Verify it's valid (recent epoch)
        assert!(cache.is_valid(&cached));
    }

    #[test]
    fn test_cache_expiration() {
        let cache_dir = unique_temp_dir("expiration");
        let cache = TleCache::new_in_dir(cache_dir, 7).expect("Failed to create cache");

        // Create an entry with an old epoch (10 days ago)
        let old_entry = CachedTle {
            norad: 88888,
            name: Some("OLD SATELLITE".to_string()),
            line1: "1 88888U 24001A   26044.51782528  .00000000  00000-0  00000-0 0  9999"
                .to_string(),
            line2: "2 88888  51.6416 247.4627 0006703 290.1234  69.8765 15.48919393123456"
                .to_string(),
            epoch_utc: Utc::now() - Duration::days(10),
            cached_at: Utc::now(),
        };

        // Write to cache
        cache.write(&old_entry).expect("Failed to write to cache");

        // Read from cache
        let cached = cache
            .read(88888)
            .expect("Failed to read from cache")
            .expect("Cache entry not found");

        // Should be marked as expired
        assert!(!cache.is_valid(&cached));
    }

    #[test]
    fn test_cache_miss() {
        let cache_dir = unique_temp_dir("miss");
        let cache = TleCache::new_in_dir(cache_dir, 7).expect("Failed to create cache");

        // Try to read a non-existent entry
        let result = cache.read(77777).expect("Read should not error");

        // Should be None (cache miss)
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_file_persistence() {
        let cache_dir = unique_temp_dir("persistence");
        let cache = TleCache::new_in_dir(cache_dir.clone(), 7).expect("Failed to create cache");

        // Write an entry
        let entry = CachedTle {
            norad: 55555,
            name: Some("PERSIST TEST".to_string()),
            line1: "1 55555U 24001A   26044.51782528  .00000000  00000-0  00000-0 0  9999"
                .to_string(),
            line2: "2 55555  51.6416 247.4627 0006703 290.1234  69.8765 15.48919393123456"
                .to_string(),
            epoch_utc: Utc::now(),
            cached_at: Utc::now(),
        };

        cache.write(&entry).expect("Write should succeed");

        // Create a new cache instance (simulating app restart)
        let cache2 = TleCache::new_in_dir(cache_dir, 7).expect("Failed to create second cache");

        // Read from the new instance
        let loaded = cache2
            .read(55555)
            .expect("Read should succeed")
            .expect("Entry should exist");

        // Verify data persisted
        assert_eq!(loaded.norad, 55555);
        assert_eq!(loaded.name.as_deref(), Some("PERSIST TEST"));
    }

    #[test]
    fn test_cache_with_custom_expiration() {
        // Short expiration window (1 day)
        let cache_dir = unique_temp_dir("custom_expiration");
        let cache = TleCache::new_in_dir(cache_dir, 1).expect("Failed to create cache");

        // Entry from 2 days ago
        let old_entry = CachedTle {
            norad: 44444,
            name: Some("SHORT EXPIRY".to_string()),
            line1: "1 44444U 24001A   26044.51782528  .00000000  00000-0  00000-0 0  9999"
                .to_string(),
            line2: "2 44444  51.6416 247.4627 0006703 290.1234  69.8765 15.48919393123456"
                .to_string(),
            epoch_utc: Utc::now() - Duration::days(2),
            cached_at: Utc::now(),
        };

        // Should be invalid with 1-day expiration
        assert!(!cache.is_valid(&old_entry));

        // Long expiration window (30 days)
        let cache_dir_30 = unique_temp_dir("custom_expiration_30");
        let cache30 = TleCache::new_in_dir(cache_dir_30, 30).expect("Failed to create cache");

        // Same entry should be valid with 30-day expiration
        assert!(cache30.is_valid(&old_entry));
    }

    #[test]
    fn test_integration_cache_then_network_simulation() {
        let cache_dir = unique_temp_dir("integration");
        let cache = TleCache::new_in_dir(cache_dir, 7).expect("Failed to create cache");
        let test_norad = 33333;

        // Simulate first fetch: cache miss, then network fetch
        let read_result = cache.read(test_norad);
        assert!(
            read_result.as_ref().unwrap().is_none(),
            "First read should be cache miss"
        );

        // Simulate network fetch result
        let network_data = CachedTle {
            norad: test_norad,
            name: Some("INTEGRATION TEST".to_string()),
            line1: "1 33333U 24001A   26044.51782528  .00000000  00000-0  00000-0 0  9999"
                .to_string(),
            line2: "2 33333  51.6416 247.4627 0006703 290.1234  69.8765 15.48919393123456"
                .to_string(),
            epoch_utc: Utc::now(),
            cached_at: Utc::now(),
        };

        // Write network result to cache
        cache
            .write(&network_data)
            .expect("Cache write should succeed");

        // Simulate second fetch: cache hit
        let cached_result = cache
            .read(test_norad)
            .expect("Read should succeed")
            .expect("Cache should contain entry");

        assert!(cache.is_valid(&cached_result), "Cache should be valid");
        assert_eq!(cached_result.norad, test_norad);
        assert_eq!(cached_result.name.as_deref(), Some("INTEGRATION TEST"));
    }
}
