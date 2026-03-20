# axora-logger

Canonical AXORA Wide Event SDK for Python.

## Install

```bash
pip install axora-logger
```

## Usage

```python
from axora_logger import Logger, ConsoleSink

logger = Logger(
    service="my-api",
    environment="production",
    sinks=[ConsoleSink()],
    default_context={"region": "eu-west-1"},
)

event = logger.start_event("user.login")
event.append_context(user_id="usr_123", method="oauth2")
await event.emit(status="ok")
```
