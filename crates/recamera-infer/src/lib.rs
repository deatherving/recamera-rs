//! Local inference engine for `.cvimodel` files on the Seeed reCamera (SG2002).
//!
//! This crate loads `.cvimodel` files that have been **pre-converted from ONNX**
//! using Sophgo's offline toolchain (`model_tool` / `cvimodel_tool`). The SDK
//! does **not** handle ONNX-to-cvimodel conversion; that must be done separately
//! before deploying to the device.
//!
//! All hardware operations currently return an error because the CVI runtime
//! FFI bindings have not yet been generated. Once bindings are available the
//! implementation will initialise the NPU and run real inference.

use std::path::{Path, PathBuf};

use recamera_core::{Error, FrameData, Result};

// Ensure the sys crate is linked when building for the target.
#[allow(unused_imports)]
use recamera_cvi_sys as _;

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

/// CVI NPU runtime handle.
///
/// Use [`Engine::new`] to create an instance, then [`Engine::load_model`] to
/// load a `.cvimodel` file for inference.
#[derive(Debug)]
pub struct Engine {
    _private: (),
}

impl Engine {
    /// Initialise the CVI NPU runtime.
    ///
    /// Currently returns an error unconditionally because the CVI runtime FFI
    /// bindings have not yet been generated.
    pub fn new() -> Result<Self> {
        Err(Error::Inference(
            "not yet implemented: requires CVI runtime bindings".into(),
        ))
    }

    /// Load a `.cvimodel` file and prepare it for inference.
    ///
    /// Returns [`Error::Inference`] if the path does not have a `.cvimodel`
    /// extension. After validation, returns the "not yet implemented" error
    /// because the CVI runtime FFI bindings have not yet been generated.
    pub fn load_model(&self, path: &Path) -> Result<Model> {
        // Validate extension.
        match path.extension().and_then(|e| e.to_str()) {
            Some("cvimodel") => {}
            _ => {
                return Err(Error::Inference(format!(
                    "expected .cvimodel extension, got: {}",
                    path.display()
                )));
            }
        }

        Err(Error::Inference(
            "not yet implemented: requires CVI runtime bindings".into(),
        ))
    }
}

/// A loaded CVI model ready for inference.
#[derive(Debug)]
pub struct Model {
    /// Metadata describing the model's input/output tensors.
    pub info: ModelInfo,
}

impl Model {
    /// Run inference on a single frame.
    ///
    /// Currently returns an error unconditionally because the CVI runtime FFI
    /// bindings have not yet been generated.
    pub fn run(&self, _input: &FrameData) -> Result<Output> {
        Err(Error::Inference(
            "not yet implemented: requires CVI runtime bindings".into(),
        ))
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
    fn engine_new_returns_not_yet_implemented() {
        let result = Engine::new();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("not yet implemented: requires CVI runtime bindings"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn extension_validation_rejects_wrong_extension() {
        let engine = Engine { _private: () };
        let result = engine.load_model(Path::new("/tmp/model.onnx"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            Error::Inference(msg) => {
                assert!(msg.contains("cvimodel"), "unexpected message: {msg}");
            }
            _ => panic!("expected Inference, got: {err}"),
        }
    }

    #[test]
    fn extension_validation_passes_then_returns_not_implemented() {
        let engine = Engine { _private: () };
        let result = engine.load_model(Path::new("/tmp/test.cvimodel"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            Error::Inference(msg) => {
                assert!(
                    msg.contains("not yet implemented"),
                    "unexpected message: {msg}"
                );
            }
            _ => panic!("expected Inference, got: {err}"),
        }
    }
}
