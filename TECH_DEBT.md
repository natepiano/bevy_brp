# Technical Debt

## JSON Field Extraction Unification (2025-07-18) - FULLY RESOLVED

### Context
Successfully unified the JSON field extraction systems between parameter and response extraction.

### Resolution
- Changed `JsonFieldProvider` trait to return owned `Value` instead of `&Value`
- Removed thread-local storage workaround completely
- Eliminated `FieldExtractor` type and all boxed closures
- Updated `FormatterConfig` to store `ResponseField` specifications directly
- **Removed `ParamType` enum entirely - now using unified `FieldType` everywhere**
- Simplified extraction to its core operation: extracting fields from JSON according to specification

### Completed Changes
- ✅ Extended `FieldType` with `Count`, `LineSplit`, and `DynamicParams` for all field types
- ✅ Enhanced `JsonFieldProvider` with dot notation support
- ✅ Removed thread-local storage from `RequestArguments`
- ✅ Deleted `field_extractor.rs` module entirely
- ✅ Updated formatter to use direct field extraction instead of closures
- ✅ Replaced all `ParamType` usage with unified `FieldType`
- ✅ All tests pass, no regressions

### Benefits
- Truly unified extraction system with a single type system
- No more duplicate type enums (`ParamType` vs `FieldType`)
- No more lifetime issues or workarounds
- Direct, simple field extraction based on specifications
- Cleaner, more maintainable codebase