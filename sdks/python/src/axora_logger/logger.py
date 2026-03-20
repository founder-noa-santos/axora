from __future__ import annotations

import inspect
import os
from typing import Any, Awaitable, Callable, TypeVar

from .sinks.base import Sink
from .wide_event import WideEvent

T = TypeVar("T")


def _resolve_service(service: str | None) -> str:
    resolved = (service or os.getenv("AXORA_SERVICE") or "").strip()
    if not resolved:
        raise ValueError("Logger requires a service name or AXORA_SERVICE")
    return resolved


def _resolve_environment(environment: str | None) -> str:
    resolved = (environment or os.getenv("AXORA_ENV") or "production").strip()
    if resolved not in {"production", "staging", "development"}:
        return "production"
    return resolved


class Logger:
    def __init__(
        self,
        service: str | None = None,
        environment: str | None = None,
        sinks: list[Sink] | None = None,
        default_context: dict[str, Any] | None = None,
    ) -> None:
        self.service = _resolve_service(service)
        self.environment = _resolve_environment(environment)
        self.sinks: list[Sink] = list(sinks or [])
        self.default_context = copy_default_context(default_context or {})

    def start_event(self, operation: str) -> WideEvent:
        event = WideEvent(operation, self.service, self.environment, self.sinks)
        if self.default_context:
            event.append_context(**self.default_context)
        return event

    async def trace(
        self,
        operation: str,
        fn: Callable[[WideEvent], Awaitable[T] | T],
    ) -> T:
        event = self.start_event(operation)
        try:
            result = fn(event)
            if inspect.isawaitable(result):
                result = await result
            await event.emit(status="ok")
            return result
        except Exception as exc:
            event.set_error(exc)
            await event.emit()
            raise


def copy_default_context(value: dict[str, Any]) -> dict[str, Any]:
    import copy

    return copy.deepcopy(value)
