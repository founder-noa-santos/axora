from __future__ import annotations

import sys
import types
from typing import Any

from axora_logger import Sink

try:
    import sentry_sdk
except ModuleNotFoundError:  # pragma: no cover - exercised when extras are absent.
    sentry_sdk = types.ModuleType("sentry_sdk")

    def _missing(*args: Any, **kwargs: Any) -> None:
        raise RuntimeError("sentry-sdk is required for SentrySink")

    class _Scope:
        def __enter__(self) -> "_Scope":
            return self

        def __exit__(self, exc_type, exc, tb) -> None:
            return None

        def set_tag(self, *args: Any, **kwargs: Any) -> None:
            return None

        def set_context(self, *args: Any, **kwargs: Any) -> None:
            return None

        level: str | None = None

    sentry_sdk.capture_exception = _missing  # type: ignore[attr-defined]
    sentry_sdk.add_breadcrumb = _missing  # type: ignore[attr-defined]
    sentry_sdk.push_scope = lambda: _Scope()  # type: ignore[attr-defined]
    sys.modules.setdefault("sentry_sdk", sentry_sdk)


class SentrySink(Sink):
    async def export(self, event: dict[str, Any]) -> None:
        is_error = event["status"] in {"error", "timeout"}
        if is_error:
            with sentry_sdk.push_scope() as scope:
                scope.set_tag("service", event["service"])
                scope.set_tag("environment", event["environment"])
                scope.set_tag("operation", event["operation"])
                scope.set_tag("axora.event_id", event["event_id"])
                scope.set_context("axora", event["context"])
                scope.level = "fatal" if event["level"] == "fatal" else event["level"]

                error_type = event["error"]["type"] or "AxoraError"
                error_cls = type(error_type, (RuntimeError,), {})
                error = error_cls(event["error"]["message"] or event["operation"])
                if event["error"]["stack"]:
                    error.__traceback__ = None
                sentry_sdk.capture_exception(error)
            return

        sentry_sdk.add_breadcrumb(
            category=event["operation"],
            message=event["operation"],
            level=event["level"],
            data={**event["context"], "duration_ms": event["duration_ms"]},
            timestamp=_iso_to_seconds(event["timestamp_start"]),
        )


def _iso_to_seconds(timestamp: str) -> float:
    from datetime import datetime

    return datetime.fromisoformat(timestamp.replace("Z", "+00:00")).timestamp()
