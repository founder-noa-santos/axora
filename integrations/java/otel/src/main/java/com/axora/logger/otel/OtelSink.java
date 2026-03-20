package com.axora.logger.otel;

import com.axora.logger.WideEventPayload;
import com.axora.logger.sinks.Sink;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.concurrent.CompletableFuture;

public final class OtelSink implements Sink {
  private final OtelLoggerLike logger;

  public OtelSink(OtelLoggerProviderLike provider) {
    this.logger = provider.getLogger("axora-logger", "0.1.0");
  }

  @Override
  public CompletableFuture<Void> export(WideEventPayload event) {
    Map<String, Object> attributes = new LinkedHashMap<>();
    attributes.put("axora.event_id", event.eventId());
    attributes.put("axora.operation", event.operation());
    attributes.put("axora.status", event.status());
    attributes.put("axora.duration_ms", event.durationMs());
    attributes.put("service.name", event.service());
    attributes.put("deployment.environment.name", event.environment());

    for (var entry : event.context().entrySet()) {
      attributes.put("axora.ctx." + entry.getKey(), entry.getValue());
    }

    if (event.error().message() != null) {
      attributes.put("exception.type", event.error().type());
      attributes.put("exception.message", event.error().message());
      attributes.put("exception.stacktrace", event.error().stack());
    }

    Map<String, Object> record = new LinkedHashMap<>();
    record.put("severityNumber", severityNumber(event.level()));
    record.put("severityText", event.level().toUpperCase());
    record.put("body", event.operation());
    record.put("attributes", attributes);
    record.put("timestamp", java.time.Instant.parse(event.timestampEnd()).toEpochMilli() * 1_000_000);
    record.put("observedTimestamp", java.time.Instant.parse(event.timestampStart()).toEpochMilli() * 1_000_000);
    record.put(
        "resource",
        Map.of(
            "attributes",
            Map.of(
                "service.name", event.service(),
                "deployment.environment.name", event.environment())));

    logger.emit(record);
    return CompletableFuture.completedFuture(null);
  }

  private static int severityNumber(String level) {
    return switch (level) {
      case "info" -> 9;
      case "warn" -> 13;
      case "error" -> 17;
      case "fatal" -> 21;
      default -> 9;
    };
  }
}
