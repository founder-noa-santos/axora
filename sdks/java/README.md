# com.openakta:logger-core

Canonical OPENAKTA Wide Event SDK for Java.

## Usage

```java
Logger logger = Logger.builder()
    .service("my-api")
    .environment("production")
    .addSink(new ConsoleSink())
    .build();

WideEvent event = logger.startEvent("user.login");
event.appendContext(Map.of("user_id", "usr_123"));
event.emitAsync();
```
