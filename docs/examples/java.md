# Java Example

```java
import com.axora.logger.Logger;
import com.axora.logger.LoggerBuilder;
import com.axora.logger.WideEvent;
import com.axora.logger.sinks.ConsoleSink;
import java.util.Map;

Logger logger = new LoggerBuilder()
    .service("billing-api")
    .environment("production")
    .addSink(new ConsoleSink())
    .defaultContext(Map.of("region", "eu-west-1"))
    .build();

WideEvent event = logger.startEvent("user.login");
event.appendContext(Map.of("user_id", "usr_123", "method", "oauth2"));
event.emit().join();

logger.trace("payment.capture", traced -> {
  traced.appendContext(Map.of("amount", 99.99, "currency", "EUR"));
  return processPayment();
}).join();
```

The Java trace API accepts a function that returns a `CompletableFuture<T>`. The emitted Wide Event is immutable after finalization and sink failures are swallowed by the SDK.
