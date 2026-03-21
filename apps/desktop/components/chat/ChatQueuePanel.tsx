"use client";

import {
  Queue,
  QueueItem,
  QueueItemContent,
  QueueItemIndicator,
  QueueList,
  QueueSection,
  QueueSectionContent,
  QueueSectionLabel,
  QueueSectionTrigger,
} from "@/components/ai-elements/queue";

const MOCK_ITEMS = [
  { id: "t1", title: "Audit preference toggles", status: "running" as const },
  {
    id: "t2",
    title: "Install AI Elements registry",
    status: "pending" as const,
  },
];

export function ChatQueuePanel() {
  return (
    <Queue className="bg-muted/20 w-full flex-1 overflow-hidden">
      <QueueSection defaultOpen>
        <QueueSectionTrigger>
          <QueueSectionLabel label="tasks (mock)" count={MOCK_ITEMS.length} />
        </QueueSectionTrigger>
        <QueueSectionContent>
          <QueueList>
            {MOCK_ITEMS.map((item) => (
              <QueueItem key={item.id}>
                <QueueItemIndicator />
                <QueueItemContent>
                  <span className="text-xs font-medium">{item.title}</span>
                  <span className="text-muted-foreground block text-[10px] capitalize">
                    {item.status}
                  </span>
                </QueueItemContent>
              </QueueItem>
            ))}
          </QueueList>
        </QueueSectionContent>
      </QueueSection>
    </Queue>
  );
}
