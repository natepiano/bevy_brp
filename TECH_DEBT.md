# Technical Debt

## Current Known Issues

### FormatterContext.params Redundancy
**Date:** 2025-07-15
**Status:** ✅ Completed  
**Context:** FormatterContext.params field duplicates HandlerContext.request.arguments. Both local and BRP handlers create FormatterContext.params which is redundant with HandlerContext.request.arguments that's already available.

**Decision:** Remove FormatterContext.params field and update FieldExtractor to use HandlerContext directly.

**Reasoning:** 
- Eliminates duplication
- Reduces memory usage
- Simplifies code by having single source of truth for request parameters
- HandlerContext.request.arguments is already validated and available

**Implementation Plan:**
1. Update FieldExtractor type signature from `Fn(&Value, &FormatterContext)` to `Fn(&Value, &HandlerContext<T>)`
2. Update create_request_field_accessor() to use handler_context.request.arguments
3. Remove params field from FormatterContext struct
4. Update ResponseFormatter format_success() method to pass HandlerContext to field extractors
5. Update initialize_template_values() to use HandlerContext.request.arguments directly
6. Update local and BRP handlers to remove params from FormatterContext creation

**Impact:** Breaking change to FieldExtractor interface, but internal to the codebase.