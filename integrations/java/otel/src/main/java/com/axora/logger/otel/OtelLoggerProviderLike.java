package com.axora.logger.otel;

public interface OtelLoggerProviderLike {
  OtelLoggerLike getLogger(String name, String version);
}
