from __future__ import annotations

import asyncio
import copy
import inspect
import json
import time
import traceback
import uuid
from datetime import datetime, timezone
from typing import Any

from .sinks.base import Sink

SDK_VERSION = "0.1.0"
VALID_ENVIRONMENTS = {"production", "staging", "development"}


def _normalize_environment(value: str | None) -> str:
    if value in VALID_ENVIRONMENTS:
        return value
    return "production"


def _normalize_error(exc: BaseException | Any) -> dict[str, str | None]:
    if isinstance(exc, BaseException):
        return {
            "type": type(exc).__name__,
            "message": str(exc) or None,
            "stack": "".join(traceback.format_exception(type(exc), exc, exc.__traceback__)),
        }

    if exc is None:
        return {"type": None, "message": None, "stack": None}

    if isinstance(exc, str):
        return {"type": "Error", "message": exc, "stack": None}

    try:
        message = json.dumps(exc, default=str)
    except Exception:
        message = str(exc)

    return {"type": type(exc).__name__, "message": message, "stack": None}


class WideEvent:
    def __init__(
        self,
        operation: str,
        service: str,
        environment: str,
        sinks: list[Sink],
    ) -> None:
        self._event_id = str(uuid.uuid4())
        self._timestamp_start = datetime.now(timezone.utc)
        self._start_monotonic = time.perf_counter()
        self._operation = operation
        self._service = service
        self._environment = _normalize_environment(environment)
        self._sinks = list(sinks)
        self._context: dict[str, Any] = {}
        self._error: dict[str, str | None] = {"type": None, "message": None, "stack": None}
        self._level = "info"
        self._status = "ok"
        self._finalized = False
        self._emit_task: asyncio.Task[None] | None = None

    def _assert_mutable(self) -> None:
        if self._finalized:
            raise RuntimeError("WideEvent has already been finalized")

    def append_context(self, **fields: Any) -> "WideEvent":
        """Merge arbitrary fields into the event context."""
        self._assert_mutable()
        self._context.update(copy.deepcopy(fields))
        return self

    def set_error(self, exc: BaseException | Any) -> "WideEvent":
        self._assert_mutable()
        self._level = "error"
        self._status = "error"
        self._error = _normalize_error(exc)
        return self

    def _build_payload(
        self,
        level: str | None = None,
        status: str | None = None,
    ) -> dict[str, Any]:
        timestamp_end = datetime.now(timezone.utc)
        payload = {
            "event_id": self._event_id,
            "service": self._service,
            "environment": self._environment,
            "timestamp_start": self._timestamp_start.isoformat(),
            "timestamp_end": timestamp_end.isoformat(),
            "duration_ms": round((time.perf_counter() - self._start_monotonic) * 1000, 3),
            "level": level or self._level,
            "operation": self._operation,
            "status": status or self._status,
            "context": copy.deepcopy(self._context),
            "error": copy.deepcopy(self._error),
            "meta": {"sdk_version": SDK_VERSION, "sdk_language": "python"},
        }
        return payload

    async def _safe_export(self, sink: Sink, payload: dict[str, Any]) -> None:
        try:
            await sink.export(payload)
        except Exception:
            return

    async def _dispatch(self, payload: dict[str, Any]) -> None:
        await asyncio.gather(*(self._safe_export(sink, payload) for sink in self._sinks))

    async def emit(self, level: str | None = None, status: str | None = None) -> None:
        if self._emit_task is not None:
            await self._emit_task
            return

        payload = self._build_payload(level=level, status=status)
        self._finalized = True
        self._emit_task = asyncio.create_task(self._dispatch(payload))
        await self._emit_task
