use strum::{Display, EnumString, IntoStaticStr};

use super::extraction::{FieldSpec, ResponseFieldType};

/// Enum representing all possible response field names
#[derive(Display, EnumString, IntoStaticStr, Debug, Clone, Copy, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum ResponseFieldName {
    /// `app_name` - Application name
    AppName,
    /// `app_name_filter` - Application name filter
    AppNameFilter,
    /// `app_running` - Whether app is running
    AppRunning,
    /// `apps` - List of applications
    Apps,
    /// `brp_responsive` - Whether BRP is responsive
    BrpResponsive,
    /// `component_count` - Number of components
    ComponentCount,
    /// `components` - Component data
    Components,
    /// `content` - Content field
    Content,
    /// `count` - General count field
    Count,
    /// `debug_enabled` - Whether debug is enabled
    DebugEnabled,
    /// `debug_info` - Debug information
    DebugInfo,
    /// `deleted_count` - Number of deleted items
    DeletedCount,
    /// `deleted_files` - List of deleted files
    DeletedFiles,
    /// `details` - Additional details
    Details,
    /// `duration_ms` - Duration in milliseconds
    DurationMs,
    /// `entities` - List of entities
    Entities,
    /// `entity` - Entity ID
    Entity,
    /// `entity_count` - Number of entities
    EntityCount,
    /// `error_count` - Number of errors
    ErrorCount,
    /// `examples` - List of examples
    Examples,
    /// `exists` - Whether something exists
    Exists,
    /// `file_path` - File path
    FilePath,
    /// `file_size_bytes` - File size in bytes
    FileSizeBytes,
    /// `filename` - File name
    Filename,
    /// `filtered_by_keyword` - Whether filtered by keyword
    FilteredByKeyword,
    /// `format_corrected` - Whether format was corrected
    FormatCorrected,
    /// `format_corrections` - Format correction information
    FormatCorrections,
    /// `keys_sent` - Keys that were sent
    KeysSent,
    /// `lines_read` - Number of lines read
    LinesRead,
    /// `log_path` - Path to log file
    LogPath,
    /// `logs` - List of logs
    Logs,
    /// `message` - Response message
    Message,
    /// `metadata` - Metadata field
    Metadata,
    /// `method_count` - Number of methods
    MethodCount,
    /// `older_than_seconds` - Age filter in seconds
    OlderThanSeconds,
    /// `parent` - Parent entity
    Parent,
    /// `path` - File or directory path
    Path,
    /// `pid` - Process ID
    Pid,
    /// `resource` - Resource data
    Resource,
    /// `resource_count` - Number of resources
    ResourceCount,
    /// `result` - General result field
    Result,
    /// `shutdown_method` - Method used for shutdown
    ShutdownMethod,
    /// `size_bytes` - Size in bytes
    SizeBytes,
    /// `size_human` - Human-readable size
    SizeHuman,
    /// `status` - Response status (success/error)
    Status,
    /// `tail_mode` - Whether in tail mode
    TailMode,
    /// `temp_directory` - Temporary directory path
    TempDirectory,
    /// `tracing_level` - Current tracing level
    TracingLevel,
    /// `tracinglog_file` - Log file path
    TracingLogFile,
    /// `type_count` - Number of types
    TypeCount,
    /// `watch_id` - Watch identifier
    WatchId,
    /// `watches` - List of watches
    Watches,
}

impl ResponseFieldName {
    /// Get the expected field type for this response field
    pub const fn field_type(self) -> ResponseFieldType {
        match self {
            // String fields
            Self::AppName
            | Self::LogPath
            | Self::Path
            | Self::ShutdownMethod
            | Self::AppNameFilter
            | Self::Filename
            | Self::FilePath
            | Self::Message
            | Self::SizeHuman
            | Self::Status
            | Self::TracingLevel
            | Self::TracingLogFile
            | Self::TempDirectory => ResponseFieldType::String,
            // Multi-line content fields - use LineSplit
            Self::Content => ResponseFieldType::LineSplit,
            // Count fields - use Count type to automatically count arrays/objects
            Self::Count
            | Self::ComponentCount
            | Self::EntityCount
            | Self::ErrorCount
            | Self::ResourceCount
            | Self::TypeCount
            | Self::MethodCount
            | Self::DeletedCount => ResponseFieldType::Count,
            // Regular number fields
            Self::Entity
            | Self::Parent
            | Self::Pid
            | Self::DurationMs
            | Self::WatchId
            | Self::OlderThanSeconds
            | Self::FileSizeBytes
            | Self::SizeBytes
            | Self::LinesRead => ResponseFieldType::Number,
            // Boolean fields
            Self::FormatCorrected
            | Self::DebugEnabled
            | Self::Exists
            | Self::FilteredByKeyword
            | Self::TailMode
            | Self::AppRunning
            | Self::BrpResponsive => ResponseFieldType::Boolean,
            // Array and Object/Any fields
            Self::Apps
            | Self::Entities
            | Self::DeletedFiles
            | Self::Examples
            | Self::Watches
            | Self::Logs
            | Self::KeysSent
            | Self::Components
            | Self::DebugInfo
            | Self::FormatCorrections
            | Self::Metadata
            | Self::Result
            | Self::Resource
            | Self::Details => ResponseFieldType::Any,
        }
    }
}

impl FieldSpec<ResponseFieldType> for ResponseFieldName {
    fn field_name(&self) -> &str {
        (*self).into()
    }

    fn field_type(&self) -> ResponseFieldType {
        (*self).field_type()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_response_field_name_conversion() {
        // Test that we can convert from string to enum
        let app_name = ResponseFieldName::from_str("app_name");
        assert!(app_name.is_ok());
        assert_eq!(
            app_name.unwrap_or(ResponseFieldName::AppName),
            ResponseFieldName::AppName
        );

        let entity_count = ResponseFieldName::from_str("entity_count");
        assert!(entity_count.is_ok());
        assert_eq!(
            entity_count.unwrap_or(ResponseFieldName::Count),
            ResponseFieldName::EntityCount
        );

        // Test Display implementation
        assert_eq!(ResponseFieldName::AppName.to_string(), "app_name");
        assert_eq!(ResponseFieldName::EntityCount.to_string(), "entity_count");
    }

    #[test]
    fn test_field_types() {
        // Test string fields
        assert_eq!(
            ResponseFieldName::AppName.field_type(),
            ResponseFieldType::String
        );
        assert_eq!(
            ResponseFieldName::LogPath.field_type(),
            ResponseFieldType::String
        );

        // Test count fields
        assert_eq!(
            ResponseFieldName::Count.field_type(),
            ResponseFieldType::Count
        );
        assert_eq!(
            ResponseFieldName::Entity.field_type(),
            ResponseFieldType::Number
        );

        // Test boolean fields
        assert_eq!(
            ResponseFieldName::FormatCorrected.field_type(),
            ResponseFieldType::Boolean
        );

        // Test any fields
        assert_eq!(
            ResponseFieldName::Components.field_type(),
            ResponseFieldType::Any
        );
    }
}
