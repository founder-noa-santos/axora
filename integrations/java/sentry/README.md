# com.openakta:logger-sentry

Sentry adapter boundary for OPENAKTA Wide Events.

## Notes

This package uses a small bridge interface so the payload mapping stays stable
without hard-coding a specific Sentry SDK surface into the adapter contract.
