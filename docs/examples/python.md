# Python Example

```python
from axora_logger import Logger
from axora_logger.sinks.console_sink import ConsoleSink

logger = Logger(
    service="billing-api",
    environment="production",
    sinks=[ConsoleSink()],
    default_context={"region": "eu-west-1"},
)

async def handle_login() -> None:
    event = logger.start_event("user.login")
    event.append_context(user_id="usr_123", method="oauth2")
    await event.emit(status="ok")

async def capture_payment() -> str:
    async def traced(event):
        event.append_context(amount=99.99, currency="EUR")
        return await process_payment()

    return await logger.trace("payment.capture", traced)
```

The Python SDK accepts sync or async traced callables. When the callable returns normally, the event is finalized with `status="ok"`. When it raises, the SDK marks the event as an error and re-raises.
