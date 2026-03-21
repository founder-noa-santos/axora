package com.openakta.logger.sinks;

import com.openakta.logger.WideEventPayload;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;

public class HttpSink implements Sink {
  private static final ObjectMapper MAPPER = new ObjectMapper();
  private static final int DEFAULT_TIMEOUT_MS = 5000;

  private final HttpClient httpClient;
  private final URI uri;
  private final Map<String, String> headers;
  private final int timeoutMs;

  public HttpSink(String url) {
    this(url, Map.of(), DEFAULT_TIMEOUT_MS, HttpClient.newHttpClient());
  }

  public HttpSink(String url, Map<String, String> headers, int timeoutMs, HttpClient httpClient) {
    String resolvedUrl = url != null && !url.isBlank() ? url : System.getenv("OPENAKTA_SINK_URL");
    if (resolvedUrl == null || resolvedUrl.isBlank()) {
      throw new IllegalArgumentException("HttpSink requires a url or OPENAKTA_SINK_URL");
    }

    this.uri = URI.create(resolvedUrl);
    this.timeoutMs = timeoutMs > 0 ? timeoutMs : DEFAULT_TIMEOUT_MS;
    this.httpClient = httpClient;
    this.headers = new LinkedHashMap<>();
    this.headers.put("Content-Type", "application/json");
    if (headers != null) {
      this.headers.putAll(headers);
    }

    String token = System.getenv("OPENAKTA_SINK_TOKEN");
    if (token != null && !token.isBlank() && !this.headers.containsKey("Authorization")) {
      this.headers.put("Authorization", "Bearer " + token);
    }
  }

  @Override
  public CompletableFuture<Void> export(WideEventPayload event) {
    try {
      HttpRequest.Builder builder =
          HttpRequest.newBuilder(uri)
              .timeout(Duration.ofMillis(timeoutMs))
              .POST(HttpRequest.BodyPublishers.ofString(MAPPER.writeValueAsString(event)));
      headers.forEach(builder::header);

      return httpClient
          .sendAsync(builder.build(), HttpResponse.BodyHandlers.discarding())
          .thenApply(
              response -> {
                if (response.statusCode() >= 400) {
                  throw new CompletionException(
                      new IOException(
                          "HTTP " + response.statusCode() + " when exporting OPENAKTA Wide Event"));
                }
                return null;
              });
    } catch (JsonProcessingException ex) {
      return CompletableFuture.failedFuture(ex);
    }
  }
}
