//! Type definitions for format discovery system
//!
//! This module contains all type definitions for the format discovery system,
//! including component field types for pattern matching.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::unified_types::UnifiedTypeInfo;

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

/// Information about a format correction applied during recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionInfo {
    /// The type that was corrected
    pub type_name:         String,
    /// Original value that needed correction
    pub original_value:    Value,
    /// Corrected value to use
    pub corrected_value:   Value,
    /// Human-readable explanation of the correction
    pub hint:              String,
    /// Component or resource name for error reporting
    pub target_type:       String,
    /// format information for error responses (usage, `valid_values`, examples)
    pub corrected_format:  Option<Value>,
    /// Type information discovered during correction (if available)
    pub type_info:         Option<UnifiedTypeInfo>,
    /// The correction method used
    pub correction_method: CorrectionMethod,
}

/// Method used to correct a format error
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CorrectionMethod {
    /// Direct replacement based on exact type knowledge
    DirectReplacement,
    /// Object to array conversion for math types
    ObjectToArray,
    /// Array to object conversion
    ArrayToObject,
    /// String to enum variant conversion
    StringToEnum,
    /// Nested structure correction
    NestedCorrection,
    /// Field name mapping or aliasing
    FieldMapping,
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

/// Method used to discover or correct a type format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiscoverySource {
    /// Information from `bevy_brp_extras` direct discovery
    DirectDiscovery,
    /// Information from Bevy's type registry
    TypeRegistry,
    /// Information inferred from error patterns
    PatternMatching,
    /// Information from built-in type knowledge
    BuiltinTypes,
    /// Manually provided or hardcoded information
    Manual,
    /// Information combined from registry and extras sources
    RegistryPlusExtras,
}

/// Information about an enum variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    /// The name of the variant
    pub name:         String,
    /// The type of the variant (Unit, Tuple, Struct)
    pub variant_type: String,
}

/// Information about an enum type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumInfo {
    /// List of enum variants
    pub variants: Vec<EnumVariant>,
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

/// Format-specific information and examples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    /// Real example values for different BRP operations
    pub examples:         HashMap<String, Value>,
    /// Available mutation paths if the type supports mutation
    pub mutation_paths:   HashMap<String, String>,
    /// Original format that caused the error (if applicable)
    pub original_format:  Option<Value>,
    /// Corrected format to use instead
    pub corrected_format: Option<Value>,
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
/// Registry and reflection status for a type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStatus {
    /// Whether the type is registered in Bevy's type registry
    pub in_registry: bool,
    /// Whether the type has reflection support
    pub has_reflect: bool,
    /// Type path as registered in the registry
    pub type_path:   Option<String>,
}

/// Serialization trait support for a type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializationSupport {
    /// Whether the type implements Serialize
    pub has_serialize:   bool,
    /// Whether the type implements Deserialize
    pub has_deserialize: bool,
    /// Whether the type can be used in BRP operations requiring serialization
    pub brp_compatible:  bool,
}

/// Result of a transformation operation containing the corrected value and a hint
#[derive(Debug, Clone)]
pub struct TransformationResult {
    /// The corrected value after transformation
    pub corrected_value: Value,
    /// Human-readable hint about the transformation
    pub hint:            String,
}

/// Category of type for quick identification and processing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TypeCategory {
    /// Unknown or unclassified type
    Unknown,
    /// Regular struct type
    Struct,
    /// Tuple struct type
    TupleStruct,
    /// Enum type
    Enum,
    /// Math type (Vec2, Vec3, Quat, etc.)
    MathType,
    /// Component type
    Component,
}
