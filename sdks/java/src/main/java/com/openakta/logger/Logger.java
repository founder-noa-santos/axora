package com.openakta.logger;

import com.openakta.logger.sinks.Sink;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.concurrent.CompletableFuture;
import java.util.function.Function;

public class Logger {
  private final String service;
  private final String environment;
  private final List<Sink> sinks;
  private final Map<String, Object> defaultContext;

  Logger(String service, String environment, List<Sink> sinks, Map<String, Object> defaultContext) {
    this.service = service;
    this.environment = environment;
    this.sinks = List.copyOf(sinks);
    this.defaultContext = new java.util.LinkedHashMap<>(defaultContext);
  }

  public static LoggerBuilder builder() {
    return new LoggerBuilder();
  }

  public WideEvent startEvent(String operation) {
    WideEvent event = new WideEvent(operation, service, environment, sinks);
    if (!defaultContext.isEmpty()) {
      event.appendContext(defaultContext);
    }
    return event;
  }

  public <T> CompletableFuture<T> trace(String operation, Function<WideEvent, CompletableFuture<T>> fn) {
    WideEvent event = startEvent(operation);
    CompletableFuture<T> future;
    try {
      future = Objects.requireNonNull(fn.apply(event), "trace function must not return null");
    } catch (Throwable throwable) {
      event.setError(throwable);
      return event.emitAsync().thenCompose(ignored -> CompletableFuture.failedFuture(throwable));
    }

    CompletableFuture<T> traced = new CompletableFuture<>();
    future.whenComplete(
        (result, throwable) -> {
          if (throwable != null) {
            Throwable cause = throwable.getCause() != null ? throwable.getCause() : throwable;
            event.setError(cause);
            event.emitAsync()
                .whenComplete(
                    (ignored, emitThrowable) -> traced.completeExceptionally(cause));
            return;
          }

          event.emitAsync()
              .whenComplete(
                  (ignored, emitThrowable) -> {
                    if (emitThrowable != null) {
                      traced.completeExceptionally(emitThrowable);
                    } else {
                      traced.complete(result);
                    }
                  });
        });
    return traced;
  }
}
