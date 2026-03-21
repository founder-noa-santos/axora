# openakta-logger

Canonical OPENAKTA Wide Event SDK for Python.

## Install

```bash
pip install openakta-logger
```

## Usage

```python
from openakta_logger import Logger, ConsoleSink

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
