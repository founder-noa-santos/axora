from __future__ import annotations

import asyncio
from io import StringIO
from unittest.mock import patch

from axora_logger_datadog import DatadogSink


def test_datadog_sink_writes_json_line() -> None:
    sink = DatadogSink()
    buffer = StringIO()
    with patch("sys.stdout", buffer):
        asyncio.run(
            sink.export(
                {
                    "event_id": "2f64f7ef-efc9-4b9f-8d70-1a0f1e4b6f2d",
                    "service": "svc",
                    "environment": "production",
                    "timestamp_start": "2026-03-19T00:00:00.000Z",
                    "timestamp_end": "2026-03-19T00:00:00.500Z",
                    "duration_ms": 500,
                    "level": "info",
                    "operation": "job.run",
                    "status": "ok",
                    "context": {"attempt": 2},
                    "error": {"type": None, "message": None, "stack": None},
                    "meta": {"sdk_version": "0.1.0", "sdk_language": "python"},
                }
            )
        )

    assert '"dd.axora_event_id":"2f64f7ef-efc9-4b9f-8d70-1a0f1e4b6f2d"' in buffer.getvalue()
