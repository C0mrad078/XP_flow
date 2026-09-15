import type { ReactNode } from "react";

import { cn } from "@/lib/utilities/cn";

export function PageContainer({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cn("mx-auto flex w-full max-w-[1400px] flex-col gap-6 px-6 py-6", className)}>
      {children}
    </div>
  );
}
