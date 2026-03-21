from __future__ import annotations

import asyncio

from openakta_logger_posthog import PosthogSink


class Client:
    def __init__(self) -> None:
        self.calls: list[tuple] = []

    def capture(self, distinct_id, event, properties, timestamp=None):
        self.calls.append((distinct_id, event, properties, timestamp))

    def shutdown(self) -> None:
        self.calls.append(("shutdown",))


def test_posthog_sink_captures_event() -> None:
    client = Client()
    sink = PosthogSink(client)

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
                "context": {"user_id": "usr_123", "attempt": 2},
                "error": {"type": None, "message": None, "stack": None},
                "meta": {"sdk_version": "0.1.0", "sdk_language": "python"},
            }
        )
    )

    assert client.calls[0][0] == "usr_123"
