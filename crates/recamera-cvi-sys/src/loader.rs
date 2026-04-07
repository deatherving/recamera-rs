//! Extern declarations for CVI vendor functions.
//!
//! These functions are resolved at link time against the vendor `.so` files
//! shipped in the SDK (see `build.rs`). At runtime the device's dynamic
//! linker loads the shared libraries and their transitive dependencies
//! (e.g. `libatomic.so`) automatically.

use crate::bindings::*;

unsafe extern "C" {
    // ----- SYS / VB (libsys.so) -----
    pub fn CVI_SYS_Init() -> CVI_S32;
    pub fn CVI_SYS_Exit() -> CVI_S32;
    pub fn CVI_SYS_Bind(src: *const MMF_CHN_S, dst: *const MMF_CHN_S) -> CVI_S32;
    pub fn CVI_SYS_UnBind(src: *const MMF_CHN_S, dst: *const MMF_CHN_S) -> CVI_S32;
    pub fn CVI_VB_Init() -> CVI_S32;
    pub fn CVI_VB_Exit() -> CVI_S32;
    pub fn CVI_VB_SetConfig(config: *const VB_CONFIG_S) -> CVI_S32;

    // ----- VI (libvi.so) -----
    pub fn CVI_VI_SetDevAttr(dev: VI_DEV, attr: *const VI_DEV_ATTR_S) -> CVI_S32;
    pub fn CVI_VI_EnableDev(dev: VI_DEV) -> CVI_S32;
    pub fn CVI_VI_DisableDev(dev: VI_DEV) -> CVI_S32;
    pub fn CVI_VI_SetChnAttr(pipe: VI_PIPE, chn: VI_CHN, attr: *mut VI_CHN_ATTR_S) -> CVI_S32;
    pub fn CVI_VI_EnableChn(pipe: VI_PIPE, chn: VI_CHN) -> CVI_S32;
    pub fn CVI_VI_DisableChn(pipe: VI_PIPE, chn: VI_CHN) -> CVI_S32;
    pub fn CVI_VI_GetChnFrame(
        pipe: VI_PIPE,
        chn: VI_CHN,
        frame_info: *mut VIDEO_FRAME_INFO_S,
        timeout_ms: CVI_S32,
    ) -> CVI_S32;
    pub fn CVI_VI_ReleaseChnFrame(
        pipe: VI_PIPE,
        chn: VI_CHN,
        frame_info: *const VIDEO_FRAME_INFO_S,
    ) -> CVI_S32;

    // ----- VPSS (libvpss.so) -----
    pub fn CVI_VPSS_CreateGrp(grp: VPSS_GRP, attr: *const VPSS_GRP_ATTR_S) -> CVI_S32;
    pub fn CVI_VPSS_DestroyGrp(grp: VPSS_GRP) -> CVI_S32;
    pub fn CVI_VPSS_StartGrp(grp: VPSS_GRP) -> CVI_S32;
    pub fn CVI_VPSS_StopGrp(grp: VPSS_GRP) -> CVI_S32;
    pub fn CVI_VPSS_SetChnAttr(
        grp: VPSS_GRP,
        chn: VPSS_CHN,
        attr: *const VPSS_CHN_ATTR_S,
    ) -> CVI_S32;
    pub fn CVI_VPSS_EnableChn(grp: VPSS_GRP, chn: VPSS_CHN) -> CVI_S32;
    pub fn CVI_VPSS_DisableChn(grp: VPSS_GRP, chn: VPSS_CHN) -> CVI_S32;
    pub fn CVI_VPSS_GetChnFrame(
        grp: VPSS_GRP,
        chn: VPSS_CHN,
        frame: *mut VIDEO_FRAME_INFO_S,
        timeout_ms: CVI_S32,
    ) -> CVI_S32;
    pub fn CVI_VPSS_ReleaseChnFrame(
        grp: VPSS_GRP,
        chn: VPSS_CHN,
        frame: *const VIDEO_FRAME_INFO_S,
    ) -> CVI_S32;

    // ----- VENC (libvenc.so) -----
    pub fn CVI_VENC_CreateChn(chn: VENC_CHN, attr: *const VENC_CHN_ATTR_S) -> CVI_S32;
    pub fn CVI_VENC_DestroyChn(chn: VENC_CHN) -> CVI_S32;
    pub fn CVI_VENC_StartRecvFrame(
        chn: VENC_CHN,
        param: *const VENC_RECV_PIC_PARAM_S,
    ) -> CVI_S32;
    pub fn CVI_VENC_StopRecvFrame(chn: VENC_CHN) -> CVI_S32;
    pub fn CVI_VENC_SendFrame(
        chn: VENC_CHN,
        frame: *const VIDEO_FRAME_INFO_S,
        timeout_ms: CVI_S32,
    ) -> CVI_S32;
    pub fn CVI_VENC_GetStream(
        chn: VENC_CHN,
        stream: *mut VENC_STREAM_S,
        timeout_ms: CVI_S32,
    ) -> CVI_S32;
    pub fn CVI_VENC_ReleaseStream(chn: VENC_CHN, stream: *mut VENC_STREAM_S) -> CVI_S32;

    // ----- NN (libcviruntime.so) -----
    pub fn CVI_NN_RegisterModel(
        model_file: *const core::ffi::c_char,
        model: *mut CVI_MODEL_HANDLE,
    ) -> CVI_RC;
    pub fn CVI_NN_GetInputOutputTensors(
        model: CVI_MODEL_HANDLE,
        inputs: *mut *mut CVI_TENSOR,
        input_num: *mut i32,
        outputs: *mut *mut CVI_TENSOR,
        output_num: *mut i32,
    ) -> CVI_RC;
    pub fn CVI_NN_Forward(
        model: CVI_MODEL_HANDLE,
        inputs: *mut CVI_TENSOR,
        input_num: i32,
        outputs: *mut CVI_TENSOR,
        output_num: i32,
    ) -> CVI_RC;
    pub fn CVI_NN_CleanupModel(model: CVI_MODEL_HANDLE) -> CVI_RC;
    pub fn CVI_NN_TensorPtr(tensor: *mut CVI_TENSOR) -> *mut core::ffi::c_void;
    pub fn CVI_NN_TensorCount(tensor: *mut CVI_TENSOR) -> usize;
    pub fn CVI_NN_TensorShape(tensor: *mut CVI_TENSOR) -> CVI_SHAPE;
}
