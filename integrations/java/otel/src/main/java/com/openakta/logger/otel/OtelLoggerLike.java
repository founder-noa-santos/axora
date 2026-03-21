package com.openakta.logger.otel;

import java.util.Map;

public interface OtelLoggerLike {
  void emit(Map<String, Object> record);
}
