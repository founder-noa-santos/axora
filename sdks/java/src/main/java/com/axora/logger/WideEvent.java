package com.axora.logger;

import com.axora.logger.sinks.Sink;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.time.Instant;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;

public class WideEvent {
  private static final String SDK_VERSION = "0.1.0";
  private static final ObjectMapper MAPPER = new ObjectMapper();

  private final String eventId = UUID.randomUUID().toString();
  private final Instant timestampStart = Instant.now();
  private final long monotonicStartNanos = System.nanoTime();
  private final String operation;
  private final String service;
  private final String environment;
  private final List<Sink> sinks;
  private final Map<String, Object> context = new LinkedHashMap<>();
  private WideEventPayload.ErrorInfo error = new WideEventPayload.ErrorInfo(null, null, null);
  private String level = "info";
  private String status = "ok";
  private boolean finalized = false;
  private CompletableFuture<Void> emitFuture;

  WideEvent(String operation, String service, String environment, List<Sink> sinks) {
    this.operation = operation;
    this.service = service;
    this.environment = environment;
    this.sinks = List.copyOf(sinks);
  }

  private void ensureMutable() {
    if (finalized) {
      throw new IllegalStateException("WideEvent has already been finalized");
    }
  }

  public WideEvent appendContext(Map<String, Object> fields) {
    ensureMutable();
    context.putAll(deepCopy(fields));
    return this;
  }

  public WideEvent setError(Throwable throwable) {
    ensureMutable();
    level = "error";
    status = "error";
    error =
        new WideEventPayload.ErrorInfo(
            throwable.getClass().getSimpleName(),
            throwable.getMessage(),
            stackTraceToString(throwable));
    return this;
  }

  private Map<String, Object> deepCopy(Map<String, Object> fields) {
    return MAPPER.convertValue(fields, new TypeReference<LinkedHashMap<String, Object>>() {});
  }

  private static String stackTraceToString(Throwable throwable) {
    StringWriter sw = new StringWriter();
    throwable.printStackTrace(new PrintWriter(sw));
    return sw.toString();
  }

  private WideEventPayload buildPayload(String overrideLevel, String overrideStatus) {
    Map<String, Object> snapshotContext = deepCopy(context);
    WideEventPayload payload =
        new WideEventPayload(
            eventId,
            service,
            environment,
            timestampStart.toString(),
            Instant.now().toString(),
            Math.round(((System.nanoTime() - monotonicStartNanos) / 1_000_000.0) * 1000.0) / 1000.0,
            overrideLevel != null ? overrideLevel : level,
            operation,
            overrideStatus != null ? overrideStatus : status,
            snapshotContext,
            error,
            new WideEventPayload.MetaInfo(SDK_VERSION, "java"));
    return payload;
  }

  private CompletableFuture<Void> safeExport(Sink sink, WideEventPayload payload) {
    try {
      return sink.export(payload).exceptionally(ex -> null);
    } catch (Throwable throwable) {
      return CompletableFuture.completedFuture(null);
    }
  }

  public CompletableFuture<Void> emitAsync() {
    return emitAsync(null, null);
  }

  public CompletableFuture<Void> emitAsync(String overrideLevel, String overrideStatus) {
    if (emitFuture != null) {
      return emitFuture;
    }

    WideEventPayload payload = buildPayload(overrideLevel, overrideStatus);
    finalized = true;
    CompletableFuture<?>[] tasks =
        sinks.stream().map(sink -> safeExport(sink, payload)).toArray(CompletableFuture[]::new);
    emitFuture = CompletableFuture.allOf(tasks);
    return emitFuture;
  }
}
