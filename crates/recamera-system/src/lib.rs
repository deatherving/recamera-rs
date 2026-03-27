//! System and device utilities for the recamera SDK.
//!
//! This crate provides access to device information, LED control via sysfs,
//! system uptime, and reboot functionality.

use std::fs;
use std::path::PathBuf;

use recamera_core::{Error, Result};

/// Information about the device hardware and operating system.
///
/// Use [`DeviceInfo::get()`] to query the running system, or construct
/// manually for testing.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// System-on-chip identifier (e.g. `"sg2002"`).
    pub soc: String,
    /// Operating system version string.
    pub os_version: String,
    /// Total physical memory in kilobytes.
    pub total_memory_kb: u64,
    /// Available physical memory in kilobytes.
    pub available_memory_kb: u64,
}

impl DeviceInfo {
    /// Query the running system for device information.
    ///
    /// Reads `/proc/meminfo` for memory statistics and `/etc/os-release` for
    /// the OS version. On non-Linux platforms (or when the files are absent)
    /// the fields gracefully fall back to default values.
    pub fn get() -> Result<Self> {
        let meminfo = fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let total_memory_kb = parse_meminfo_field(&meminfo, "MemTotal");
        let available_memory_kb = parse_meminfo_field(&meminfo, "MemAvailable");

        let os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();
        let os_version = os_release
            .lines()
            .find_map(|line| {
                let stripped = line.strip_prefix("PRETTY_NAME=")?;
                Some(stripped.trim_matches('"').to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());

        // Attempt to detect SoC from /proc/device-tree/model or fall back.
        let soc = fs::read_to_string("/proc/device-tree/model")
            .map(|s| s.trim_matches('\0').trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(Self {
            soc,
            os_version,
            total_memory_kb,
            available_memory_kb,
        })
    }
}

/// Parse a numeric kB field from `/proc/meminfo` content.
///
/// Returns `0` if the field is not found or cannot be parsed.
fn parse_meminfo_field(meminfo: &str, field: &str) -> u64 {
    meminfo
        .lines()
        .find_map(|line| {
            let rest = line.strip_prefix(field)?.trim_start();
            let rest = rest.strip_prefix(':')?.trim();
            // Value is like "1234 kB"
            rest.split_whitespace().next()?.parse::<u64>().ok()
        })
        .unwrap_or(0)
}

/// Controls a single LED exposed through the Linux sysfs interface.
///
/// Each LED is represented by a brightness file at
/// `/sys/class/leds/{name}/brightness`.
#[derive(Debug)]
pub struct Led {
    /// Path to the brightness sysfs file.
    path: PathBuf,
}

impl Led {
    /// Open an LED by its sysfs name.
    ///
    /// The LED must exist at `/sys/class/leds/{name}/brightness`.
    ///
    /// # Errors
    ///
    /// Returns an error if the brightness file does not exist.
    pub fn new(name: &str) -> Result<Self> {
        let path = PathBuf::from(format!("/sys/class/leds/{name}/brightness"));
        if !path.exists() {
            return Err(Error::System(format!("LED not found: {name}")));
        }
        Ok(Self { path })
    }

    /// Create an `Led` with an arbitrary file path.
    ///
    /// This is primarily useful for testing without real sysfs entries.
    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// Set the LED brightness.
    ///
    /// Writes the decimal value to the sysfs brightness file.
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails (e.g. permission denied).
    pub fn set_brightness(&self, value: u32) -> Result<()> {
        fs::write(&self.path, value.to_string())
            .map_err(|e| Error::System(format!("failed to set LED brightness: {e}")))
    }

    /// Read the current LED brightness.
    ///
    /// Reads and parses the decimal value from the sysfs brightness file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or the content is not a
    /// valid integer.
    pub fn get_brightness(&self) -> Result<u32> {
        let content = fs::read_to_string(&self.path)
            .map_err(|e| Error::System(format!("failed to read LED brightness: {e}")))?;
        content
            .trim()
            .parse::<u32>()
            .map_err(|e| Error::System(format!("invalid brightness value: {e}")))
    }
}

/// Return the system uptime in seconds.
///
/// Reads `/proc/uptime` and returns the first field (seconds since boot).
///
/// # Errors
///
/// Returns an error if `/proc/uptime` cannot be read or parsed.
pub fn uptime() -> Result<f64> {
    let content = fs::read_to_string("/proc/uptime")
        .map_err(|e| Error::System(format!("failed to read /proc/uptime: {e}")))?;
    content
        .split_whitespace()
        .next()
        .ok_or_else(|| Error::System("empty /proc/uptime".to_string()))?
        .parse::<f64>()
        .map_err(|e| Error::System(format!("invalid uptime value: {e}")))
}

/// Reboot the system.
///
/// Executes the `reboot` command. This requires appropriate privileges
/// (typically root).
///
/// # Errors
///
/// Returns an error if the reboot command cannot be executed.
pub fn reboot() -> Result<()> {
    let status = std::process::Command::new("reboot")
        .status()
        .map_err(|e| Error::System(format!("failed to execute reboot: {e}")))?;
    if !status.success() {
        return Err(Error::System(format!(
            "reboot exited with status: {status}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_meminfo_total() {
        let meminfo = "\
MemTotal:        1024000 kB
MemFree:          512000 kB
MemAvailable:     768000 kB
";
        assert_eq!(parse_meminfo_field(meminfo, "MemTotal"), 1_024_000);
        assert_eq!(parse_meminfo_field(meminfo, "MemAvailable"), 768_000);
        assert_eq!(parse_meminfo_field(meminfo, "MemFree"), 512_000);
    }

    #[test]
    fn parse_meminfo_missing_field() {
        let meminfo = "MemTotal:        1024000 kB\n";
        assert_eq!(parse_meminfo_field(meminfo, "MemAvailable"), 0);
        assert_eq!(parse_meminfo_field(meminfo, ""), 0);
    }

    #[test]
    fn parse_meminfo_empty() {
        assert_eq!(parse_meminfo_field("", "MemTotal"), 0);
    }

    #[test]
    fn led_set_get_brightness() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("brightness");
        // Create the file
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"0").unwrap();
        }

        let led = Led::with_path(path);
        assert_eq!(led.get_brightness().unwrap(), 0);

        led.set_brightness(255).unwrap();
        assert_eq!(led.get_brightness().unwrap(), 255);

        led.set_brightness(0).unwrap();
        assert_eq!(led.get_brightness().unwrap(), 0);
    }

    #[test]
    fn led_new_missing() {
        let result = Led::new("nonexistent_led_that_does_not_exist");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("LED not found"));
    }

    #[test]
    fn device_info_get_does_not_panic() {
        // On non-Linux this still works via graceful fallback.
        let info = DeviceInfo::get().unwrap();
        // Just verify the struct was populated (values depend on platform).
        assert!(!info.os_version.is_empty());
        assert!(!info.soc.is_empty());
    }
}
