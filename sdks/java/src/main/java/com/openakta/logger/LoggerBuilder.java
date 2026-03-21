package com.openakta.logger;

import com.openakta.logger.sinks.Sink;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public class LoggerBuilder {
  private static final ObjectMapper MAPPER = new ObjectMapper();

  private String service;
  private String environment;
  private final List<Sink> sinks = new ArrayList<>();
  private Map<String, Object> defaultContext = new LinkedHashMap<>();

  public LoggerBuilder service(String service) {
    this.service = service;
    return this;
  }

  public LoggerBuilder environment(String environment) {
    this.environment = environment;
    return this;
  }

  public LoggerBuilder addSink(Sink sink) {
    this.sinks.add(sink);
    return this;
  }

  public LoggerBuilder sinks(List<Sink> sinks) {
    this.sinks.clear();
    this.sinks.addAll(sinks);
    return this;
  }

  public LoggerBuilder defaultContext(Map<String, Object> context) {
    this.defaultContext = deepCopy(context);
    return this;
  }

  public Logger build() {
    return new Logger(resolveService(service), resolveEnvironment(environment), sinks, defaultContext);
  }

  private static String resolveService(String candidate) {
    String resolved = candidate != null && !candidate.isBlank() ? candidate : System.getenv("OPENAKTA_SERVICE");
    if (resolved == null || resolved.isBlank()) {
      throw new IllegalStateException("Logger requires a service name or OPENAKTA_SERVICE");
    }
    return resolved;
  }

  private static String resolveEnvironment(String candidate) {
    String resolved = candidate != null && !candidate.isBlank() ? candidate : System.getenv("OPENAKTA_ENV");
    if (resolved == null || resolved.isBlank()) {
      return "production";
    }
    return switch (resolved) {
      case "production", "staging", "development" -> resolved;
      default -> "production";
    };
  }

  @SuppressWarnings("unchecked")
  private Map<String, Object> deepCopy(Map<String, Object> input) {
    return MAPPER.convertValue(input, new TypeReference<LinkedHashMap<String, Object>>() {});
  }
}
