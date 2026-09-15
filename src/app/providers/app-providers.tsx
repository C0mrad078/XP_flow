import type { ReactNode } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { HashRouter } from "react-router-dom";

import { TooltipProvider } from "@/components/ui/tooltip";
import { queryClient } from "@/lib/query-client";

export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider client={queryClient}>
      <HashRouter>
        <TooltipProvider delayDuration={300}>{children}</TooltipProvider>
      </HashRouter>
    </QueryClientProvider>
  );
}
