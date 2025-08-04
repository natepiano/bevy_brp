//! Type definitions for format discovery system
//!
//! This module contains all type definitions for the format discovery system,
//! including component field types for pattern matching.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{CorrectionInfo, UnifiedTypeInfo};

/// Represents color component fields
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorField {
    /// Red component (R in RGB)
    Red,
    /// Green component (G in RGB)
    Green,
    /// Blue component (B in RGB)
    Blue,
    /// Alpha component (transparency)
    Alpha,
    /// Hue component (H in HSL/HSV)
    Hue,
    /// Saturation component (S in HSL/HSV)
    Saturation,
    /// Lightness component (L in HSL)
    Lightness,
    /// Value component (V in HSV)
    Value,
    /// Whiteness component (W in HWB)
    Whiteness,
    /// Blackness component (B in HWB)
    Blackness,
    /// Chroma component (C in LCH)
    Chroma,
    /// A component (in Lab color space)
    A,
    /// B component (in Lab color space)
    B,
}

/// Represents mathematical vector/quaternion component fields
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathField {
    /// X component
    X,
    /// Y component
    Y,
    /// Z component
    Z,
    /// W component
    W,
}

/// Represents all supported component types (colors and math types)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    // Color types
    /// Linear RGBA color
    LinearRgba,
    /// sRGB color with alpha
    Srgba,
    /// HSL color with alpha
    Hsla,
    /// HSV color with alpha
    Hsva,
    /// HWB color with alpha
    Hwba,
    /// Lab color with alpha
    Laba,
    /// LCH color with alpha
    Lcha,
    /// Oklab color with alpha
    Oklaba,
    /// Oklch color with alpha
    Oklcha,
    /// XYZ color with alpha
    Xyza,

    // Math types - floating point
    /// 2D vector (f32)
    Vec2,
    /// 3D vector (f32)
    Vec3,
    /// 4D vector (f32)
    Vec4,
    /// Quaternion (f32)
    Quat,

    // Math types - signed integers
    /// 2D vector (i32)
    IVec2,
    /// 3D vector (i32)
    IVec3,
    /// 4D vector (i32)
    IVec4,

    // Math types - unsigned integers
    /// 2D vector (u32)
    UVec2,
    /// 3D vector (u32)
    UVec3,
    /// 4D vector (u32)
    UVec4,

    // Math types - double precision
    /// 2D vector (f64)
    DVec2,
    /// 3D vector (f64)
    DVec3,
    /// 4D vector (f64)
    DVec4,
}

impl ComponentType {
    /// Checks if this is a color type
    pub const fn is_color(self) -> bool {
        matches!(
            self,
            Self::LinearRgba
                | Self::Srgba
                | Self::Hsla
                | Self::Hsva
                | Self::Hwba
                | Self::Laba
                | Self::Lcha
                | Self::Oklaba
                | Self::Oklcha
                | Self::Xyza
        )
    }

    /// Checks if this is a Lab-based color type
    pub const fn is_lab_based(self) -> bool {
        matches!(self, Self::Laba | Self::Lcha | Self::Oklaba | Self::Oklcha)
    }
}

/// Represents a field access on a component
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldAccess {
    /// The component type being accessed
    pub component_type: ComponentType,
    /// The field being accessed (either color or math field)
    pub field:          Field,
}

/// Represents either a color field or a math field
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// Color component field
    Color(ColorField),
    /// Math component field
    Math(MathField),
}

/// Result of individual correction attempts during recovery
#[derive(Debug, Clone)]
pub enum CorrectionResult {
    /// Correction was successfully applied
    Corrected { correction_info: CorrectionInfo },
    /// Correction could not be applied but metadata was discovered
    CannotCorrect {
        type_info: UnifiedTypeInfo,
        reason:    String,
    },
}

/// Format correction information for a type (component or resource)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatCorrection {
    pub component:            String, // Keep field name for API compatibility
    pub original_format:      Value,
    pub corrected_format:     Value,
    pub hint:                 String,
    pub supported_operations: Option<Vec<String>>,
    pub mutation_paths:       Option<Vec<String>>,
    pub type_category:        Option<String>,
}

impl FormatCorrection {}

/// Status of format correction attempts
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatCorrectionStatus {
    /// Format discovery was not enabled for this request
    NotApplicable,
    /// No format correction was attempted
    NotAttempted,
    /// Format correction was applied and the operation succeeded
    Succeeded,
    /// Format correction was attempted but the operation still failed
    AttemptedButFailed,
}
