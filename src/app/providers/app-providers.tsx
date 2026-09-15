import type { ReactNode } from "react";
import { HashRouter } from "react-router-dom";

import { TooltipProvider } from "@/components/ui/tooltip";

export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <HashRouter>
      <TooltipProvider delayDuration={300}>{children}</TooltipProvider>
    </HashRouter>
  );
}
