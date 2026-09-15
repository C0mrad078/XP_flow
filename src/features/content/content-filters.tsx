import type * as React from "react";
import { SlidersHorizontal, X } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useChannels } from "@/hooks/use-channels";
import { useSources } from "@/hooks/use-sources";
import { useContentStore } from "@/stores/content-store";
import { VALIDATION_STATUS_LABELS, VIDEO_PRIORITIES, VIDEO_PRIORITY_LABELS } from "@/types/media";

const ORIENTATION_LABELS = { vertical: "Vertical", square: "Square", landscape: "Landscape" } as const;

/** Compact popover filters + removable chips (section 76) — no giant
 * settings dialog for narrowing the library down. */
export function ContentFilters() {
  const filters = useContentStore((state) => state.filters);
  const setFilters = useContentStore((state) => state.setFilters);
  const { data: channels = [] } = useChannels();
  const { data: sources = [] } = useSources();

  const activeChips: Array<{ key: string; label: string; onRemove: () => void }> = [];
  if (filters.channelId) {
    const channel = channels.find((c) => c.id === filters.channelId);
    activeChips.push({
      key: "channel",
      label: `Channel: ${channel?.name ?? "…"}`,
      onRemove: () => setFilters({ channelId: undefined }),
    });
  }
  if (filters.sourceId) {
    const source = sources.find((s) => s.id === filters.sourceId);
    activeChips.push({
      key: "source",
      label: `Source: ${source?.name ?? "…"}`,
      onRemove: () => setFilters({ sourceId: undefined }),
    });
  }
  if (filters.orientation) {
    activeChips.push({
      key: "orientation",
      label: `Orientation: ${ORIENTATION_LABELS[filters.orientation]}`,
      onRemove: () => setFilters({ orientation: undefined }),
    });
  }
  if (filters.priority) {
    activeChips.push({
      key: "priority",
      label: `Priority: ${VIDEO_PRIORITY_LABELS[filters.priority]}`,
      onRemove: () => setFilters({ priority: undefined }),
    });
  }
  if (filters.validationStatus) {
    activeChips.push({
      key: "validation",
      label: `Status: ${VALIDATION_STATUS_LABELS[filters.validationStatus]}`,
      onRemove: () => setFilters({ validationStatus: undefined }),
    });
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-2">
        <Input
          value={filters.search}
          onChange={(e) => setFilters({ search: e.target.value })}
          placeholder="Search title, filename, notes…"
          className="w-64"
        />

        <Popover>
          <PopoverTrigger asChild>
            <Button variant="outline" size="sm">
              <SlidersHorizontal />
              Filters
            </Button>
          </PopoverTrigger>
          <PopoverContent align="start" className="flex w-72 flex-col gap-3">
            <FilterField label="Channel">
              <Select
                value={filters.channelId ?? "any"}
                onValueChange={(v) => setFilters({ channelId: v === "any" ? undefined : v })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="any">Any channel</SelectItem>
                  {channels.map((c) => (
                    <SelectItem key={c.id} value={c.id}>
                      {c.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </FilterField>

            <FilterField label="Source">
              <Select
                value={filters.sourceId ?? "any"}
                onValueChange={(v) => setFilters({ sourceId: v === "any" ? undefined : v })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="any">Any source</SelectItem>
                  {sources.map((s) => (
                    <SelectItem key={s.id} value={s.id}>
                      {s.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </FilterField>

            <FilterField label="Orientation">
              <Select
                value={filters.orientation ?? "any"}
                onValueChange={(v) =>
                  setFilters({ orientation: v === "any" ? undefined : (v as typeof filters.orientation) })
                }
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="any">Any orientation</SelectItem>
                  {Object.entries(ORIENTATION_LABELS).map(([value, label]) => (
                    <SelectItem key={value} value={value}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </FilterField>

            <FilterField label="Priority">
              <Select
                value={filters.priority ?? "any"}
                onValueChange={(v) =>
                  setFilters({ priority: v === "any" ? undefined : (v as typeof filters.priority) })
                }
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="any">Any priority</SelectItem>
                  {VIDEO_PRIORITIES.map((p) => (
                    <SelectItem key={p} value={p}>
                      {VIDEO_PRIORITY_LABELS[p]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </FilterField>

            <FilterField label="Validation status">
              <Select
                value={filters.validationStatus ?? "any"}
                onValueChange={(v) =>
                  setFilters({
                    validationStatus: v === "any" ? undefined : (v as typeof filters.validationStatus),
                  })
                }
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="any">Any status</SelectItem>
                  {Object.entries(VALIDATION_STATUS_LABELS).map(([value, label]) => (
                    <SelectItem key={value} value={value}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </FilterField>
          </PopoverContent>
        </Popover>
      </div>

      {activeChips.length > 0 && (
        <div className="flex flex-wrap items-center gap-1.5">
          {activeChips.map((chip) => (
            <Badge key={chip.key} variant="primary" className="gap-1 pr-1">
              {chip.label}
              <button
                type="button"
                onClick={chip.onRemove}
                className="rounded-full p-0.5 hover:bg-primary/20"
                aria-label={`Remove ${chip.label} filter`}
              >
                <X className="size-3" />
              </button>
            </Badge>
          ))}
        </div>
      )}
    </div>
  );
}

function FilterField({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-1">
      <label className="text-label text-foreground">{label}</label>
      {children}
    </div>
  );
}
