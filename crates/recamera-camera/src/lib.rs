//! Camera capture for the recamera SDK.
//!
//! This crate wraps the CVI MPI video pipeline (Sensor -> VI -> ISP -> VPSS ->
//! VENC) and exposes a safe Rust API for configuring the camera, starting and
//! stopping video streams, and capturing individual frames.
//!
//! # Example
//!
//! ```rust,no_run
//! use recamera_camera::{Camera, CameraConfig};
//!
//! let mut camera = Camera::new(CameraConfig::default()).unwrap();
//! camera.start_stream().unwrap();
//! let frame = camera.capture().unwrap();
//! println!("Captured {}x{} frame, {} bytes", frame.width(), frame.height(), frame.as_bytes().len());
//! camera.stop_stream().unwrap();
//! ```

use std::mem::MaybeUninit;
use std::sync::Arc;
use std::thread::JoinHandle;

use recamera_core::{Error, FrameData, ImageFormat, Resolution, Result};
use recamera_cvi_sys::CviLibs;

/// Video channel selector.
///
/// Each channel corresponds to a VPSS output group on the CVI pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Channel {
    /// CH0 -- raw RGB888 output from the ISP/VPSS.
    Raw,
    /// CH1 -- JPEG-compressed output from VENC.
    Jpeg,
    /// CH2 -- H.264-encoded video stream from VENC.
    H264,
}

impl Channel {
    /// Returns the VPSS channel index for this channel.
    fn vpss_chn(&self) -> i32 {
        match self {
            Channel::Raw => 0,
            Channel::Jpeg => 1,
            Channel::H264 => 2,
        }
    }

    /// Returns the image format for this channel.
    fn image_format(&self) -> ImageFormat {
        match self {
            Channel::Raw => ImageFormat::Rgb888,
            Channel::Jpeg => ImageFormat::Jpeg,
            Channel::H264 => ImageFormat::H264,
        }
    }
}

/// Configuration for the camera pipeline.
///
/// Use the [`Default`] implementation for a sensible starting point
/// (1920x1080, 30 fps, JPEG channel). Can be loaded from a TOML file
/// when the `serde` feature is enabled.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
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

/// Check a CVI return code, converting non-zero to an error.
fn check_cvi(rc: i32, context: &str) -> Result<()> {
    if rc == 0 {
        Ok(())
    } else {
        Err(Error::Camera(format!("{context} failed (rc={rc})")))
    }
}

/// Camera handle that wraps the CVI MPI video pipeline.
///
/// Manages the full Sensor -> VI -> ISP -> VPSS -> VENC pipeline.
/// The vendor shared libraries are loaded at runtime on the reCamera device.
///
/// Create one with [`Camera::new`], then call [`Camera::start_stream`] before
/// capturing frames with [`Camera::capture`].
pub struct Camera {
    config: CameraConfig,
    libs: Arc<CviLibs>,
    streaming: bool,
    isp_thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for Camera {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Camera")
            .field("config", &self.config)
            .field("streaming", &self.streaming)
            .finish()
    }
}

impl Camera {
    /// Create a new camera handle with the given configuration.
    ///
    /// Loads the CVI vendor libraries and initializes the system and video
    /// buffer pools. This must be called on the reCamera device where the
    /// vendor `.so` files are installed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Camera`] if the vendor libraries cannot be loaded
    /// or if system initialization fails.
    pub fn new(config: CameraConfig) -> Result<Self> {
        let libs = CviLibs::load()
            .map_err(|e| Error::Camera(format!("failed to load CVI libraries: {e}")))?;

        // Initialize VB and SYS — match the C++ SDK init sequence exactly.
        unsafe {
            // 1. Clean up any prior state (prevents hangs after crashes)
            let _ = libs.cvi_sys_exit();
            let _ = libs.cvi_vb_exit();

            // 2. Set number of VI devices (C++ does this before VB init)
            let rc = libs
                .cvi_vi_set_dev_num(1)
                .map_err(|e| Error::Camera(format!("VI_SetDevNum symbol: {e}")))?;
            check_cvi(rc, "CVI_VI_SetDevNum")?;

            // 3. Configure and init VB pools
            let mut vb_config: recamera_cvi_sys::VB_CONFIG_S = std::mem::zeroed();
            vb_config.u32MaxPoolCnt = 1;
            vb_config.astCommPool[0].u32BlkSize =
                config.resolution.width * config.resolution.height * 3; // RGB888 worst case
            vb_config.astCommPool[0].u32BlkCnt = 4;

            let rc = libs
                .cvi_vb_set_config(&vb_config)
                .map_err(|e| Error::Camera(format!("VB_SetConfig symbol: {e}")))?;
            check_cvi(rc, "CVI_VB_SetConfig")?;

            let rc = libs
                .cvi_vb_init()
                .map_err(|e| Error::Camera(format!("VB_Init symbol: {e}")))?;
            check_cvi(rc, "CVI_VB_Init")?;

            let rc = libs
                .cvi_sys_init()
                .map_err(|e| Error::Camera(format!("SYS_Init symbol: {e}")))?;
            check_cvi(rc, "CVI_SYS_Init")?;

            // 4. Set VI-VPSS mode: VI_OFFLINE_VPSS_ONLINE (matches C++ default)
            let mut vi_vpss_mode: recamera_cvi_sys::VI_VPSS_MODE_S = std::mem::zeroed();
            vi_vpss_mode.aenMode[0] =
                recamera_cvi_sys::VI_VPSS_MODE_E::VI_OFFLINE_VPSS_ONLINE;

            let rc = libs
                .cvi_sys_set_vi_vpss_mode(&vi_vpss_mode)
                .map_err(|e| Error::Camera(format!("SYS_SetVIVPSSMode symbol: {e}")))?;
            check_cvi(rc, "CVI_SYS_SetVIVPSSMode")?;

            // 5. Set VPSS mode: SINGLE with ISP input (matches C++ default)
            let mut vpss_mode: recamera_cvi_sys::VPSS_MODE_S = std::mem::zeroed();
            vpss_mode.enMode = recamera_cvi_sys::VPSS_MODE_E::VPSS_MODE_SINGLE;
            vpss_mode.aenInput[0] = recamera_cvi_sys::VPSS_INPUT_E::VPSS_INPUT_ISP;

            let rc = libs
                .cvi_sys_set_vpss_mode_ex(&vpss_mode)
                .map_err(|e| Error::Camera(format!("SYS_SetVPSSModeEx symbol: {e}")))?;
            check_cvi(rc, "CVI_SYS_SetVPSSModeEx")?;
        }

        Ok(Self {
            config,
            libs: Arc::new(libs),
            streaming: false,
            isp_thread: None,
        })
    }

    /// Names of sensor driver objects to probe, in priority order.
    const SENSOR_OBJS: &[&[u8]] = &[
        b"stSnsGc2053_Obj\0",
        b"stSnsOv5647_Obj\0",
    ];

    /// Try each known sensor object until one probes successfully.
    unsafe fn probe_sensor(
        libs: &CviLibs,
        pipe: i32,
        sns_cfg: &mut recamera_cvi_sys::ISP_SNS_CFG_S,
    ) -> Result<*mut recamera_cvi_sys::CVI_VOID> {
        for name in Self::SENSOR_OBJS {
            let sns_obj = match unsafe { libs.get_sensor_obj(name) } {
                Ok(ptr) => ptr,
                Err(_) => continue,
            };
            let rc = unsafe { libs.cvi_isp_sns_init(pipe, sns_cfg, sns_obj, 0) }
                .map_err(|e| Error::Camera(format!("ISP_SnsInit symbol: {e}")))?;
            if rc == 0 {
                let name_str = std::str::from_utf8(&name[..name.len() - 1]).unwrap_or("?");
                eprintln!("recamera: detected sensor {name_str}");
                return Ok(sns_obj);
            }
        }
        Err(Error::Camera(
            "no supported sensor detected (tried GC2053, OV5647)".into(),
        ))
    }

    /// Start the video stream.
    ///
    /// Configures VI, VPSS, and VENC channels and begins capture.
    /// After this call, [`Camera::capture`] can be used to retrieve frames.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Camera`] if any pipeline stage fails to start.
    pub fn start_stream(&mut self) -> Result<()> {
        if self.streaming {
            return Ok(());
        }

        let w = self.config.resolution.width;
        let h = self.config.resolution.height;

        unsafe {
            // Setup VI device 0, pipe 0, channel 0
            let mut vi_dev_attr: recamera_cvi_sys::VI_DEV_ATTR_S = std::mem::zeroed();
            vi_dev_attr.stSize.u32Width = w;
            vi_dev_attr.stSize.u32Height = h;

            let rc = self
                .libs
                .cvi_vi_set_dev_attr(0, &vi_dev_attr)
                .map_err(|e| Error::Camera(format!("VI_SetDevAttr symbol: {e}")))?;
            check_cvi(rc, "CVI_VI_SetDevAttr")?;

            let rc = self
                .libs
                .cvi_vi_enable_dev(0)
                .map_err(|e| Error::Camera(format!("VI_EnableDev symbol: {e}")))?;
            check_cvi(rc, "CVI_VI_EnableDev")?;

            // -- VI Pipe --
            let mut vi_pipe_attr: recamera_cvi_sys::VI_PIPE_ATTR_S = std::mem::zeroed();
            vi_pipe_attr.u32MaxW = w;
            vi_pipe_attr.u32MaxH = h;
            vi_pipe_attr.enPixFmt = recamera_cvi_sys::PIXEL_FORMAT_E::PIXEL_FORMAT_NV21;

            let rc = self
                .libs
                .cvi_vi_create_pipe(0, &vi_pipe_attr)
                .map_err(|e| Error::Camera(format!("VI_CreatePipe symbol: {e}")))?;
            check_cvi(rc, "CVI_VI_CreatePipe")?;

            let rc = self
                .libs
                .cvi_vi_start_pipe(0)
                .map_err(|e| Error::Camera(format!("VI_StartPipe symbol: {e}")))?;
            check_cvi(rc, "CVI_VI_StartPipe")?;

            // -- ISP --
            let mut isp_pub_attr: recamera_cvi_sys::ISP_PUB_ATTR_S = std::mem::zeroed();
            isp_pub_attr.stWndRect.u32Width = w;
            isp_pub_attr.stWndRect.u32Height = h;
            isp_pub_attr.stSnsSize.u32Width = w;
            isp_pub_attr.stSnsSize.u32Height = h;
            isp_pub_attr.f32FrameRate = self.config.fps as f32;
            isp_pub_attr.enBayer = recamera_cvi_sys::ISP_BAYER_FORMAT_E::BAYER_FORMAT_BG;
            isp_pub_attr.enWDRMode = recamera_cvi_sys::WDR_MODE_E::WDR_MODE_NONE;
            isp_pub_attr.u8SnsMode = 0;

            let rc = self
                .libs
                .cvi_isp_set_pub_attr(0, &isp_pub_attr)
                .map_err(|e| Error::Camera(format!("ISP_SetPubAttr symbol: {e}")))?;
            check_cvi(rc, "CVI_ISP_SetPubAttr")?;

            let rc = self
                .libs
                .cvi_isp_mem_init(0)
                .map_err(|e| Error::Camera(format!("ISP_MemInit symbol: {e}")))?;
            check_cvi(rc, "CVI_ISP_MemInit")?;

            // -- Sensor probe (auto-detect) --
            let mut sns_cfg: recamera_cvi_sys::ISP_SNS_CFG_S = std::mem::zeroed();
            sns_cfg.stSnsSize.u32Width = w;
            sns_cfg.stSnsSize.u32Height = h;
            sns_cfg.f32FrameRate = self.config.fps as f32;
            sns_cfg.enWDRMode = recamera_cvi_sys::WDR_MODE_E::WDR_MODE_NONE;
            sns_cfg.S32MipiDevno = 0;
            sns_cfg.u8Mclk = 0;
            sns_cfg.bMclkEn = 1; // CVI_TRUE
            sns_cfg.lane_id = [-1, -1, -1, -1, -1]; // sensor will patch
            sns_cfg.pn_swap = [0, 0, 0, 0, 0];
            sns_cfg.busInfo = recamera_cvi_sys::ISP_SNS_COMMBUS_U { s8I2cDev: 0 };
            sns_cfg.I2cAddr = -1; // sensor default

            Self::probe_sensor(&self.libs, 0, &mut sns_cfg)?;

            // -- ISP Init + Run --
            let rc = self
                .libs
                .cvi_isp_init(0)
                .map_err(|e| Error::Camera(format!("ISP_Init symbol: {e}")))?;
            check_cvi(rc, "CVI_ISP_Init")?;

            // CVI_ISP_Run blocks — run in a dedicated thread
            let isp_libs = Arc::clone(&self.libs);
            self.isp_thread = Some(std::thread::spawn(move || {
                let _ = isp_libs.cvi_isp_run(0);
            }));

            let mut vi_chn_attr: recamera_cvi_sys::VI_CHN_ATTR_S = std::mem::zeroed();
            vi_chn_attr.stSize.u32Width = w;
            vi_chn_attr.stSize.u32Height = h;
            vi_chn_attr.enPixelFormat = recamera_cvi_sys::PIXEL_FORMAT_E::PIXEL_FORMAT_NV21;

            let rc = self
                .libs
                .cvi_vi_set_chn_attr(0, 0, &mut vi_chn_attr)
                .map_err(|e| Error::Camera(format!("VI_SetChnAttr symbol: {e}")))?;
            check_cvi(rc, "CVI_VI_SetChnAttr")?;

            let rc = self
                .libs
                .cvi_vi_enable_chn(0, 0)
                .map_err(|e| Error::Camera(format!("VI_EnableChn symbol: {e}")))?;
            check_cvi(rc, "CVI_VI_EnableChn")?;

            // Setup VPSS group 0, channel based on config
            let mut vpss_grp_attr: recamera_cvi_sys::VPSS_GRP_ATTR_S = std::mem::zeroed();
            vpss_grp_attr.u32MaxW = w;
            vpss_grp_attr.u32MaxH = h;
            vpss_grp_attr.enPixelFormat = recamera_cvi_sys::PIXEL_FORMAT_E::PIXEL_FORMAT_NV21;

            let rc = self
                .libs
                .cvi_vpss_create_grp(0, &vpss_grp_attr)
                .map_err(|e| Error::Camera(format!("VPSS_CreateGrp symbol: {e}")))?;
            check_cvi(rc, "CVI_VPSS_CreateGrp")?;

            let vpss_chn = self.config.channel.vpss_chn();
            let mut vpss_chn_attr: recamera_cvi_sys::VPSS_CHN_ATTR_S = std::mem::zeroed();
            vpss_chn_attr.u32Width = w;
            vpss_chn_attr.u32Height = h;
            vpss_chn_attr.enPixelFormat = match self.config.channel {
                Channel::Raw => recamera_cvi_sys::PIXEL_FORMAT_E::PIXEL_FORMAT_RGB_888,
                _ => recamera_cvi_sys::PIXEL_FORMAT_E::PIXEL_FORMAT_NV21,
            };

            let rc = self
                .libs
                .cvi_vpss_set_chn_attr(0, vpss_chn, &vpss_chn_attr)
                .map_err(|e| Error::Camera(format!("VPSS_SetChnAttr symbol: {e}")))?;
            check_cvi(rc, "CVI_VPSS_SetChnAttr")?;

            let rc = self
                .libs
                .cvi_vpss_enable_chn(0, vpss_chn)
                .map_err(|e| Error::Camera(format!("VPSS_EnableChn symbol: {e}")))?;
            check_cvi(rc, "CVI_VPSS_EnableChn")?;

            let rc = self
                .libs
                .cvi_vpss_start_grp(0)
                .map_err(|e| Error::Camera(format!("VPSS_StartGrp symbol: {e}")))?;
            check_cvi(rc, "CVI_VPSS_StartGrp")?;
        }

        self.streaming = true;
        Ok(())
    }

    /// Stop the video stream.
    ///
    /// Tears down the VI/VPSS pipeline. After this call, [`Camera::capture`]
    /// will return an error until [`Camera::start_stream`] is called again.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Camera`] if any pipeline stage fails to stop.
    pub fn stop_stream(&mut self) -> Result<()> {
        if !self.streaming {
            return Ok(());
        }

        unsafe {
            // 1. Stop ISP first (signals CVI_ISP_Run to return)
            let _ = self.libs.cvi_isp_exit(0);
        }

        // 2. Join the ISP thread
        if let Some(handle) = self.isp_thread.take() {
            let _ = handle.join();
        }

        let vpss_chn = self.config.channel.vpss_chn();

        unsafe {
            // 3. Tear down VPSS
            let _ = self.libs.cvi_vpss_stop_grp(0);
            let _ = self.libs.cvi_vpss_disable_chn(0, vpss_chn);
            let _ = self.libs.cvi_vpss_destroy_grp(0);

            // 4. Tear down VI channel
            let _ = self.libs.cvi_vi_disable_chn(0, 0);

            // 5. Tear down VI pipe
            let _ = self.libs.cvi_vi_stop_pipe(0);
            let _ = self.libs.cvi_vi_destroy_pipe(0);

            // 6. Tear down VI device
            let _ = self.libs.cvi_vi_disable_dev(0);
        }

        self.streaming = false;
        Ok(())
    }

    /// Capture a single frame from the active stream.
    ///
    /// Retrieves a frame from the VPSS channel, copies the pixel data, and
    /// releases the hardware buffer.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Camera`] if the camera is not streaming or if the
    /// frame cannot be retrieved.
    pub fn capture(&self) -> Result<Frame> {
        if !self.streaming {
            return Err(Error::Camera("camera is not streaming".into()));
        }

        let vpss_chn = self.config.channel.vpss_chn();
        let mut frame_info = MaybeUninit::<recamera_cvi_sys::VIDEO_FRAME_INFO_S>::zeroed();

        unsafe {
            let frame_ptr = frame_info.as_mut_ptr();

            let rc = self
                .libs
                .cvi_vpss_get_chn_frame(0, vpss_chn, frame_ptr, 1000)
                .map_err(|e| Error::Camera(format!("VPSS_GetChnFrame symbol: {e}")))?;
            check_cvi(rc, "CVI_VPSS_GetChnFrame")?;

            let frame_info = frame_info.assume_init();
            let vframe = &frame_info.stVFrame;

            // Copy frame data from the hardware buffer
            let data_len = vframe.u32Length[0] as usize
                + vframe.u32Length[1] as usize
                + vframe.u32Length[2] as usize;

            let data = if !vframe.pu8VirAddr[0].is_null() && data_len > 0 {
                std::slice::from_raw_parts(vframe.pu8VirAddr[0], data_len).to_vec()
            } else {
                Vec::new()
            };

            // Release the frame back to the hardware
            let _ = self
                .libs
                .cvi_vpss_release_chn_frame(0, vpss_chn, &frame_info);

            Ok(Frame {
                data: FrameData {
                    data,
                    width: vframe.u32Width,
                    height: vframe.u32Height,
                    format: self.config.channel.image_format(),
                    timestamp_ms: vframe.u64PTS,
                },
            })
        }
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

impl Drop for Camera {
    fn drop(&mut self) {
        if self.streaming {
            let _ = self.stop_stream();
        }
        unsafe {
            let _ = self.libs.cvi_sys_exit();
            let _ = self.libs.cvi_vb_exit();
        }
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
    fn channel_vpss_mapping() {
        assert_eq!(Channel::Raw.vpss_chn(), 0);
        assert_eq!(Channel::Jpeg.vpss_chn(), 1);
        assert_eq!(Channel::H264.vpss_chn(), 2);
    }

    #[test]
    fn channel_image_format() {
        assert_eq!(Channel::Raw.image_format(), ImageFormat::Rgb888);
        assert_eq!(Channel::Jpeg.image_format(), ImageFormat::Jpeg);
        assert_eq!(Channel::H264.image_format(), ImageFormat::H264);
    }
}
