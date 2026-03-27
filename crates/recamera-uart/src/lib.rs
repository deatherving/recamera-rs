//! UART / serial communication for the recamera SDK.
//!
//! This crate provides a thin wrapper around the [`serialport`] crate, exposing
//! configuration types ([`UartConfig`], [`DataBits`], [`Parity`], [`StopBits`])
//! and a [`Uart`] handle that implements [`std::io::Read`] and
//! [`std::io::Write`].

use std::io;
use std::time::Duration;

use recamera_core::{Error, Result};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Number of data bits per character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBits {
    /// Five data bits.
    Five,
    /// Six data bits.
    Six,
    /// Seven data bits.
    Seven,
    /// Eight data bits.
    Eight,
}

impl From<DataBits> for serialport::DataBits {
    fn from(value: DataBits) -> Self {
        match value {
            DataBits::Five => serialport::DataBits::Five,
            DataBits::Six => serialport::DataBits::Six,
            DataBits::Seven => serialport::DataBits::Seven,
            DataBits::Eight => serialport::DataBits::Eight,
        }
    }
}

/// Parity checking mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    /// No parity bit.
    None,
    /// Odd parity.
    Odd,
    /// Even parity.
    Even,
}

impl From<Parity> for serialport::Parity {
    fn from(value: Parity) -> Self {
        match value {
            Parity::None => serialport::Parity::None,
            Parity::Odd => serialport::Parity::Odd,
            Parity::Even => serialport::Parity::Even,
        }
    }
}

/// Number of stop bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopBits {
    /// One stop bit.
    One,
    /// Two stop bits.
    Two,
}

impl From<StopBits> for serialport::StopBits {
    fn from(value: StopBits) -> Self {
        match value {
            StopBits::One => serialport::StopBits::One,
            StopBits::Two => serialport::StopBits::Two,
        }
    }
}

// ---------------------------------------------------------------------------
// UartConfig
// ---------------------------------------------------------------------------

/// Configuration for a UART serial port connection.
///
/// Use the [`Default`] implementation for the most common settings:
/// 115 200 baud, 8 data bits, no parity, 1 stop bit, 1 000 ms timeout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UartConfig {
    /// Baud rate in bits per second.
    pub baud_rate: u32,
    /// Number of data bits per character.
    pub data_bits: DataBits,
    /// Parity checking mode.
    pub parity: Parity,
    /// Number of stop bits.
    pub stop_bits: StopBits,
    /// Read/write timeout.
    pub timeout: Duration,
}

impl Default for UartConfig {
    fn default() -> Self {
        Self {
            baud_rate: 115_200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_millis(1000),
        }
    }
}

// ---------------------------------------------------------------------------
// Uart
// ---------------------------------------------------------------------------

/// A UART serial port handle.
///
/// Wraps a [`serialport::SerialPort`] trait object and exposes standard
/// [`io::Read`] and [`io::Write`] implementations so it can be used with
/// generic I/O combinators.
pub struct Uart {
    port: Box<dyn serialport::SerialPort>,
}

impl std::fmt::Debug for Uart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Uart")
            .field("name", &self.port.name())
            .finish()
    }
}

impl Uart {
    /// Open a serial port at `path` with the given [`UartConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Serial`] if the port cannot be opened (e.g. the
    /// device path does not exist or the user lacks permissions).
    pub fn open(path: &str, config: &UartConfig) -> Result<Self> {
        let port = serialport::new(path, config.baud_rate)
            .data_bits(config.data_bits.into())
            .parity(config.parity.into())
            .stop_bits(config.stop_bits.into())
            .timeout(config.timeout)
            .open()
            .map_err(|e| Error::Serial(e.to_string()))?;
        Ok(Self { port })
    }
}

impl io::Read for Uart {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.port.read(buf)
    }
}

impl io::Write for Uart {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.port.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.port.flush()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = UartConfig::default();
        assert_eq!(cfg.baud_rate, 115_200);
        assert_eq!(cfg.data_bits, DataBits::Eight);
        assert_eq!(cfg.parity, Parity::None);
        assert_eq!(cfg.stop_bits, StopBits::One);
        assert_eq!(cfg.timeout, Duration::from_millis(1000));
    }

    #[test]
    fn data_bits_conversion() {
        assert_eq!(
            serialport::DataBits::Five,
            serialport::DataBits::from(DataBits::Five)
        );
        assert_eq!(
            serialport::DataBits::Six,
            serialport::DataBits::from(DataBits::Six)
        );
        assert_eq!(
            serialport::DataBits::Seven,
            serialport::DataBits::from(DataBits::Seven)
        );
        assert_eq!(
            serialport::DataBits::Eight,
            serialport::DataBits::from(DataBits::Eight)
        );
    }

    #[test]
    fn parity_conversion() {
        assert_eq!(
            serialport::Parity::None,
            serialport::Parity::from(Parity::None)
        );
        assert_eq!(
            serialport::Parity::Odd,
            serialport::Parity::from(Parity::Odd)
        );
        assert_eq!(
            serialport::Parity::Even,
            serialport::Parity::from(Parity::Even)
        );
    }

    #[test]
    fn stop_bits_conversion() {
        assert_eq!(
            serialport::StopBits::One,
            serialport::StopBits::from(StopBits::One)
        );
        assert_eq!(
            serialport::StopBits::Two,
            serialport::StopBits::from(StopBits::Two)
        );
    }

    #[test]
    fn open_nonexistent_port_fails() {
        let cfg = UartConfig::default();
        let result = Uart::open("/dev/ttyNONEXISTENT_12345", &cfg);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Serial(_)));
    }

    #[test]
    fn config_clone_and_debug() {
        let cfg = UartConfig::default();
        let cfg2 = cfg.clone();
        assert_eq!(cfg, cfg2);
        // Ensure Debug is implemented
        let debug_str = format!("{:?}", cfg);
        assert!(debug_str.contains("UartConfig"));
    }

    #[test]
    fn enum_debug_impls() {
        assert_eq!(format!("{:?}", DataBits::Eight), "Eight");
        assert_eq!(format!("{:?}", Parity::None), "None");
        assert_eq!(format!("{:?}", StopBits::One), "One");
    }
}
