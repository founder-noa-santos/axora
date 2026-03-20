package com.axora.logger;

import com.axora.logger.sinks.Sink;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class LoggerTest {
  static class CaptureSink implements Sink {
    final List<WideEventPayload> events = new ArrayList<>();

    @Override
    public CompletableFuture<Void> export(WideEventPayload event) {
      events.add(event);
      return CompletableFuture.completedFuture(null);
    }
  }

  @Test
  void snapshotsContextAndFinalizesEvent() {
    CaptureSink sink = new CaptureSink();
    Logger logger =
        Logger.builder()
            .service("my-api")
            .environment("staging")
            .addSink(sink)
            .defaultContext(Map.of("region", "eu-west-1"))
            .build();

    Map<String, Object> details = new java.util.LinkedHashMap<>();
    details.put("step", 1);
    WideEvent event = logger.startEvent("user.login");
    event.appendContext(Map.of("details", details));
    event.emitAsync().join();
    details.put("step", 2);

    assertEquals(1, sink.events.size());
    assertEquals("my-api", sink.events.get(0).service());
    assertEquals("staging", sink.events.get(0).environment());
    assertEquals("ok", sink.events.get(0).status());
    assertEquals(1, ((Map<?, ?>) sink.events.get(0).context().get("details")).get("step"));

    assertThrows(IllegalStateException.class, () -> event.appendContext(Map.of("later", true)));
  }

  @Test
  void traceCapturesErrors() {
    CaptureSink sink = new CaptureSink();
    Logger logger = Logger.builder().service("svc").addSink(sink).build();

    assertThrows(
        java.util.concurrent.CompletionException.class,
        () ->
            logger.trace(
                    "job.fail",
                    event -> {
                      event.appendContext(Map.of("attempt", 1));
                      throw new RuntimeException("boom");
                    })
                .join());

    assertEquals("error", sink.events.get(0).level());
    assertEquals("error", sink.events.get(0).status());
  }
}
