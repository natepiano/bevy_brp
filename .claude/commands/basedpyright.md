# DO NOT CHECK IF DIRECTORY EXISTS - RUN DIRECTLY

**NEVER** check if .claude/scripts/ exists. **NEVER** use `if [ -d ".claude/scripts/" ]` or any similar check.

Run these commands **IMMEDIATELY** without any checks:

```bash
cd .claude && ~/.local/bin/basedpyright scripts/
```


Create a todo list for each issue returned by basedpyright. **Fix all errors first, then fix all warnings** - both must be resolved. For each issue add the following to the todo list:

<BasedpyrightTodos>
- [ ] fix all type errors FIRST (these are critical and must be resolved before warnings)
- [ ] fix all type warnings SECOND (these must also be fixed, but only after all errors are resolved)
- [ ] fix each type annotation issue (or group of tightly related issues in the same function/class)
- [ ] if the issue is a code change, run `~/.local/bin/basedpyright <filename>` on the fixed file to make sure it passes type checking before moving to the next fix
</BasedpyrightTodos>

**Priority Order**:
1. **Errors** (must be 0) - Fix these first as they indicate actual type mismatches
2. **Warnings** (must be 0) - Fix these second to ensure full type safety
3. **Notes** (optional) - Can be addressed if time permits

**Important**
- Add proper type hints for all function parameters and return types
- Replace `Any` types with specific types where possible
- For JSON data, create TypedDict or dataclass definitions to properly type the data structures
- Don't use `# type: ignore` comments to suppress warnings - fix the underlying type issues
- **Exception**: `# pyright: ignore[reportAny]` is acceptable ONLY when:
  - Loading JSON files with `json.load()` where the structure varies or comes from external sources
  - Using third-party libraries that don't provide type stubs
  - Other cases where the source legitimately returns `Any` and cannot be typed more specifically
