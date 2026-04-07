#![allow(unsafe_op_in_unsafe_fn)]
//! Runtime dynamic loader for CVI vendor shared libraries.
//!
//! The vendor `.so` libraries are loaded at runtime via `dlopen` with
//! `RTLD_LAZY | RTLD_GLOBAL`. This combination is essential on musl-based
//! systems (like reCamera-OS) where the vendor libraries were built for
//! glibc:
//!
//! - **`RTLD_LAZY`** -- only resolve symbols when actually called, avoiding
//!   failures from unused transitive dependencies.
//! - **`RTLD_GLOBAL`** -- make each library's symbols visible to all
//!   subsequently loaded libraries, so cross-library references (e.g.
//!   `libvi.so` using `log_levels` from `libsys.so`) resolve correctly.
//!
//! Known transitive dependencies like `libatomic.so` (byte-level atomics
//! on RISC-V) are preloaded before the vendor libraries.

use libloading::Library;
use libloading::os::unix::{Library as UnixLibrary, RTLD_GLOBAL, RTLD_LAZY};
use std::path::Path;

use crate::bindings::*;

/// Search paths tried when loading vendor libraries.
///
/// These are the standard locations where CVI shared objects are installed
/// on reCamera-OS.
const LIB_SEARCH_PATHS: &[&str] = &[
    "/usr/lib/",
    "/lib/",
    "/mnt/system/lib/",
    "/mnt/system/usr/lib/",
];

/// Holds loaded CVI vendor library handles.
///
/// Each field corresponds to one of the vendor shared libraries required
/// by the CVI multimedia pipeline. The libraries are loaded once via
/// [`CviLibs::load`] and remain mapped for the lifetime of this struct.
///
/// # Example
///
/// ```rust,no_run
/// use recamera_cvi_sys::CviLibs;
///
/// let libs = CviLibs::load().expect("failed to load CVI libraries");
/// unsafe {
///     let rc = libs.cvi_sys_init().expect("symbol lookup failed");
///     assert_eq!(rc, 0); // CVI_SUCCESS
/// }
/// ```
pub struct CviLibs {
    /// Handle to `libatomic.so` — provides byte/halfword atomic fallbacks
    /// on RISC-V. Loaded first with RTLD_GLOBAL so vendor libs can resolve
    /// `__atomic_compare_exchange_1` etc. `Option` because it may be absent.
    _atomic: Option<Library>,
    /// Handle to `libsys.so` (SYS and VB functions).
    sys: Library,
    /// Handle to `libvi.so` (video input functions).
    vi: Library,
    /// Handle to `libvpss.so` (video processing subsystem functions).
    vpss: Library,
    /// Handle to `libvenc.so` (video encoding functions).
    venc: Library,
    /// Handle to `libcviruntime.so` (NPU inference runtime).
    cviruntime: Library,
}

/// Try to load a shared library by name, searching [`LIB_SEARCH_PATHS`].
///
/// Libraries are opened with `RTLD_LAZY | RTLD_GLOBAL` so that:
/// - symbols are resolved on first use (not at load time), and
/// - each library's exports are visible to subsequently loaded libraries.
///
/// Returns the first successfully loaded library, or the error from the
/// last attempted path.
fn load_library(name: &str) -> Result<Library, libloading::Error> {
    let mut last_err = None;
    for dir in LIB_SEARCH_PATHS {
        let path = Path::new(dir).join(name);
        match unsafe { UnixLibrary::open(Some(&path), RTLD_LAZY | RTLD_GLOBAL) } {
            Ok(lib) => return Ok(lib.into()),
            Err(e) => last_err = Some(e),
        }
    }
    // If none of the paths worked, try the bare name and let the dynamic
    // linker resolve it via LD_LIBRARY_PATH / system defaults.
    unsafe { UnixLibrary::open(Some(Path::new(name)), RTLD_LAZY | RTLD_GLOBAL) }
        .map(Library::from)
        .map_err(|e| last_err.unwrap_or(e))
}

impl CviLibs {
    /// Load all CVI vendor libraries from the standard search paths.
    ///
    /// `libatomic.so` is loaded first (if available) to provide byte-level
    /// atomic operations needed by the vendor libs on RISC-V. Then the five
    /// vendor libraries are loaded in dependency order.
    ///
    /// # Errors
    ///
    /// Returns a [`libloading::Error`] if any of the five vendor libraries
    /// cannot be found or loaded.
    pub fn load() -> Result<Self, libloading::Error> {
        // Preload libatomic — provides __atomic_compare_exchange_1 etc.
        // needed by vendor libs on RISC-V. Missing on some hosts, so
        // failure is silently ignored.
        let _atomic = load_library("libatomic.so.1")
            .or_else(|_| load_library("libatomic.so"))
            .ok();

        Ok(Self {
            _atomic,
            sys: load_library("libsys.so")?,
            vi: load_library("libvi.so")?,
            vpss: load_library("libvpss.so")?,
            venc: load_library("libvenc.so")?,
            cviruntime: load_library("libcviruntime.so")?,
        })
    }

    // ---------------------------------------------------------------
    // SYS functions (from libsys.so)
    // ---------------------------------------------------------------

    /// Initialize the CVI system. Must be called before any other CVI API.
    ///
    /// # Safety
    ///
    /// Calls into a C shared library via `dlsym`. The caller must ensure the
    /// library is compatible with the running kernel and hardware.
    pub unsafe fn cvi_sys_init(&self) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn() -> CVI_S32> =
            self.sys.get(b"CVI_SYS_Init")?;
        Ok(func())
    }

    /// Shut down the CVI system.
    ///
    /// # Safety
    ///
    /// See [`CviLibs::cvi_sys_init`].
    pub unsafe fn cvi_sys_exit(&self) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn() -> CVI_S32> =
            self.sys.get(b"CVI_SYS_Exit")?;
        Ok(func())
    }

    /// Bind a source channel to a destination channel.
    ///
    /// # Safety
    ///
    /// Both pointers must be valid and point to initialized `MMF_CHN_S` structs.
    pub unsafe fn cvi_sys_bind(
        &self,
        src: *const MMF_CHN_S,
        dst: *const MMF_CHN_S,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(*const MMF_CHN_S, *const MMF_CHN_S) -> CVI_S32,
        > = self.sys.get(b"CVI_SYS_Bind")?;
        Ok(func(src, dst))
    }

    /// Unbind a previously bound source/destination channel pair.
    ///
    /// # Safety
    ///
    /// Both pointers must be valid and point to initialized `MMF_CHN_S` structs.
    pub unsafe fn cvi_sys_unbind(
        &self,
        src: *const MMF_CHN_S,
        dst: *const MMF_CHN_S,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(*const MMF_CHN_S, *const MMF_CHN_S) -> CVI_S32,
        > = self.sys.get(b"CVI_SYS_UnBind")?;
        Ok(func(src, dst))
    }

    // ---------------------------------------------------------------
    // VB functions (from libsys.so)
    // ---------------------------------------------------------------

    /// Initialize the video buffer pool.
    ///
    /// # Safety
    ///
    /// Must be called after [`CviLibs::cvi_vb_set_config`] and before
    /// allocating any video buffers.
    pub unsafe fn cvi_vb_init(&self) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn() -> CVI_S32> =
            self.sys.get(b"CVI_VB_Init")?;
        Ok(func())
    }

    /// Tear down the video buffer pool.
    ///
    /// # Safety
    ///
    /// All buffers must have been released before calling this.
    pub unsafe fn cvi_vb_exit(&self) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn() -> CVI_S32> =
            self.sys.get(b"CVI_VB_Exit")?;
        Ok(func())
    }

    /// Set the common video buffer pool configuration.
    ///
    /// # Safety
    ///
    /// `config` must point to a valid, initialized `VB_CONFIG_S`.
    pub unsafe fn cvi_vb_set_config(
        &self,
        config: *const VB_CONFIG_S,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(*const VB_CONFIG_S) -> CVI_S32> =
            self.sys.get(b"CVI_VB_SetConfig")?;
        Ok(func(config))
    }

    // ---------------------------------------------------------------
    // VI functions (from libvi.so)
    // ---------------------------------------------------------------

    /// Set video input device attributes.
    ///
    /// # Safety
    ///
    /// `attr` must point to a valid `VI_DEV_ATTR_S`.
    pub unsafe fn cvi_vi_set_dev_attr(
        &self,
        dev: VI_DEV,
        attr: *const VI_DEV_ATTR_S,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(VI_DEV, *const VI_DEV_ATTR_S) -> CVI_S32,
        > = self.vi.get(b"CVI_VI_SetDevAttr")?;
        Ok(func(dev, attr))
    }

    /// Enable a video input device.
    ///
    /// # Safety
    ///
    /// The device must have been configured via [`CviLibs::cvi_vi_set_dev_attr`].
    pub unsafe fn cvi_vi_enable_dev(&self, dev: VI_DEV) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(VI_DEV) -> CVI_S32> =
            self.vi.get(b"CVI_VI_EnableDev")?;
        Ok(func(dev))
    }

    /// Disable a video input device.
    ///
    /// # Safety
    ///
    /// The device must have been enabled first.
    pub unsafe fn cvi_vi_disable_dev(&self, dev: VI_DEV) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(VI_DEV) -> CVI_S32> =
            self.vi.get(b"CVI_VI_DisableDev")?;
        Ok(func(dev))
    }

    /// Set video input channel attributes.
    ///
    /// # Safety
    ///
    /// `attr` must point to a valid `VI_CHN_ATTR_S`.
    pub unsafe fn cvi_vi_set_chn_attr(
        &self,
        pipe: VI_PIPE,
        chn: VI_CHN,
        attr: *mut VI_CHN_ATTR_S,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(VI_PIPE, VI_CHN, *mut VI_CHN_ATTR_S) -> CVI_S32,
        > = self.vi.get(b"CVI_VI_SetChnAttr")?;
        Ok(func(pipe, chn, attr))
    }

    /// Enable a video input channel.
    ///
    /// # Safety
    ///
    /// The channel must have been configured first.
    pub unsafe fn cvi_vi_enable_chn(
        &self,
        pipe: VI_PIPE,
        chn: VI_CHN,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(VI_PIPE, VI_CHN) -> CVI_S32> =
            self.vi.get(b"CVI_VI_EnableChn")?;
        Ok(func(pipe, chn))
    }

    /// Disable a video input channel.
    ///
    /// # Safety
    ///
    /// The channel must have been enabled first.
    pub unsafe fn cvi_vi_disable_chn(
        &self,
        pipe: VI_PIPE,
        chn: VI_CHN,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(VI_PIPE, VI_CHN) -> CVI_S32> =
            self.vi.get(b"CVI_VI_DisableChn")?;
        Ok(func(pipe, chn))
    }

    /// Get a frame from a video input channel.
    ///
    /// # Safety
    ///
    /// `frame_info` must point to a valid `VIDEO_FRAME_INFO_S` that will be
    /// written to. The returned frame must later be released with
    /// [`CviLibs::cvi_vi_release_chn_frame`].
    pub unsafe fn cvi_vi_get_chn_frame(
        &self,
        pipe: VI_PIPE,
        chn: VI_CHN,
        frame_info: *mut VIDEO_FRAME_INFO_S,
        timeout_ms: CVI_S32,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(VI_PIPE, VI_CHN, *mut VIDEO_FRAME_INFO_S, CVI_S32) -> CVI_S32,
        > = self.vi.get(b"CVI_VI_GetChnFrame")?;
        Ok(func(pipe, chn, frame_info, timeout_ms))
    }

    /// Release a frame previously obtained from a video input channel.
    ///
    /// # Safety
    ///
    /// `frame_info` must point to a frame obtained from
    /// [`CviLibs::cvi_vi_get_chn_frame`].
    pub unsafe fn cvi_vi_release_chn_frame(
        &self,
        pipe: VI_PIPE,
        chn: VI_CHN,
        frame_info: *const VIDEO_FRAME_INFO_S,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(VI_PIPE, VI_CHN, *const VIDEO_FRAME_INFO_S) -> CVI_S32,
        > = self.vi.get(b"CVI_VI_ReleaseChnFrame")?;
        Ok(func(pipe, chn, frame_info))
    }

    // ---------------------------------------------------------------
    // VPSS functions (from libvpss.so)
    // ---------------------------------------------------------------

    /// Create a VPSS processing group.
    ///
    /// # Safety
    ///
    /// `attr` must point to a valid `VPSS_GRP_ATTR_S`.
    pub unsafe fn cvi_vpss_create_grp(
        &self,
        grp: VPSS_GRP,
        attr: *const VPSS_GRP_ATTR_S,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(VPSS_GRP, *const VPSS_GRP_ATTR_S) -> CVI_S32,
        > = self.vpss.get(b"CVI_VPSS_CreateGrp")?;
        Ok(func(grp, attr))
    }

    /// Destroy a VPSS processing group.
    ///
    /// # Safety
    ///
    /// The group must have been stopped and all channels disabled first.
    pub unsafe fn cvi_vpss_destroy_grp(&self, grp: VPSS_GRP) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(VPSS_GRP) -> CVI_S32> =
            self.vpss.get(b"CVI_VPSS_DestroyGrp")?;
        Ok(func(grp))
    }

    /// Start a VPSS processing group.
    ///
    /// # Safety
    ///
    /// The group must have been created first.
    pub unsafe fn cvi_vpss_start_grp(&self, grp: VPSS_GRP) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(VPSS_GRP) -> CVI_S32> =
            self.vpss.get(b"CVI_VPSS_StartGrp")?;
        Ok(func(grp))
    }

    /// Stop a VPSS processing group.
    ///
    /// # Safety
    ///
    /// The group must have been started first.
    pub unsafe fn cvi_vpss_stop_grp(&self, grp: VPSS_GRP) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(VPSS_GRP) -> CVI_S32> =
            self.vpss.get(b"CVI_VPSS_StopGrp")?;
        Ok(func(grp))
    }

    /// Set VPSS channel attributes.
    ///
    /// # Safety
    ///
    /// `attr` must point to a valid `VPSS_CHN_ATTR_S`.
    pub unsafe fn cvi_vpss_set_chn_attr(
        &self,
        grp: VPSS_GRP,
        chn: VPSS_CHN,
        attr: *const VPSS_CHN_ATTR_S,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(VPSS_GRP, VPSS_CHN, *const VPSS_CHN_ATTR_S) -> CVI_S32,
        > = self.vpss.get(b"CVI_VPSS_SetChnAttr")?;
        Ok(func(grp, chn, attr))
    }

    /// Enable a VPSS channel.
    ///
    /// # Safety
    ///
    /// The channel must have been configured first.
    pub unsafe fn cvi_vpss_enable_chn(
        &self,
        grp: VPSS_GRP,
        chn: VPSS_CHN,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(VPSS_GRP, VPSS_CHN) -> CVI_S32> =
            self.vpss.get(b"CVI_VPSS_EnableChn")?;
        Ok(func(grp, chn))
    }

    /// Disable a VPSS channel.
    ///
    /// # Safety
    ///
    /// The channel must have been enabled first.
    pub unsafe fn cvi_vpss_disable_chn(
        &self,
        grp: VPSS_GRP,
        chn: VPSS_CHN,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(VPSS_GRP, VPSS_CHN) -> CVI_S32> =
            self.vpss.get(b"CVI_VPSS_DisableChn")?;
        Ok(func(grp, chn))
    }

    /// Get a frame from a VPSS channel.
    ///
    /// # Safety
    ///
    /// `frame` must point to a valid `VIDEO_FRAME_INFO_S` that will be
    /// written to. The returned frame must later be released with
    /// [`CviLibs::cvi_vpss_release_chn_frame`].
    pub unsafe fn cvi_vpss_get_chn_frame(
        &self,
        grp: VPSS_GRP,
        chn: VPSS_CHN,
        frame: *mut VIDEO_FRAME_INFO_S,
        timeout_ms: CVI_S32,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(VPSS_GRP, VPSS_CHN, *mut VIDEO_FRAME_INFO_S, CVI_S32) -> CVI_S32,
        > = self.vpss.get(b"CVI_VPSS_GetChnFrame")?;
        Ok(func(grp, chn, frame, timeout_ms))
    }

    /// Release a frame previously obtained from a VPSS channel.
    ///
    /// # Safety
    ///
    /// `frame` must point to a frame obtained from
    /// [`CviLibs::cvi_vpss_get_chn_frame`].
    pub unsafe fn cvi_vpss_release_chn_frame(
        &self,
        grp: VPSS_GRP,
        chn: VPSS_CHN,
        frame: *const VIDEO_FRAME_INFO_S,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(VPSS_GRP, VPSS_CHN, *const VIDEO_FRAME_INFO_S) -> CVI_S32,
        > = self.vpss.get(b"CVI_VPSS_ReleaseChnFrame")?;
        Ok(func(grp, chn, frame))
    }

    // ---------------------------------------------------------------
    // VENC functions (from libvenc.so)
    // ---------------------------------------------------------------

    /// Create a video encoding channel.
    ///
    /// # Safety
    ///
    /// `attr` must point to a valid `VENC_CHN_ATTR_S`.
    pub unsafe fn cvi_venc_create_chn(
        &self,
        chn: VENC_CHN,
        attr: *const VENC_CHN_ATTR_S,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(VENC_CHN, *const VENC_CHN_ATTR_S) -> CVI_S32,
        > = self.venc.get(b"CVI_VENC_CreateChn")?;
        Ok(func(chn, attr))
    }

    /// Destroy a video encoding channel.
    ///
    /// # Safety
    ///
    /// The channel must have been stopped and all streams released first.
    pub unsafe fn cvi_venc_destroy_chn(&self, chn: VENC_CHN) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(VENC_CHN) -> CVI_S32> =
            self.venc.get(b"CVI_VENC_DestroyChn")?;
        Ok(func(chn))
    }

    /// Start receiving frames on an encoding channel.
    ///
    /// # Safety
    ///
    /// `param` must point to a valid `VENC_RECV_PIC_PARAM_S`.
    pub unsafe fn cvi_venc_start_recv_frame(
        &self,
        chn: VENC_CHN,
        param: *const VENC_RECV_PIC_PARAM_S,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(VENC_CHN, *const VENC_RECV_PIC_PARAM_S) -> CVI_S32,
        > = self.venc.get(b"CVI_VENC_StartRecvFrame")?;
        Ok(func(chn, param))
    }

    /// Stop receiving frames on an encoding channel.
    ///
    /// # Safety
    ///
    /// The channel must have been started first.
    pub unsafe fn cvi_venc_stop_recv_frame(
        &self,
        chn: VENC_CHN,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(VENC_CHN) -> CVI_S32> =
            self.venc.get(b"CVI_VENC_StopRecvFrame")?;
        Ok(func(chn))
    }

    /// Send a frame to a video encoding channel.
    ///
    /// # Safety
    ///
    /// `frame` must point to a valid `VIDEO_FRAME_INFO_S`.
    pub unsafe fn cvi_venc_send_frame(
        &self,
        chn: VENC_CHN,
        frame: *const VIDEO_FRAME_INFO_S,
        timeout_ms: CVI_S32,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(VENC_CHN, *const VIDEO_FRAME_INFO_S, CVI_S32) -> CVI_S32,
        > = self.venc.get(b"CVI_VENC_SendFrame")?;
        Ok(func(chn, frame, timeout_ms))
    }

    /// Get an encoded stream from a video encoding channel.
    ///
    /// # Safety
    ///
    /// `stream` must point to a valid `VENC_STREAM_S` that will be written to.
    /// The returned stream must later be released with
    /// [`CviLibs::cvi_venc_release_stream`].
    pub unsafe fn cvi_venc_get_stream(
        &self,
        chn: VENC_CHN,
        stream: *mut VENC_STREAM_S,
        timeout_ms: CVI_S32,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(VENC_CHN, *mut VENC_STREAM_S, CVI_S32) -> CVI_S32,
        > = self.venc.get(b"CVI_VENC_GetStream")?;
        Ok(func(chn, stream, timeout_ms))
    }

    /// Release an encoded stream previously obtained from a VENC channel.
    ///
    /// # Safety
    ///
    /// `stream` must point to a stream obtained from
    /// [`CviLibs::cvi_venc_get_stream`].
    pub unsafe fn cvi_venc_release_stream(
        &self,
        chn: VENC_CHN,
        stream: *mut VENC_STREAM_S,
    ) -> Result<CVI_S32, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(VENC_CHN, *mut VENC_STREAM_S) -> CVI_S32,
        > = self.venc.get(b"CVI_VENC_ReleaseStream")?;
        Ok(func(chn, stream))
    }

    // ---------------------------------------------------------------
    // NN functions (from libcviruntime.so)
    // ---------------------------------------------------------------

    /// Register a .cvimodel file and get a model handle.
    ///
    /// # Safety
    ///
    /// `model_file` must be a valid null-terminated C string pointing to an
    /// existing `.cvimodel` file. `model` must point to valid, writable memory.
    pub unsafe fn cvi_nn_register_model(
        &self,
        model_file: *const core::ffi::c_char,
        model: *mut CVI_MODEL_HANDLE,
    ) -> Result<CVI_RC, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(*const core::ffi::c_char, *mut CVI_MODEL_HANDLE) -> CVI_RC,
        > = self.cviruntime.get(b"CVI_NN_RegisterModel")?;
        Ok(func(model_file, model))
    }

    /// Get input and output tensors from a model.
    ///
    /// # Safety
    ///
    /// `model` must be a valid handle obtained from
    /// [`CviLibs::cvi_nn_register_model`]. All pointer arguments must be valid
    /// and writable.
    pub unsafe fn cvi_nn_get_input_output_tensors(
        &self,
        model: CVI_MODEL_HANDLE,
        inputs: *mut *mut CVI_TENSOR,
        input_num: *mut i32,
        outputs: *mut *mut CVI_TENSOR,
        output_num: *mut i32,
    ) -> Result<CVI_RC, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(
                CVI_MODEL_HANDLE,
                *mut *mut CVI_TENSOR,
                *mut i32,
                *mut *mut CVI_TENSOR,
                *mut i32,
            ) -> CVI_RC,
        > = self.cviruntime.get(b"CVI_NN_GetInputOutputTensors")?;
        Ok(func(model, inputs, input_num, outputs, output_num))
    }

    /// Run inference (blocking).
    ///
    /// # Safety
    ///
    /// `model` must be a valid handle. `inputs` and `outputs` must point to
    /// tensor arrays of the correct length.
    pub unsafe fn cvi_nn_forward(
        &self,
        model: CVI_MODEL_HANDLE,
        inputs: *mut CVI_TENSOR,
        input_num: i32,
        outputs: *mut CVI_TENSOR,
        output_num: i32,
    ) -> Result<CVI_RC, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(
                CVI_MODEL_HANDLE,
                *mut CVI_TENSOR,
                i32,
                *mut CVI_TENSOR,
                i32,
            ) -> CVI_RC,
        > = self.cviruntime.get(b"CVI_NN_Forward")?;
        Ok(func(model, inputs, input_num, outputs, output_num))
    }

    /// Cleanup/unload a model.
    ///
    /// # Safety
    ///
    /// `model` must be a valid handle that has not already been cleaned up.
    pub unsafe fn cvi_nn_cleanup_model(
        &self,
        model: CVI_MODEL_HANDLE,
    ) -> Result<CVI_RC, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(CVI_MODEL_HANDLE) -> CVI_RC> =
            self.cviruntime.get(b"CVI_NN_CleanupModel")?;
        Ok(func(model))
    }

    /// Get tensor buffer pointer.
    ///
    /// # Safety
    ///
    /// `tensor` must point to a valid `CVI_TENSOR`.
    pub unsafe fn cvi_nn_tensor_ptr(
        &self,
        tensor: *mut CVI_TENSOR,
    ) -> Result<*mut core::ffi::c_void, libloading::Error> {
        let func: libloading::Symbol<
            unsafe extern "C" fn(*mut CVI_TENSOR) -> *mut core::ffi::c_void,
        > = self.cviruntime.get(b"CVI_NN_TensorPtr")?;
        Ok(func(tensor))
    }

    /// Get tensor element count.
    ///
    /// # Safety
    ///
    /// `tensor` must point to a valid `CVI_TENSOR`.
    pub unsafe fn cvi_nn_tensor_count(
        &self,
        tensor: *mut CVI_TENSOR,
    ) -> Result<usize, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(*mut CVI_TENSOR) -> usize> =
            self.cviruntime.get(b"CVI_NN_TensorCount")?;
        Ok(func(tensor))
    }

    /// Get tensor shape.
    ///
    /// # Safety
    ///
    /// `tensor` must point to a valid `CVI_TENSOR`.
    pub unsafe fn cvi_nn_tensor_shape(
        &self,
        tensor: *mut CVI_TENSOR,
    ) -> Result<CVI_SHAPE, libloading::Error> {
        let func: libloading::Symbol<unsafe extern "C" fn(*mut CVI_TENSOR) -> CVI_SHAPE> =
            self.cviruntime.get(b"CVI_NN_TensorShape")?;
        Ok(func(tensor))
    }
}
