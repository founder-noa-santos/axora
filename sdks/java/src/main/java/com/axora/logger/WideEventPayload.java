package com.axora.logger;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Map;

public record WideEventPayload(
    @JsonProperty("event_id") String eventId,
    @JsonProperty("service") String service,
    @JsonProperty("environment") String environment,
    @JsonProperty("timestamp_start") String timestampStart,
    @JsonProperty("timestamp_end") String timestampEnd,
    @JsonProperty("duration_ms") double durationMs,
    @JsonProperty("level") String level,
    @JsonProperty("operation") String operation,
    @JsonProperty("status") String status,
    @JsonProperty("context") Map<String, Object> context,
    @JsonProperty("error") ErrorInfo error,
    @JsonProperty("meta") MetaInfo meta
) {
  public record ErrorInfo(
      @JsonProperty("type") String type,
      @JsonProperty("message") String message,
      @JsonProperty("stack") String stack) {}

  public record MetaInfo(
      @JsonProperty("sdk_version") String sdkVersion,
      @JsonProperty("sdk_language") String sdkLanguage) {}
}
