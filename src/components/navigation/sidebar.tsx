import { ChevronsLeft, ChevronsRight, Sparkles } from "lucide-react";
import { NavLink } from "react-router-dom";

import { NAV_GROUPS } from "@/app/routes/nav-items";
import { IconButton } from "@/components/ui/icon-button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utilities/cn";
import { useUIStore } from "@/stores/ui-store";

export function Sidebar() {
  const collapsed = useUIStore((state) => state.sidebarCollapsed);
  const toggleSidebar = useUIStore((state) => state.toggleSidebar);

  return (
    <aside
      className={cn(
        "flex h-full shrink-0 flex-col border-r border-border bg-surface transition-[width] duration-[var(--animate-duration-base)]",
        collapsed ? "w-[68px]" : "w-60",
      )}
    >
      <div className={cn("flex h-14 shrink-0 items-center gap-2 px-4", collapsed && "justify-center px-0")}>
        <div className="flex size-7 shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground">
          <Sparkles className="size-4" />
        </div>
        {!collapsed && (
          <span className="text-section-title font-semibold tracking-tight text-foreground">XP FLOW</span>
        )}
      </div>

      <nav className="flex flex-1 flex-col gap-5 overflow-y-auto px-3 py-2">
        {NAV_GROUPS.map((group) => (
          <div key={group.label} className="flex flex-col gap-1">
            {!collapsed && <p className="px-2 pb-1 text-caption">{group.label}</p>}
            {group.items.map((item) => (
              <NavRow
                key={item.path}
                label={item.label}
                path={item.path}
                icon={item.icon}
                collapsed={collapsed}
              />
            ))}
          </div>
        ))}
      </nav>

      <div
        className={cn(
          "flex h-12 shrink-0 items-center border-t border-border px-3",
          collapsed && "justify-center px-0",
        )}
      >
        <IconButton
          label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          size="sm"
          onClick={toggleSidebar}
          className="text-muted-foreground"
        >
          {collapsed ? <ChevronsRight className="size-4" /> : <ChevronsLeft className="size-4" />}
        </IconButton>
      </div>
    </aside>
  );
}

interface NavRowProps {
  label: string;
  path: string;
  icon: (typeof NAV_GROUPS)[number]["items"][number]["icon"];
  collapsed: boolean;
}

function NavRow({ label, path, icon: Icon, collapsed }: NavRowProps) {
  const link = (
    <NavLink
      to={path}
      className={({ isActive }) =>
        cn(
          "group flex items-center gap-2.5 rounded-md px-2 py-1.5 text-sm font-medium text-muted-foreground",
          "transition-colors hover:bg-surface-hover hover:text-foreground",
          collapsed && "justify-center px-0 py-2",
          isActive && "bg-primary/10 text-primary hover:bg-primary/10 hover:text-primary",
        )
      }
    >
      <Icon className="size-4 shrink-0" />
      {!collapsed && <span className="truncate">{label}</span>}
    </NavLink>
  );

  if (!collapsed) return link;

  return (
    <Tooltip>
      <TooltipTrigger asChild>{link}</TooltipTrigger>
      <TooltipContent side="right">{label}</TooltipContent>
    </Tooltip>
  );
}
