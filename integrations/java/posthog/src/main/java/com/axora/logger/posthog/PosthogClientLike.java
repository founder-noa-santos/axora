package com.axora.logger.posthog;

import java.time.Instant;
import java.util.Map;

public interface PosthogClientLike {
  void capture(String distinctId, String event, Map<String, Object> properties, Instant timestamp);

  void shutdown();
}
