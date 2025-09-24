# Remove the trace log

<GetTraceLogPath/>
Get the current trace log file path:
- Use `mcp__brp__brp_get_trace_log_path` to get the correct trace log path
- This ensures portability across different systems

<CheckFileExists/>
Verify the trace log exists:
- Check if the file exists at the returned path
- If no file exists, inform the user that no trace log was found

<RemoveTraceLog/>
Remove the trace log file:
- Use Bash tool to execute `rm [discovered_path]` with description "Remove BRP trace log file"
- Handle potential permission errors gracefully

<ConfirmRemoval/>
Confirm successful removal:
- Verify the file has been deleted
- Provide clear feedback about the operation result

<HandleErrors/>
Handle common error scenarios:
- File not found (already removed or never created)
- Permission denied (insufficient rights)
- Other filesystem errors
