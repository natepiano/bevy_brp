# Comprehensive Checklist: Rename `brp_type_schema` to `brp_type_guide`

This plan covers the complete rename from `brp_type_schema` → `brp_type_guide` and `brp_all_type_schemas` → `brp_all_type_guides`.

## 1. Files and Directories to Rename

### Directories
- [ ] `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/` → `brp_type_guide/`

### Files to Rename
- [ ] `/Users/natemccoy/rust/bevy_brp/.claude/commands/tests/type_schema.md` → `type_guide.md`
- [ ] `/Users/natemccoy/rust/bevy_brp/.claude/commands/complex_component_type_schema.md` → `complex_component_type_guide.md`
- [ ] `/Users/natemccoy/rust/bevy_brp/mcp/help_text/brp_type_schema.txt` → `brp_type_guide.txt`
- [ ] `/Users/natemccoy/rust/bevy_brp/mcp/help_text/brp_all_type_schemas.txt` → `brp_all_type_guides.txt`

## 2. Struct Names and Type Names

### Main Types in `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/tool.rs`
- [ ] Line 23: `TypeSchemaParams` → `TypeGuideParams`
- [ ] Line 34: `TypeSchemaResult` → `TypeGuideResult`
- [ ] Line 51: `TypeSchema` → `TypeGuide`
- [ ] Line 67: `TypeSchemaEngine` → `TypeGuideEngine`

### Response Types in `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/response_types.rs`
- [ ] Line 205: `TypeSchemaResponse` → `TypeGuideResponse`
- [ ] Line 218: `TypeSchemaSummary` → `TypeGuideSummary`

## 3. Enum Variants

### Tool Name Enum in `/Users/natemccoy/rust/bevy_brp/mcp/src/tool/tool_name.rs`
- [ ] Line 267: `BrpTypeSchema` → `BrpTypeGuide`
- [ ] Line 269: `BrpAllTypeSchemas` → `BrpAllTypeGuides`

## 4. Function Names and Methods

### In `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/mutation_path_builder/recursion_context.rs`
- [ ] Line 72: `get_type_schema` → `get_type_guide`
- [ ] Line 251: `create_minimal_type_schema_error` → `create_minimal_type_guide_error`

### In `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_client/client.rs`
- [ ] Line 266: `create_full_type_schema_error` → `create_full_type_guide_error`

### In `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_schema/type_info.rs`
- [ ] Line 273: `extract_reflect_types(type_schema: &Value)` → `extract_reflect_types(type_guide: &Value)`
- [ ] Line 287: `extract_schema_info(type_schema: &Value)` → `extract_schema_info(type_guide: &Value)`

## 5. Variable Names and Parameters

### Parameter and Variable Names
- [ ] All instances of `type_schema` parameter → `type_guide`
- [ ] All instances of `type_schema` variable → `type_guide`

**Key locations:**
- [ ] Line 59: `type_schema: &Value` in type_info.rs
- [ ] Line 120: `let type_info = if let Some(type_schema)` in tool.rs
- [ ] Line 327: `type_schema: &Value` parameter in enum_builder.rs
- [ ] Multiple instances in builder files

## 6. Module References and Use Statements

### Module Declarations in `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/mod.rs`
- [ ] Line 2: `mod brp_type_schema;` → `mod brp_type_guide;`
- [ ] Line 18: `pub use brp_type_schema::{...}` → `pub use brp_type_guide::{...}`

### Use Statements (50+ instances)
All `use crate::brp_tools::brp_type_schema::...` imports need updating across:
- [ ] All mutation path builder files
- [ ] Tool implementation files
- [ ] Response type files
- [ ] Engine files

## 7. Configuration Files

### MCP Tool Names in `/Users/natemccoy/rust/bevy_brp/.claude/settings.local.json`
- [ ] Line 78: `"mcp__brp__brp_type_schema"` → `"mcp__brp__brp_type_guide"`
- [ ] Line 81: `"mcp__brp__brp_all_type_schemas"` → `"mcp__brp__brp_all_type_guides"`

### Test Configuration in `/Users/natemccoy/rust/bevy_brp/.claude/commands/test_config.json`
- [ ] Line 13: `"test_name": "type_schema"` → `"test_name": "type_guide"`
- [ ] Line 14: `"test_file": ".claude/commands/tests/type_schema.md"` → `".claude/commands/tests/type_guide.md"`

## 8. Documentation and Comments

### Tool Documentation in `/Users/natemccoy/rust/bevy_brp/mcp/src/tool/tool_name.rs`
- [ ] Line 266: `/// \`brp_type_schema\` - Local type schema discovery` → `/// \`brp_type_guide\` - Local type guide discovery`
- [ ] Line 268: `/// \`brp_all_type_schemas\` - Get schemas for all registered types` → `/// \`brp_all_type_guides\` - Get guides for all registered types`

### File Header Comments
- [ ] Line 1: `//! \`brp_type_schema\` tool` in tool.rs → `//! \`brp_type_guide\` tool`
- [ ] Line 1: `//! Public API response types for the \`brp_type_schema\` tool` in response_types.rs → `//! Public API response types for the \`brp_type_guide\` tool`

### Error Messages and Help Text
- [ ] Line 257: "Use the brp_type_schema tool" in client.rs → "Use the brp_type_guide tool"
- [ ] Line 16: "type_schema embedding" comment in types.rs → "type_guide embedding"

## 9. Markdown Documentation Files

### Documentation Files to Update
- [ ] `README.md` - Update tool references and examples
- [ ] `CHANGELOG.md` - Update any references to the tool
- [ ] `CLAUDE.md` - Update any references to the tool
- [ ] Various plan-*.md files - Update references
- [ ] Test markdown files - Update test descriptions and examples

## 10. JSON Reference Files

### Reference JSON in `.claude/commands/reference_json/`
- [ ] All files containing `"mcp_tool": "brp_type_schema"` → `"mcp_tool": "brp_type_guide"`
- [ ] All files containing `"mcp_tool": "brp_all_type_schemas"` → `"mcp_tool": "brp_all_type_guides"`

## 11. Help Text Files Content

### Help Text Updates
- [ ] `/Users/natemccoy/rust/bevy_brp/mcp/help_text/brp_type_guide.txt` (renamed file) - Update internal references
- [ ] `/Users/natemccoy/rust/bevy_brp/mcp/help_text/brp_all_type_guides.txt` (renamed file) - Update internal references

## 12. Macro and Derive Attributes

### Derive and Attribute Updates
Look for any derive macros or attributes that might reference the old names:
- [ ] Tool-related derive attributes
- [ ] Serde rename attributes
- [ ] Any custom attributes referencing the old names

## 13. Test Files and Examples

### Test Files
- [ ] All test files mentioning the tool names
- [ ] Example usage in documentation
- [ ] Integration test references

## 14. Build and Configuration Scripts

### Build Scripts
- [ ] Any build.rs files that might reference the tools
- [ ] Cargo.toml descriptions or metadata
- [ ] CI/CD pipeline references

## 15. String Literals and Constants

### String Constants
- [ ] Any const strings containing the old tool names
- [ ] Error message strings
- [ ] Log message strings
- [ ] Format strings in macros

---

## Execution Strategy

**Recommended order:**
1. **File/Directory renames first** (to avoid broken imports)
2. **Module declarations and use statements**
3. **Type definitions (structs, enums)**
4. **Function names and implementations**
5. **Variable names and parameters**
6. **Configuration files**
7. **Documentation and comments**
8. **Test files and examples**

## Verification Steps

After completing the rename:
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] All MCP tools load correctly
- [ ] Integration tests pass
- [ ] Documentation builds without warnings
- [ ] No remaining references to old names (global search)

---

**Total Estimated Changes:** 150+ references across 50+ files