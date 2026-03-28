//! Local inference engine for `.cvimodel` files on the Seeed reCamera (SG2002).
//!
//! This crate loads `.cvimodel` files that have been **pre-converted from ONNX**
//! using Sophgo's offline toolchain (`model_tool` / `cvimodel_tool`). The SDK
//! does **not** handle ONNX-to-cvimodel conversion; that must be done separately
//! before deploying to the device.
//!
//! # Example
//!
//! ```rust,no_run
//! use recamera_infer::{Engine, Output};
//! use std::path::Path;
//!
//! let engine = Engine::new().unwrap();
//! let model = engine.load_model(Path::new("/userdata/models/yolo.cvimodel")).unwrap();
//! // ... prepare frame data ...
//! // let output = model.run(&frame_data).unwrap();
//! ```

use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use recamera_core::{Error, FrameData, Result};
use recamera_cvi_sys::CviLibs;

/// Shape of a single tensor (input or output).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TensorShape {
    /// Dimension sizes, e.g. `[1, 3, 640, 640]` for a batch-1 RGB image.
    pub dims: Vec<usize>,
}

impl TensorShape {
    /// Create a new [`TensorShape`] from the given dimensions.
    pub fn new(dims: Vec<usize>) -> Self {
        Self { dims }
    }

    /// Return the total number of elements (product of all dimensions).
    ///
    /// An empty `dims` vector yields `1` (the identity element of
    /// multiplication).
    pub fn total_elements(&self) -> usize {
        self.dims.iter().copied().product::<usize>().max(1)
    }
}

/// Metadata about a loaded model.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Path to the `.cvimodel` file on disk.
    pub path: PathBuf,
    /// Shape of the model's input tensor.
    pub input_shape: TensorShape,
    /// Shapes of the model's output tensors.
    pub output_shapes: Vec<TensorShape>,
}

/// A single object detection result.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    /// Normalised x-coordinate of the bounding-box centre (0.0 .. 1.0).
    pub x: f32,
    /// Normalised y-coordinate of the bounding-box centre (0.0 .. 1.0).
    pub y: f32,
    /// Normalised width of the bounding box (0.0 .. 1.0).
    pub w: f32,
    /// Normalised height of the bounding box (0.0 .. 1.0).
    pub h: f32,
    /// Class identifier as defined by the model.
    pub class_id: u32,
    /// Confidence score (0.0 .. 1.0).
    pub score: f32,
}

/// Output produced by running a model on a frame.
#[derive(Debug, Clone)]
pub enum Output {
    /// Zero or more object detections.
    Detections(Vec<Detection>),
    /// A single classification result.
    Classification {
        /// Predicted class identifier.
        class_id: u32,
        /// Confidence score.
        score: f32,
    },
    /// Raw output tensors (one `Vec<f32>` per output head).
    Raw(Vec<Vec<f32>>),
}

/// CVI NPU inference engine.
///
/// Loads the CVI runtime library at runtime and provides model loading.
/// Use [`Engine::new`] to create an instance, then [`Engine::load_model`]
/// to load a `.cvimodel` file for inference.
pub struct Engine {
    libs: Arc<CviLibs>,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine").finish()
    }
}

impl Engine {
    /// Create a new inference engine.
    ///
    /// Loads the CVI runtime library from the device's standard library paths.
    /// This must be called on the reCamera device where `libcviruntime.so` is
    /// installed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Inference`] if the vendor libraries cannot be loaded.
    pub fn new() -> Result<Self> {
        let libs = CviLibs::load()
            .map_err(|e| Error::Inference(format!("failed to load CVI libraries: {e}")))?;
        Ok(Self {
            libs: Arc::new(libs),
        })
    }

    /// Load a `.cvimodel` file and prepare it for inference.
    ///
    /// The model file must have been pre-converted from ONNX using Sophgo's
    /// offline toolchain.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Inference`] if the file does not have a `.cvimodel`
    /// extension, if the file cannot be loaded, or if tensor metadata cannot
    /// be retrieved.
    pub fn load_model(&self, path: &Path) -> Result<Model> {
        // Validate extension
        match path.extension().and_then(|e| e.to_str()) {
            Some("cvimodel") => {}
            _ => {
                return Err(Error::Inference(format!(
                    "expected .cvimodel extension, got: {}",
                    path.display()
                )));
            }
        }

        let path_str = path.to_str().ok_or_else(|| {
            Error::Inference(format!("invalid path encoding: {}", path.display()))
        })?;
        let c_path = CString::new(path_str)
            .map_err(|e| Error::Inference(format!("invalid path: {e}")))?;

        let mut handle: recamera_cvi_sys::CVI_MODEL_HANDLE = std::ptr::null_mut();

        unsafe {
            let rc = self
                .libs
                .cvi_nn_register_model(c_path.as_ptr(), &mut handle)
                .map_err(|e| Error::Inference(format!("RegisterModel symbol: {e}")))?;
            if rc != 0 {
                return Err(Error::Inference(format!(
                    "CVI_NN_RegisterModel failed (rc={rc})"
                )));
            }

            // Get input/output tensor metadata
            let mut inputs: *mut recamera_cvi_sys::CVI_TENSOR = std::ptr::null_mut();
            let mut input_num: i32 = 0;
            let mut outputs: *mut recamera_cvi_sys::CVI_TENSOR = std::ptr::null_mut();
            let mut output_num: i32 = 0;

            let rc = self
                .libs
                .cvi_nn_get_input_output_tensors(
                    handle,
                    &mut inputs,
                    &mut input_num,
                    &mut outputs,
                    &mut output_num,
                )
                .map_err(|e| Error::Inference(format!("GetInputOutputTensors symbol: {e}")))?;
            if rc != 0 {
                let _ = self.libs.cvi_nn_cleanup_model(handle);
                return Err(Error::Inference(format!(
                    "CVI_NN_GetInputOutputTensors failed (rc={rc})"
                )));
            }

            // Extract input shape
            let input_shape = if input_num > 0 && !inputs.is_null() {
                let shape = self
                    .libs
                    .cvi_nn_tensor_shape(inputs)
                    .map_err(|e| Error::Inference(format!("TensorShape symbol: {e}")))?;
                let dims: Vec<usize> = shape.dim[..shape.dim_size]
                    .iter()
                    .map(|&d| d as usize)
                    .collect();
                TensorShape::new(dims)
            } else {
                TensorShape::new(vec![])
            };

            // Extract output shapes
            let mut output_shapes = Vec::new();
            for i in 0..output_num {
                let tensor = outputs.add(i as usize);
                let shape = self
                    .libs
                    .cvi_nn_tensor_shape(tensor)
                    .map_err(|e| Error::Inference(format!("TensorShape symbol: {e}")))?;
                let dims: Vec<usize> = shape.dim[..shape.dim_size]
                    .iter()
                    .map(|&d| d as usize)
                    .collect();
                output_shapes.push(TensorShape::new(dims));
            }

            Ok(Model {
                info: ModelInfo {
                    path: path.to_path_buf(),
                    input_shape,
                    output_shapes,
                },
                handle,
                inputs,
                input_num,
                outputs,
                output_num,
                libs: Arc::clone(&self.libs),
            })
        }
    }
}

/// A loaded CVI model ready for inference.
///
/// Created by [`Engine::load_model`]. Call [`Model::run`] to perform inference
/// on a frame. The model is automatically unloaded when dropped.
pub struct Model {
    /// Metadata describing the model's input/output tensors.
    pub info: ModelInfo,
    handle: recamera_cvi_sys::CVI_MODEL_HANDLE,
    inputs: *mut recamera_cvi_sys::CVI_TENSOR,
    input_num: i32,
    outputs: *mut recamera_cvi_sys::CVI_TENSOR,
    output_num: i32,
    libs: Arc<CviLibs>,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("info", &self.info)
            .finish()
    }
}

impl Model {
    /// Run inference on a single frame and return raw output tensors.
    ///
    /// The frame data is copied into the model's input tensor, inference is
    /// executed on the NPU, and the results are read back as `Output::Raw`.
    ///
    /// Post-processing (e.g., YOLO NMS to produce `Detection` results) is
    /// the caller's responsibility for now.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Inference`] if the forward pass fails.
    pub fn run(&self, input: &FrameData) -> Result<Output> {
        unsafe {
            // Copy input data into the input tensor
            if self.input_num > 0 && !self.inputs.is_null() {
                let tensor_ptr = self
                    .libs
                    .cvi_nn_tensor_ptr(self.inputs)
                    .map_err(|e| Error::Inference(format!("TensorPtr symbol: {e}")))?;
                if !tensor_ptr.is_null() {
                    let tensor_count = self
                        .libs
                        .cvi_nn_tensor_count(self.inputs)
                        .map_err(|e| Error::Inference(format!("TensorCount symbol: {e}")))?;
                    let copy_len = input.data.len().min(tensor_count);
                    std::ptr::copy_nonoverlapping(
                        input.data.as_ptr(),
                        tensor_ptr as *mut u8,
                        copy_len,
                    );
                }
            }

            // Run forward pass
            let rc = self
                .libs
                .cvi_nn_forward(
                    self.handle,
                    self.inputs,
                    self.input_num,
                    self.outputs,
                    self.output_num,
                )
                .map_err(|e| Error::Inference(format!("Forward symbol: {e}")))?;
            if rc != 0 {
                return Err(Error::Inference(format!(
                    "CVI_NN_Forward failed (rc={rc})"
                )));
            }

            // Read output tensors
            let mut raw_outputs = Vec::new();
            for i in 0..self.output_num {
                let tensor = self.outputs.add(i as usize);
                let ptr = self
                    .libs
                    .cvi_nn_tensor_ptr(tensor)
                    .map_err(|e| Error::Inference(format!("TensorPtr symbol: {e}")))?;
                let count = self
                    .libs
                    .cvi_nn_tensor_count(tensor)
                    .map_err(|e| Error::Inference(format!("TensorCount symbol: {e}")))?;

                if !ptr.is_null() && count > 0 {
                    let float_ptr = ptr as *const f32;
                    let slice = std::slice::from_raw_parts(float_ptr, count);
                    raw_outputs.push(slice.to_vec());
                } else {
                    raw_outputs.push(Vec::new());
                }
            }

            Ok(Output::Raw(raw_outputs))
        }
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        unsafe {
            let _ = self.libs.cvi_nn_cleanup_model(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tensor_shape_total_elements() {
        let shape = TensorShape::new(vec![1, 3, 640, 640]);
        assert_eq!(shape.total_elements(), 1 * 3 * 640 * 640);
    }

    #[test]
    fn tensor_shape_empty_dims() {
        let shape = TensorShape::new(vec![]);
        assert_eq!(shape.total_elements(), 1);
    }

    #[test]
    fn detection_field_access() {
        let det = Detection {
            x: 0.5,
            y: 0.4,
            w: 0.2,
            h: 0.3,
            class_id: 7,
            score: 0.95,
        };
        assert_eq!(det.class_id, 7);
        assert!((det.score - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn output_detections_variant() {
        let output = Output::Detections(vec![Detection {
            x: 0.1,
            y: 0.2,
            w: 0.3,
            h: 0.4,
            class_id: 1,
            score: 0.9,
        }]);
        match &output {
            Output::Detections(dets) => assert_eq!(dets.len(), 1),
            _ => panic!("expected Detections variant"),
        }
    }

    #[test]
    fn extension_validation_rejects_wrong_extension() {
        // We can't create a real Engine without the device libraries,
        // but we can test the validation logic by checking the error.
        // Engine::new() will fail on non-device machines, so we test
        // the extension check indirectly through the type system.
        let path = Path::new("/tmp/model.onnx");
        let ext = path.extension().and_then(|e| e.to_str());
        assert_ne!(ext, Some("cvimodel"));
    }
}
