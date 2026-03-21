from __future__ import annotations

import asyncio
import json
import sys
from typing import Any

from .base import Sink


class ConsoleSink(Sink):
    async def export(self, event: dict[str, Any]) -> None:
        line = json.dumps(event, separators=(",", ":"), ensure_ascii=False) + "\n"
        await asyncio.to_thread(sys.stdout.write, line)
