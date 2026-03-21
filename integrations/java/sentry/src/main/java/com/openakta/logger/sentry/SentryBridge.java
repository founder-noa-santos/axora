package com.openakta.logger.sentry;

import java.util.function.Consumer;

public interface SentryBridge {
  void withScope(Consumer<SentryScopeBridge> callback);

  void captureException(Throwable throwable);

  void addBreadcrumb(String category, String message, String level, double timestampSeconds, java.util.Map<String, Object> data);
}
