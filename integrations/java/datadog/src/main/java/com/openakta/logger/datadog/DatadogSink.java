package com.openakta.logger.datadog;

import com.openakta.logger.WideEventPayload;
import com.openakta.logger.sinks.Sink;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.concurrent.CompletableFuture;

public final class DatadogSink implements Sink {
  private static final ObjectMapper MAPPER = new ObjectMapper();

  @Override
  public CompletableFuture<Void> export(WideEventPayload event) {
    Map<String, Object> logEntry = new LinkedHashMap<>();
    logEntry.put("date", event.timestampStart());
    logEntry.put("status", event.level());
    logEntry.put("service", event.service());
    logEntry.put("message", event.operation());
    logEntry.put("duration", event.durationMs());
    logEntry.putAll(event.context());
    logEntry.put("dd.openakta_event_id", event.eventId());
    logEntry.put("dd.env", event.environment());

    if (event.error().message() != null) {
      logEntry.put(
          "error",
          Map.of(
              "kind", event.error().type(),
              "message", event.error().message(),
              "stack", event.error().stack()));
    }

    try {
      System.out.println(MAPPER.writeValueAsString(logEntry));
      return CompletableFuture.completedFuture(null);
    } catch (Exception ex) {
      return CompletableFuture.failedFuture(ex);
    }
  }
}
