from __future__ import annotations

from unittest.mock import patch

import asyncio

from axora_logger_sentry import SentrySink


def test_sentry_sink_captures_errors() -> None:
    sink = SentrySink()
    with patch("sentry_sdk.capture_exception") as capture_exception, patch("sentry_sdk.push_scope") as push_scope:
        scope = push_scope.return_value.__enter__.return_value
        asyncio.run(
            sink.export(
                {
                    "event_id": "2f64f7ef-efc9-4b9f-8d70-1a0f1e4b6f2d",
                    "service": "svc",
                    "environment": "production",
                    "timestamp_start": "2026-03-19T00:00:00.000Z",
                    "timestamp_end": "2026-03-19T00:00:00.500Z",
                    "duration_ms": 500,
                    "level": "error",
                    "operation": "job.fail",
                    "status": "error",
                    "context": {"attempt": 1},
                    "error": {"type": "Error", "message": "boom", "stack": "stack"},
                    "meta": {"sdk_version": "0.1.0", "sdk_language": "python"},
                }
            )
        )

        capture_exception.assert_called_once()
        scope.set_tag.assert_any_call("service", "svc")

