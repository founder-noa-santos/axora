import { z } from "zod";

export const MessageRoleSchema = z.enum([
  "user",
  "assistant",
  "tool",
  "system",
]);

export const ToolCallSchema = z.object({
  id: z.string(),
  name: z.string(),
  parameters: z.record(z.string(), z.unknown()),
  status: z.enum([
    "pending",
    "running",
    "input-available",
    "complete",
    "error",
  ]),
  result: z.string().optional(),
  error: z.string().optional(),
  requiresApproval: z.boolean().default(false),
});

export const MessageSchema = z.object({
  key: z.string(),
  role: MessageRoleSchema,
  content: z.string(),
  isStreaming: z.boolean().default(false),
  isComplete: z.boolean().default(false),
  reasoning: z
    .object({
      content: z.string(),
      durationMs: z.number(),
      isStreaming: z.boolean(),
      isComplete: z.boolean(),
    })
    .optional(),
  toolCalls: z.array(ToolCallSchema).optional(),
  checkpointId: z.string().optional(),
  versions: z
    .array(
      z.object({
        id: z.string(),
        content: z.string(),
      }),
    )
    .default([]),
  timestamp: z.string().datetime(),
});

export type Message = z.infer<typeof MessageSchema>;
export type ToolCall = z.infer<typeof ToolCallSchema>;
