import type { ReactNode } from "react";

export function EmptyState({
  icon,
  title,
  body,
}: {
  icon: ReactNode;
  title: string;
  body: string;
}) {
  return (
    <div className="rounded-[18px] border border-dashed border-white/10 bg-black/10 p-5">
      <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-[14px] bg-white/6">
        {icon}
      </div>
      <h3 className="text-[14px] font-semibold tracking-[-0.01em]">{title}</h3>
      <p className="mt-2 muted-copy">{body}</p>
    </div>
  );
}
