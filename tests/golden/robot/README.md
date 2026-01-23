# Robot Mode Golden Files

Golden files for robot mode JSON output regression testing.

## Purpose

These files capture the expected JSON structure for robot mode output. They serve as a contract that AI agents depend on - ANY change to field names, structure, or formatting is considered a breaking change.

## File Inventory

### Device Operations
- `device_info.json` - Single device info output
- `device_list_single.json` - List with one device
- `device_list_multiple.json` - List with multiple devices
- `device_list_empty.json` - Empty device list

### Error Responses
- `error_no_device.json` - No devices found error
- `error_multiple_devices.json` - Multiple devices without serial specified

### Display Operations
- `brightness_set.json` - Brightness change confirmation
- `key_set.json` - Key image set confirmation
- `key_cleared.json` - Key cleared confirmation
- `key_filled.json` - Key filled with color confirmation
- `all_cleared.json` - All keys cleared confirmation
- `all_filled.json` - All keys filled confirmation

### Button Events
- `button_event_press.json` - Button press event (compact JSON)
- `button_event_release.json` - Button release event (compact JSON)
- `button_states.json` - Current button states array

### Batch Operations
- `batch_set_keys.json` - Batch set-keys results
- `batch_fill_keys.json` - Batch fill-keys results
- `batch_clear_keys.json` - Batch clear-keys results

### Messages
- `success.json` - Generic success message
- `warning.json` - Warning message
- `info.json` - Info message
- `version.json` - Version info output

## Updating Golden Files

If a deliberate change to JSON output is needed:

1. Update the corresponding golden file(s)
2. Run tests to verify the change
3. Update CHANGELOG.md to note the breaking change
4. Consider major version bump for breaking changes

## JSON Requirements

All robot mode JSON must:
- Use snake_case for field names (never camelCase)
- Use numbers for numeric values (not strings)
- Use booleans for boolean values (not strings)
- Button events must be single-line compact JSON (for streaming)
- Errors must include: error, message, suggestion, recoverable fields
