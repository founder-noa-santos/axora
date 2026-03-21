from __future__ import annotations

import json
import sys
from typing import Any

from openakta_logger import Sink


class DatadogSink(Sink):
    async def export(self, event: dict[str, Any]) -> None:
        log_entry: dict[str, Any] = {
            "date": event["timestamp_start"],
            "status": event["level"],
            "service": event["service"],
            "message": event["operation"],
            "duration": event["duration_ms"],
            **event["context"],
            "dd.openakta_event_id": event["event_id"],
            "dd.env": event["environment"],
        }
        if event["error"]["message"]:
            log_entry["error"] = {
                "kind": event["error"]["type"],
                "message": event["error"]["message"],
                "stack": event["error"]["stack"],
            }
        sys.stdout.write(json.dumps(log_entry, separators=(",", ":"), ensure_ascii=False) + "\n")
