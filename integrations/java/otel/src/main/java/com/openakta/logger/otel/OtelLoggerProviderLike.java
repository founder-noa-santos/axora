package com.openakta.logger.otel;

public interface OtelLoggerProviderLike {
  OtelLoggerLike getLogger(String name, String version);
}
