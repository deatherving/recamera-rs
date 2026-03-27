//! Camera capture for the recamera SDK.
//!
//! This crate wraps the CVI MPI video pipeline (Sensor -> VI -> ISP -> VPSS ->
//! VENC) and exposes a safe Rust API for configuring the camera, starting and
//! stopping video streams, and capturing individual frames.
//!
//! All hardware operations currently return an error because the CVI MPI FFI
//! bindings have not yet been generated. Once bindings are available the
//! implementation will initialise the pipeline and capture real frames.

use recamera_core::{Error, FrameData, ImageFormat, Resolution, Result};

/// Video channel selector.
///
/// Each channel corresponds to a VPSS output group on the CVI pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    /// CH0 — raw RGB888 output from the ISP/VPSS.
    Raw,
    /// CH1 — JPEG-compressed output from VENC.
    Jpeg,
    /// CH2 — H.264-encoded video stream from VENC.
    H264,
}

/// Configuration for the camera pipeline.
///
/// Use the [`Default`] implementation for a sensible starting point
/// (1920x1080, 30 fps, JPEG channel).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraConfig {
    /// Capture resolution (width x height in pixels).
    pub resolution: Resolution,
    /// Target frame rate in frames per second.
    pub fps: u32,
    /// Which video channel to capture from.
    pub channel: Channel,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            resolution: Resolution::new(1920, 1080),
            fps: 30,
            channel: Channel::Jpeg,
        }
    }
}

/// A single captured video frame.
///
/// Wraps a [`FrameData`] value from `recamera-core` and provides convenient
/// accessor methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// The underlying frame data.
    pub data: FrameData,
}

impl Frame {
    /// Frame width in pixels.
    pub fn width(&self) -> u32 {
        self.data.width
    }

    /// Frame height in pixels.
    pub fn height(&self) -> u32 {
        self.data.height
    }

    /// The pixel/encoding format of this frame.
    pub fn format(&self) -> ImageFormat {
        self.data.format
    }

    /// The raw bytes of the frame (pixel data or encoded bitstream).
    pub fn as_bytes(&self) -> &[u8] {
        &self.data.data
    }

    /// Capture timestamp in milliseconds since an unspecified epoch.
    pub fn timestamp_ms(&self) -> u64 {
        self.data.timestamp_ms
    }
}

/// Camera handle that wraps the CVI MPI video pipeline.
///
/// This struct will manage the full Sensor -> VI -> ISP -> VPSS -> VENC
/// pipeline once the CVI MPI FFI bindings are generated. Until then, all
/// hardware operations return an error.
///
/// Create one with [`Camera::new`], then call [`Camera::start_stream`] before
/// capturing frames with [`Camera::capture`].
#[derive(Debug)]
pub struct Camera {
    /// Current camera configuration.
    config: CameraConfig,
    /// Whether the camera is currently streaming.
    streaming: bool,
}

impl Camera {
    /// Create a new camera handle with the given configuration.
    ///
    /// Currently returns an error unconditionally because the CVI MPI FFI
    /// bindings have not yet been generated.
    pub fn new(_config: CameraConfig) -> Result<Self> {
        Err(Error::Camera(
            "not yet implemented: requires CVI MPI bindings".into(),
        ))
    }

    /// Start the video stream.
    ///
    /// After this call, [`Camera::capture`] can be used to retrieve frames.
    pub fn start_stream(&mut self) -> Result<()> {
        Err(Error::Camera(
            "not yet implemented: requires CVI MPI bindings".into(),
        ))
    }

    /// Stop the video stream.
    ///
    /// After this call, [`Camera::capture`] will return an error until
    /// [`Camera::start_stream`] is called again.
    pub fn stop_stream(&mut self) -> Result<()> {
        Err(Error::Camera(
            "not yet implemented: requires CVI MPI bindings".into(),
        ))
    }

    /// Capture a single frame from the active stream.
    ///
    /// Returns an error if the camera is not currently streaming.
    pub fn capture(&self) -> Result<Frame> {
        Err(Error::Camera(
            "not yet implemented: requires CVI MPI bindings".into(),
        ))
    }

    /// Returns a reference to the current camera configuration.
    pub fn config(&self) -> &CameraConfig {
        &self.config
    }

    /// Returns `true` if the camera is currently streaming.
    pub fn is_streaming(&self) -> bool {
        self.streaming
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_camera_config() {
        let cfg = CameraConfig::default();
        assert_eq!(cfg.resolution, Resolution::new(1920, 1080));
        assert_eq!(cfg.fps, 30);
        assert_eq!(cfg.channel, Channel::Jpeg);
    }

    #[test]
    fn frame_accessors() {
        let frame = Frame {
            data: FrameData {
                data: vec![0xAA, 0xBB, 0xCC],
                width: 640,
                height: 480,
                format: ImageFormat::Jpeg,
                timestamp_ms: 42,
            },
        };
        assert_eq!(frame.width(), 640);
        assert_eq!(frame.height(), 480);
        assert_eq!(frame.format(), ImageFormat::Jpeg);
        assert_eq!(frame.as_bytes(), &[0xAA, 0xBB, 0xCC]);
        assert_eq!(frame.timestamp_ms(), 42);
    }

    #[test]
    fn camera_new_returns_not_yet_implemented() {
        let result = Camera::new(CameraConfig::default());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("not yet implemented: requires CVI MPI bindings"),
            "unexpected error message: {err}"
        );
    }
}
