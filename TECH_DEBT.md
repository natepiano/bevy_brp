# Technical Debt

## Port/Parameter Handling Refactor (2025-07-28)

### Context
Refactoring the tool parameter and result handling system to simplify tool implementations and improve the architecture. The goal is to have tools extract their own parameters and return them in the ToolResult, removing manual port handling from tools.

### Completed Work (Phases 1-3)
1. **Phase 1**: Infrastructure preparation
   - ✅ Moved ParamStruct trait to parameters.rs with impl for ()
   - ✅ Added get_call_info() method to ToolName
   - ✅ Updated ToolResult struct to include params: Option<P> field
   - ✅ Added temporary with_port_typed/without_port_typed constructors

2. **Phase 2**: Core trait updates
   - ✅ Updated ToolFn trait to add Params associated type (without default due to Rust limitations)
   - ✅ Changed ToolFn::call signature to return ToolResult<Output, Params>
   - ✅ Updated all tool implementations to specify type Params
   - ✅ Added ParamStruct bound with Serialize requirement
   - ✅ Updated type erasure layer to pass ToolResult to format_result

3. **Phase 3**: Response formatting updates
   - ✅ Updated format_result to accept ToolResult<T, P> instead of separate parameters
   - ✅ Added parameters field to JsonResponse and ResponseBuilder
   - ✅ Updated format_result to create CallInfo internally using tool_name and port

### Remaining Work (Phases 4-5)
4. **Phase 4**: Tool migration (partially complete)
   - ✅ Verified pattern with ListBevyApps (parameterless) and Status (with params)
   - ⚠️ Need to update remaining ~30 tools to return params in ToolResult
   - ⚠️ Need to update BRP tools macro to generate the new pattern

5. **Phase 5**: Cleanup
   - Remove with_port/without_port constructors
   - Remove port field from ToolResult
   - Remove #[to_call_info] annotations
   - Update tests

### Technical Decisions
1. **Associated type defaults**: Cannot use `type Params = ()` default due to unstable Rust feature. All tools must explicitly specify `type Params`.

2. **Serialize bound**: Added to ParamStruct trait rather than individual usages. All parameter structs now derive Serialize.

3. **Temporary constructors**: Added with_port_typed/without_port_typed to allow incremental migration without breaking existing code.

### Tool Migration Pattern
**Parameterless tools**:
```rust
fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output, Self::Params>> {
    Box::pin(async move {
        // Extract unit type parameters
        let _: () = ctx.extract_parameter_values()?;
        
        let result = handle_impl(ctx).await;
        Ok(ToolResult {
            port: None,
            result,
            params: None,
        })
    })
}
```

**Tools with parameters**:
```rust
fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output, Self::Params>> {
    Box::pin(async move {
        let params: MyParams = ctx.extract_parameter_values()?;
        let port = params.port;
        
        let result = handle_impl(&params.field, port).await;
        Ok(ToolResult {
            port: Some(port),
            result,
            params: Some(params),
        })
    })
}
```

### Next Steps
The next subagent should:
1. Continue from Phase 4.2 in .todo.json
2. Update all remaining tools following the patterns above
3. Update the BRP tools macro
4. Complete Phase 5 cleanup
5. Run full test suite