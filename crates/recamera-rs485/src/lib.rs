//! RS-485 helpers for the recamera SDK.
//!
//! This crate wraps a [`recamera_uart::Uart`] port with optional GPIO-based
//! direction control for half-duplex RS-485 communication. When a direction
//! GPIO is configured, the driver toggles the DE/RE pin via sysfs before
//! transmitting or receiving.

use recamera_core::{Error, Result};
use recamera_uart::Uart;
use std::io::{Read, Write};
use std::path::Path;

/// RS-485 configuration.
///
/// Controls optional GPIO-based direction switching for half-duplex RS-485
/// transceivers. When [`direction_gpio`](Rs485Config::direction_gpio) is set,
/// the driver writes `"1"` (transmit) or `"0"` (receive) to the given sysfs
/// path before each operation.
#[derive(Debug, Clone)]
pub struct Rs485Config {
    /// Path to the GPIO sysfs file for the DE/RE direction pin.
    ///
    /// For example, `"/sys/class/gpio/gpio42/value"`. When `None`, no
    /// direction switching is performed.
    pub direction_gpio: Option<String>,
}

impl Default for Rs485Config {
    fn default() -> Self {
        Self {
            direction_gpio: None,
        }
    }
}

/// RS-485 wrapper around a UART port.
///
/// Handles DE/RE direction pin toggling via GPIO sysfs if configured.
/// Use [`Rs485::send`] and [`Rs485::receive`] for half-duplex communication.
pub struct Rs485 {
    uart: Uart,
    config: Rs485Config,
}

impl Rs485 {
    /// Create a new RS-485 wrapper around the given UART port.
    ///
    /// The `config` controls optional direction-pin GPIO toggling.
    pub fn new(uart: Uart, config: Rs485Config) -> Self {
        Self { uart, config }
    }

    /// Set direction pin high (transmit mode).
    fn set_transmit(&self) -> Result<()> {
        if let Some(ref gpio) = self.config.direction_gpio {
            std::fs::write(Path::new(gpio), "1")
                .map_err(|e| Error::Serial(format!("failed to set TX direction: {e}")))?;
        }
        Ok(())
    }

    /// Set direction pin low (receive mode).
    fn set_receive(&self) -> Result<()> {
        if let Some(ref gpio) = self.config.direction_gpio {
            std::fs::write(Path::new(gpio), "0")
                .map_err(|e| Error::Serial(format!("failed to set RX direction: {e}")))?;
        }
        Ok(())
    }

    /// Send data over RS-485.
    ///
    /// Toggles the direction pin to transmit mode, writes all bytes, flushes
    /// the underlying UART, then switches back to receive mode.
    pub fn send(&mut self, data: &[u8]) -> Result<()> {
        self.set_transmit()?;
        self.uart
            .write_all(data)
            .map_err(|e| Error::Serial(format!("RS-485 write failed: {e}")))?;
        self.uart
            .flush()
            .map_err(|e| Error::Serial(format!("RS-485 flush failed: {e}")))?;
        self.set_receive()?;
        Ok(())
    }

    /// Receive data from RS-485.
    ///
    /// Ensures the direction pin is in receive mode, then reads available
    /// bytes into `buf`. Returns the number of bytes read.
    pub fn receive(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.set_receive()?;
        self.uart
            .read(buf)
            .map_err(|e| Error::Serial(format!("RS-485 read failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_no_gpio() {
        let config = Rs485Config::default();
        assert!(config.direction_gpio.is_none());
    }

    #[test]
    fn rs485_config_with_gpio() {
        let config = Rs485Config {
            direction_gpio: Some("/sys/class/gpio/gpio42/value".into()),
        };
        assert_eq!(
            config.direction_gpio.as_deref(),
            Some("/sys/class/gpio/gpio42/value")
        );
    }
}
