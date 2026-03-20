# com.axora:logger-sentry

Sentry adapter boundary for AXORA Wide Events.

## Notes

This package uses a small bridge interface so the payload mapping stays stable
without hard-coding a specific Sentry SDK surface into the adapter contract.
