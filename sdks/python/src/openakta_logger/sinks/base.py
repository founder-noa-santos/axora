from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any


class Sink(ABC):
    """
    Abstract base for all OPENAKTA exporters.
    Implement export to send the Wide Event to any destination.
    """

    @abstractmethod
    async def export(self, event: dict[str, Any]) -> None:
        """
        Called once per finalized Wide Event.
        Must be an async coroutine. Exceptions are caught and swallowed by the core SDK.
        """

    async def flush(self) -> None:
        """Optional graceful shutdown hook."""

