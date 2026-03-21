"use client";

import {
  Checkpoint,
  CheckpointIcon,
  CheckpointTrigger,
} from "@/components/ai-elements/checkpoint";

export function ChatCheckpoint({ checkpointId }: { checkpointId: string }) {
  return (
    <Checkpoint className="mt-2">
      <CheckpointTrigger>
        <CheckpointIcon />
        <span className="text-muted-foreground text-xs">
          Checkpoint · {checkpointId.slice(0, 8)}…
        </span>
      </CheckpointTrigger>
    </Checkpoint>
  );
}
