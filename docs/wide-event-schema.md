# Wide Event Schema

This document is the canonical payload contract for AXORA diagnostics SDKs.
Every language implementation emits one sealed Wide Event per logical operation.

## Contract Summary

- `event_id` is a UUID v4.
- `timestamp_start` and `timestamp_end` are ISO 8601 UTC timestamps.
- `duration_ms` is a monotonic duration in milliseconds.
- `context` is the only open-ended object; it accepts arbitrary structured fields.
- `error` is always present and always structured.
- `meta` carries SDK identity and language metadata.

## JSON Schema

The machine-readable contract lives in:

[`docs/wide-event-schema.json`](/Users/noasantos/Fluri/axora/docs/wide-event-schema.json)

## Notes for SDK Authors

- The payload is snapshot-based. Context values must be cloned on append so later caller mutation does not alter the emitted event.
- After finalization, mutation APIs must reject further changes.
- Sink failures must be isolated from business logic.
- Integration adapters should map from this canonical payload, not invent new top-level fields.
