package com.axora.logger.posthog;

import com.axora.logger.WideEventPayload;
import com.axora.logger.sinks.Sink;
import java.time.Instant;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.concurrent.CompletableFuture;

public final class PosthogSink implements Sink {
  private final PosthogClientLike client;

  public PosthogSink(PosthogClientLike client) {
    this.client = client;
  }

  @Override
  public CompletableFuture<Void> export(WideEventPayload event) {
    String distinctId = event.context().containsKey("user_id")
        ? String.valueOf(event.context().get("user_id"))
        : "service:" + event.service();

    Map<String, Object> properties = new LinkedHashMap<>(event.context());
    properties.put("axora_event_id", event.eventId());
    properties.put("axora_service", event.service());
    properties.put("status", event.status());
    properties.put("level", event.level());
    properties.put("duration_ms", event.durationMs());
    if (event.error().message() != null) {
      properties.put("error_message", event.error().message());
    }

    client.capture(distinctId, event.operation(), properties, Instant.parse(event.timestampStart()));
    return CompletableFuture.completedFuture(null);
  }

  @Override
  public CompletableFuture<Void> flush() {
    client.shutdown();
    return CompletableFuture.completedFuture(null);
  }
}
