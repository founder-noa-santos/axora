from __future__ import annotations

from typing import Any, Protocol

from openakta_logger import Sink


class LoggerLike(Protocol):
    def emit(self, record: dict[str, Any]) -> None: ...


class ProviderLike(Protocol):
    def get_logger(self, name: str, version: str | None = None) -> LoggerLike: ...


class OtelSink(Sink):
    def __init__(self, provider: ProviderLike, logger_name: str = "openakta-logger") -> None:
        self._logger = provider.get_logger(logger_name, "0.1.0")

    async def export(self, event: dict[str, Any]) -> None:
        attributes: dict[str, Any] = {
            "openakta.event_id": event["event_id"],
            "openakta.operation": event["operation"],
            "openakta.status": event["status"],
            "openakta.duration_ms": event["duration_ms"],
            "service.name": event["service"],
            "deployment.environment.name": event["environment"],
        }

        for key, value in event["context"].items():
            attributes[f"openakta.ctx.{key}"] = value

        if event["error"]["message"]:
            attributes["exception.type"] = event["error"]["type"]
            attributes["exception.message"] = event["error"]["message"]
            attributes["exception.stacktrace"] = event["error"]["stack"]

        self._logger.emit(
            {
                "severityNumber": _severity_number(event["level"]),
                "severityText": event["level"].upper(),
                "body": event["operation"],
                "attributes": attributes,
                "timestamp": _iso_to_nanos(event["timestamp_end"]),
                "observedTimestamp": _iso_to_nanos(event["timestamp_start"]),
                "resource": {
                    "attributes": {
                        "service.name": event["service"],
                        "deployment.environment.name": event["environment"],
                    }
                },
            }
        )


def _iso_to_nanos(timestamp: str) -> int:
    from datetime import datetime

    return int(datetime.fromisoformat(timestamp.replace("Z", "+00:00")).timestamp() * 1_000_000_000)


def _severity_number(level: str) -> int:
    mapping = {"info": 9, "warn": 13, "error": 17, "fatal": 21}
    return mapping.get(level, 9)
