from __future__ import annotations

from typing import Any, Protocol

from axora_logger import Sink


class PosthogClientLike(Protocol):
    def capture(self, distinct_id: str, event: str, properties: dict[str, Any], timestamp: str | None = None) -> None: ...

    def shutdown(self) -> None: ...


class PosthogSink(Sink):
    def __init__(self, client: PosthogClientLike) -> None:
        self._client = client

    async def export(self, event: dict[str, Any]) -> None:
        distinct_id = event["context"].get("user_id") or f"service:{event['service']}"
        self._client.capture(
            distinct_id,
            event["operation"],
            {
                **event["context"],
                "axora_event_id": event["event_id"],
                "axora_service": event["service"],
                "status": event["status"],
                "level": event["level"],
                "duration_ms": event["duration_ms"],
                **({"error_message": event["error"]["message"]} if event["error"]["message"] else {}),
            },
            timestamp=event["timestamp_start"],
        )

    async def flush(self) -> None:
        self._client.shutdown()
