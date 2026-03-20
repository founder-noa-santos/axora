from __future__ import annotations

from typing import Any

from axora_logger_otel import OtelSink


class LoggerCapture:
    def __init__(self) -> None:
        self.records: list[dict[str, Any]] = []

    def emit(self, record: dict[str, Any]) -> None:
        self.records.append(record)


class ProviderCapture:
    def __init__(self) -> None:
        self.logger = LoggerCapture()

    def get_logger(self, name: str, version: str | None = None) -> LoggerCapture:
        assert name == "axora-logger"
        assert version == "0.1.0"
        return self.logger


def test_otel_sink_maps_payload() -> None:
    provider = ProviderCapture()
    sink = OtelSink(provider)

    import asyncio

    asyncio.run(
        sink.export(
            {
                "event_id": "2f64f7ef-efc9-4b9f-8d70-1a0f1e4b6f2d",
                "service": "svc",
                "environment": "production",
                "timestamp_start": "2026-03-19T00:00:00.000Z",
                "timestamp_end": "2026-03-19T00:00:00.500Z",
                "duration_ms": 500,
                "level": "warn",
                "operation": "job.run",
                "status": "timeout",
                "context": {"attempt": 2},
                "error": {"type": "TimeoutError", "message": "slow", "stack": "stack"},
                "meta": {"sdk_version": "0.1.0", "sdk_language": "python"},
            }
        )
    )

    assert provider.logger.records[0]["severityText"] == "WARN"
    assert provider.logger.records[0]["attributes"]["axora.ctx.attempt"] == 2
