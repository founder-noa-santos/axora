from __future__ import annotations

import asyncio
from typing import Any

import pytest

from openakta_logger import Logger, Sink


class CaptureSink(Sink):
    def __init__(self) -> None:
        self.events: list[dict[str, Any]] = []

    async def export(self, event: dict[str, Any]) -> None:
        self.events.append(event)


class FailingSink(Sink):
    async def export(self, event: dict[str, Any]) -> None:
        raise RuntimeError("sink failed")


def test_snapshots_context_and_finalizes_event() -> None:
    sink = CaptureSink()
    logger = Logger(
        service="my-api",
        environment="staging",
        sinks=[sink],
        default_context={"region": "eu-west-1"},
    )

    details = {"step": 1}
    event = logger.start_event("user.login")
    event.append_context(details=details)

    asyncio.run(event.emit())
    details["step"] = 2

    assert sink.events[0]["service"] == "my-api"
    assert sink.events[0]["environment"] == "staging"
    assert sink.events[0]["status"] == "ok"
    assert sink.events[0]["context"] == {
        "region": "eu-west-1",
        "details": {"step": 1},
    }

    with pytest.raises(RuntimeError):
        event.append_context(later=True)


def test_env_fallback_is_used_when_constructor_values_are_missing(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("OPENAKTA_SERVICE", "env-service")
    monkeypatch.setenv("OPENAKTA_ENV", "development")

    sink = CaptureSink()
    logger = Logger(sinks=[sink])

    asyncio.run(logger.trace("task.run", lambda _event: None))

    assert sink.events[0]["service"] == "env-service"
    assert sink.events[0]["environment"] == "development"
    assert sink.events[0]["operation"] == "task.run"


def test_sink_failures_do_not_break_trace() -> None:
    logger = Logger(service="svc", sinks=[FailingSink()])
    def traced(event):
        event.append_context(ok=True)
        return 42

    result = asyncio.run(logger.trace("job.run", traced))
    assert result == 42


def test_trace_records_errors() -> None:
    sink = CaptureSink()
    logger = Logger(service="svc", sinks=[sink])

    with pytest.raises(RuntimeError, match="boom"):
        asyncio.run(
            logger.trace(
                "job.fail",
                lambda event: (_ for _ in ()).throw(RuntimeError("boom")),
            )
        )

    assert sink.events[0]["level"] == "error"
    assert sink.events[0]["status"] == "error"
