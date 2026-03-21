from __future__ import annotations

import asyncio
import json
import os
from typing import Any, Mapping
from urllib import error, request

from .base import Sink

DEFAULT_TIMEOUT_MS = 5000


class HttpSink(Sink):
    def __init__(
        self,
        *,
        url: str | None = None,
        headers: Mapping[str, str] | None = None,
        timeout_ms: int | None = None,
        token: str | None = None,
    ) -> None:
        resolved_url = url or os.getenv("OPENAKTA_SINK_URL")
        if not resolved_url:
            raise ValueError("HttpSink requires a url or OPENAKTA_SINK_URL")

        resolved_token = token or os.getenv("OPENAKTA_SINK_TOKEN")
        self._url = resolved_url
        self._timeout_s = (timeout_ms or DEFAULT_TIMEOUT_MS) / 1000.0
        self._headers = {"Content-Type": "application/json", **dict(headers or {})}
        if resolved_token and "Authorization" not in self._headers:
            self._headers["Authorization"] = f"Bearer {resolved_token}"

    def _post(self, event: dict[str, Any]) -> None:
        body = json.dumps(event, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
        req = request.Request(self._url, data=body, headers=self._headers, method="POST")
        try:
            with request.urlopen(req, timeout=self._timeout_s) as response:
                if response.status >= 400:
                    raise RuntimeError(f"HTTP {response.status} {response.reason}")
        except error.HTTPError as exc:
            details = exc.read().decode("utf-8", errors="replace")
            raise RuntimeError(f"HTTP {exc.code} {exc.reason}: {details}") from exc

    async def export(self, event: dict[str, Any]) -> None:
        await asyncio.to_thread(self._post, event)
