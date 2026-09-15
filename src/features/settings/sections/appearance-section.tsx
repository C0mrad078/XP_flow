import { Check, Monitor, Moon, Sun } from "lucide-react";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utilities/cn";
import { useSettingsStore } from "@/stores/settings-store";
import type { ThemePreference } from "@/types/domain";

const THEME_OPTIONS: Array<{ value: ThemePreference; label: string; icon: typeof Sun }> = [
  { value: "dark", label: "Dark", icon: Moon },
  { value: "light", label: "Light", icon: Sun },
  { value: "system", label: "System", icon: Monitor },
];

export function AppearanceSection() {
  const theme = useSettingsStore((state) => state.settings.theme);
  const setTheme = useSettingsStore((state) => state.setTheme);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Theme</CardTitle>
        <CardDescription>
          Dark is XP FLOW's polished default. Light mode is available structurally.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-3 gap-3">
          {THEME_OPTIONS.map((option) => (
            <button
              key={option.value}
              type="button"
              onClick={() => setTheme(option.value)}
              className={cn(
                "flex flex-col items-center gap-2 rounded-lg border border-border bg-surface-elevated p-4 transition-colors",
                "hover:border-border-strong",
                theme === option.value && "border-primary bg-primary/5",
              )}
            >
              <div className="relative">
                <option.icon className="size-5 text-foreground" />
                {theme === option.value && (
                  <Check className="absolute -right-2 -top-2 size-3.5 rounded-full bg-primary p-0.5 text-primary-foreground" />
                )}
              </div>
              <span className="text-body-small font-medium text-foreground">{option.label}</span>
            </button>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
