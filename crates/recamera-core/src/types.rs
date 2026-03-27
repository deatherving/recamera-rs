//! Common types shared across the recamera SDK.

/// Pixel / encoding format of an image or video frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// 24-bit RGB, 8 bits per channel, packed.
    Rgb888,
    /// JPEG-compressed image.
    Jpeg,
    /// H.264 encoded video frame.
    H264,
    /// YUV 4:2:0 semi-planar (NV21) format, commonly used on embedded cameras.
    Nv21,
}

/// A width/height pair describing an image or video resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Resolution {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Resolution {
    /// Create a new [`Resolution`] with the given dimensions.
    ///
    /// This is a `const fn` so it can be used in constant contexts.
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Raw frame data captured from a camera or produced by processing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameData {
    /// The raw pixel or encoded bytes.
    pub data: Vec<u8>,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// The encoding format of [`data`](Self::data).
    pub format: ImageFormat,
    /// Capture timestamp in milliseconds since an unspecified epoch.
    pub timestamp_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_format_clone_copy_eq() {
        let a = ImageFormat::Jpeg;
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(ImageFormat::Rgb888, ImageFormat::Nv21);
    }

    #[test]
    fn image_format_debug() {
        // Ensure Debug is implemented and produces something sensible.
        let dbg = format!("{:?}", ImageFormat::H264);
        assert_eq!(dbg, "H264");
    }

    #[test]
    fn resolution_new_const() {
        const RES: Resolution = Resolution::new(1920, 1080);
        assert_eq!(RES.width, 1920);
        assert_eq!(RES.height, 1080);
    }

    #[test]
    fn resolution_equality() {
        let a = Resolution::new(640, 480);
        let b = Resolution::new(640, 480);
        let c = Resolution::new(320, 240);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn frame_data_construction() {
        let frame = FrameData {
            data: vec![0u8; 640 * 480 * 3],
            width: 640,
            height: 480,
            format: ImageFormat::Rgb888,
            timestamp_ms: 1234,
        };
        assert_eq!(frame.data.len(), 640 * 480 * 3);
        assert_eq!(frame.width, 640);
        assert_eq!(frame.height, 480);
        assert_eq!(frame.format, ImageFormat::Rgb888);
        assert_eq!(frame.timestamp_ms, 1234);
    }

    #[test]
    fn frame_data_clone() {
        let frame = FrameData {
            data: vec![1, 2, 3],
            width: 1,
            height: 1,
            format: ImageFormat::Jpeg,
            timestamp_ms: 0,
        };
        let cloned = frame.clone();
        assert_eq!(frame, cloned);
    }
}
