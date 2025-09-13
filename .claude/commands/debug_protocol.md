# Debug Protocol

Protocol for debugging BRP tool issues by examining trace logs.

<InstallWarning>
## IMPORTANT NOTE ##
If you have recently made changes and haven't intalled it, remember you need to ask the user to install it. If you haven't made changes in this session without installing, then you can ignore this and continue to the rest of the instructions without commenting on this
</InstallWarning>

## Steps

### Step 1: Install Tool (if needed)
If you haven't recently installed the tool or have made changes that need testing:
- Follow instructions in `.claude/commands/build_and_install.md`
- Skip this step if you've just recently done the build and install

### Step 2: Remove Old Trace Log
Clean up any existing trace log to ensure you're only seeing new output:
```bash
rm /var/folders/rf/twhh0jfd243fpltn5k0w1t980000gn/T/bevy_brp_mcp_trace.log
```

### Step 3: Set Trace Level
Enable debug-level tracing using the BRP tool:
```bash
mcp__brp__brp_set_tracing_level(level="debug")
```

### Step 4: Execute Test Command
Run the command you want to debug. The trace log will capture detailed information about:
- Parameter extraction and validation
- Method resolution processes
- Request processing steps
- Error diagnostics and stack traces

### Step 5: Examine Trace Log
Read the trace log to analyze the results:
```bash
Read(/var/folders/rf/twhh0jfd243fpltn5k0w1t980000gn/T/bevy_brp_mcp_trace.log)
```

Look for:
- Parameter parsing details
- BRP request/response payloads
- Error messages and stack traces
- Type resolution information

## Notes
- The trace log persists across BRP sessions until manually deleted
- Debug level provides detailed information without the verbosity of trace level
- Useful for diagnosing issues with type discovery, mutation paths, and BRP operations
