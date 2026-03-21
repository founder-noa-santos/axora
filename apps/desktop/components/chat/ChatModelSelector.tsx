"use client";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Sparkles } from "lucide-react";

const MODELS = ["gpt-4o", "gpt-4.1", "claude-3-7-sonnet", "auto"];

export function ChatModelSelector({
  value,
  onValueChange,
}: {
  value: string;
  onValueChange: (v: string) => void;
}) {
  return (
    <div className="flex items-center gap-2">
      <Sparkles className="text-muted-foreground size-4" />
      <Select value={value} onValueChange={onValueChange}>
        <SelectTrigger className="h-8 w-[160px] border-none bg-transparent text-xs shadow-none focus:ring-0">
          <SelectValue placeholder="Model" />
        </SelectTrigger>
        <SelectContent>
          {MODELS.map((m) => (
            <SelectItem key={m} value={m} className="text-xs">
              {m}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
