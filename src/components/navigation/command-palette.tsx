import { useEffect } from "react";
import { Command } from "cmdk";
import { LayoutDashboard, PanelLeft } from "lucide-react";
import { useNavigate } from "react-router-dom";

import { ALL_NAV_ITEMS } from "@/app/routes/nav-items";
import { cn } from "@/lib/utilities/cn";
import { useUIStore } from "@/stores/ui-store";

export function CommandPalette() {
  const open = useUIStore((state) => state.commandPaletteOpen);
  const setOpen = useUIStore((state) => state.setCommandPaletteOpen);
  const toggleSidebar = useUIStore((state) => state.toggleSidebar);
  const navigate = useNavigate();

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const isModifierPressed = event.metaKey || event.ctrlKey;
      if (isModifierPressed && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setOpen(!open);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, setOpen]);

  function go(path: string) {
    navigate(path);
    setOpen(false);
  }

  return (
    <Command.Dialog
      open={open}
      onOpenChange={setOpen}
      label="Command palette"
      className={cn(
        "fixed left-1/2 top-[18%] z-(--z-command-palette) w-full max-w-lg -translate-x-1/2",
        "overflow-hidden rounded-xl border border-border bg-surface-elevated shadow-2xl",
        "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95",
      )}
      shouldFilter
    >
      <div className="flex items-center border-b border-border px-3">
        <Command.Input
          autoFocus
          placeholder="Type a command or search…"
          className="h-12 w-full bg-transparent text-sm text-foreground outline-none placeholder:text-muted"
        />
        <kbd className="rounded border border-border px-1.5 py-0.5 font-mono-data text-[0.6875rem] text-muted">
          Esc
        </kbd>
      </div>

      <Command.List className="max-h-80 overflow-y-auto p-2">
        <Command.Empty className="px-3 py-8 text-center text-body-small text-muted-foreground">
          No results found.
        </Command.Empty>

        <Command.Group
          heading="Navigate"
          className="px-1 py-1 text-caption [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:pb-1"
        >
          <PaletteItem icon={LayoutDashboard} label="Go to Dashboard" onSelect={() => go("/")} />
          {ALL_NAV_ITEMS.map((item) => (
            <PaletteItem
              key={item.path}
              icon={item.icon}
              label={`Go to ${item.label}`}
              onSelect={() => go(item.path)}
            />
          ))}
        </Command.Group>

        <Command.Group
          heading="Actions"
          className="px-1 py-1 text-caption [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:pb-1"
        >
          <PaletteItem
            icon={PanelLeft}
            label="Toggle sidebar"
            onSelect={() => {
              toggleSidebar();
              setOpen(false);
            }}
          />
        </Command.Group>
      </Command.List>
    </Command.Dialog>
  );
}

interface PaletteItemProps {
  icon: typeof LayoutDashboard;
  label: string;
  onSelect: () => void;
}

function PaletteItem({ icon: Icon, label, onSelect }: PaletteItemProps) {
  return (
    <Command.Item
      onSelect={onSelect}
      className={cn(
        "flex cursor-pointer items-center gap-2.5 rounded-md px-2.5 py-2 text-sm text-foreground outline-none",
        "data-[selected=true]:bg-primary/10 data-[selected=true]:text-primary",
      )}
    >
      <Icon className="size-4 text-muted-foreground group-data-[selected=true]:text-primary" />
      {label}
    </Command.Item>
  );
}
