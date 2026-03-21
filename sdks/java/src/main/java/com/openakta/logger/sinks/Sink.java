package com.openakta.logger.sinks;

import com.openakta.logger.WideEventPayload;
import java.util.concurrent.CompletableFuture;

public interface Sink {
  CompletableFuture<Void> export(WideEventPayload event);

  default CompletableFuture<Void> flush() {
    return CompletableFuture.completedFuture(null);
  }
}
