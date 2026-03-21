package com.openakta.logger.sentry;

import java.util.Map;

public interface SentryScopeBridge {
  void setTag(String key, String value);

  void setExtras(Map<String, Object> extras);

  void setLevel(String level);
}
