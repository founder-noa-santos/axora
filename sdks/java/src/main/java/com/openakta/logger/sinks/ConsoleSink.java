package com.openakta.logger.sinks;

import com.openakta.logger.WideEventPayload;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;

public class ConsoleSink implements Sink {
  private static final ObjectMapper MAPPER = new ObjectMapper();

  @Override
  public CompletableFuture<Void> export(WideEventPayload event) {
    return CompletableFuture.runAsync(
        () -> {
          try {
            System.out.println(MAPPER.writeValueAsString(event));
          } catch (JsonProcessingException ex) {
            throw new CompletionException(ex);
          }
        });
  }
}
