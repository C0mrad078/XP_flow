import type { LucideIcon } from "lucide-react";

import { cn } from "@/lib/utilities/cn";

export interface SettingsSection {
  id: string;
  label: string;
  icon: LucideIcon;
}

export function SettingsNav({
  sections,
  active,
  onSelect,
}: {
  sections: SettingsSection[];
  active: string;
  onSelect: (id: string) => void;
}) {
  return (
    <nav className="flex w-48 shrink-0 flex-col gap-0.5">
      {sections.map((section) => (
        <button
          key={section.id}
          type="button"
          onClick={() => onSelect(section.id)}
          className={cn(
            "flex items-center gap-2.5 rounded-md px-2.5 py-1.5 text-left text-sm font-medium text-muted-foreground",
            "transition-colors hover:bg-surface-hover hover:text-foreground",
            active === section.id && "bg-primary/10 text-primary hover:bg-primary/10 hover:text-primary",
          )}
        >
          <section.icon className="size-4" />
          {section.label}
        </button>
      ))}
    </nav>
  );
}
