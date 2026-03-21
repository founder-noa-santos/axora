"use client";

import {
  Plan,
  PlanContent,
  PlanDescription,
  PlanHeader,
  PlanTitle,
  PlanTrigger,
} from "@/components/ai-elements/plan";

const MOCK_STEPS = [
  { id: "1", label: "Survey codebase structure" },
  { id: "2", label: "Implement chat wrappers" },
  { id: "3", label: "Wire settings panels" },
];

export function ChatPlanPanel() {
  return (
    <Plan className="bg-muted/20 w-full shrink-0" defaultOpen>
      <PlanHeader>
        <div className="min-w-0 space-y-1 pr-2">
          <PlanTitle>Mission plan</PlanTitle>
          <PlanDescription>Mock plan · UI only</PlanDescription>
        </div>
        <PlanTrigger />
      </PlanHeader>
      <PlanContent>
        <ol className="text-muted-foreground list-decimal space-y-2 pl-4 text-xs">
          {MOCK_STEPS.map((s) => (
            <li key={s.id}>{s.label}</li>
          ))}
        </ol>
        <p className="text-muted-foreground mt-3 text-[10px]">
          Status: idle (no bridge)
        </p>
      </PlanContent>
    </Plan>
  );
}
