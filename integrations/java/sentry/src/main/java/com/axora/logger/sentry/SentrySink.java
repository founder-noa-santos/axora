package com.axora.logger.sentry;

import com.axora.logger.WideEventPayload;
import com.axora.logger.sinks.Sink;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.concurrent.CompletableFuture;

public final class SentrySink implements Sink {
  private final SentryBridge bridge;

  public SentrySink(SentryBridge bridge) {
    this.bridge = bridge;
  }

  @Override
  public CompletableFuture<Void> export(WideEventPayload event) {
    if ("error".equals(event.status()) || "timeout".equals(event.status())) {
      bridge.withScope(
          scope -> {
            scope.setTag("service", event.service());
            scope.setTag("environment", event.environment());
            scope.setTag("operation", event.operation());
            scope.setTag("axora.event_id", event.eventId());
            scope.setExtras(event.context());
            scope.setLevel("fatal".equals(event.level()) ? "fatal" : event.level());

            RuntimeException error =
                new RuntimeException(event.error().message() != null ? event.error().message() : event.operation());
            bridge.captureException(error);
          });
      return CompletableFuture.completedFuture(null);
    }

    Map<String, Object> data = new LinkedHashMap<>(event.context());
    data.put("duration_ms", event.durationMs());
    data.put("axora_event_id", event.eventId());
    bridge.addBreadcrumb(
        event.operation(),
        event.operation(),
        event.level(),
        java.time.Instant.parse(event.timestampStart()).toEpochMilli() / 1000.0,
        data);
    return CompletableFuture.completedFuture(null);
  }
}
