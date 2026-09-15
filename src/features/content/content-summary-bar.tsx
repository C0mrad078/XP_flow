import { useContentSummary } from "@/hooks/use-content";
import { formatInteger } from "@/lib/formatting/number";
import { cn } from "@/lib/utilities/cn";
import { useContentStore } from "@/stores/content-store";
import type { VideoLibrarySummary } from "@/types/media";

type TileKey = keyof VideoLibrarySummary;

const TILES: Array<{ key: TileKey; label: string; tone?: "warning" | "danger" }> = [
  { key: "total", label: "All" },
  { key: "ready", label: "Ready" },
  { key: "processing", label: "Processing" },
  { key: "duplicates", label: "Duplicates", tone: "warning" },
  { key: "invalid", label: "Invalid", tone: "danger" },
  { key: "missing", label: "Missing", tone: "danger" },
  { key: "unassigned", label: "Unassigned" },
  { key: "archived", label: "Archived" },
];

/** Real database-derived counts (section 74) — not client-side row
 * counting, which would require loading the whole library. */
export function ContentSummaryBar() {
  const { data: summary } = useContentSummary();
  const filters = useContentStore((state) => state.filters);
  const setFilters = useContentStore((state) => state.setFilters);

  function isActive(key: TileKey) {
    switch (key) {
      case "total":
        return (
          !filters.validationStatus &&
          !filters.availabilityStatus &&
          !filters.unassignedOnly &&
          !filters.possibleDuplicatesOnly &&
          !filters.includeArchived
        );
      case "ready":
        return filters.validationStatus === "valid" && filters.availabilityStatus === undefined;
      case "processing":
        return filters.validationStatus === "pending";
      case "duplicates":
        return filters.possibleDuplicatesOnly;
      case "invalid":
        return filters.validationStatus === "invalid";
      case "missing":
        return filters.availabilityStatus === "missing";
      case "unassigned":
        return filters.unassignedOnly;
      case "archived":
        return filters.includeArchived && !filters.validationStatus;
      default:
        return false;
    }
  }

  function apply(key: TileKey) {
    switch (key) {
      case "total":
        setFilters({
          validationStatus: undefined,
          availabilityStatus: undefined,
          unassignedOnly: false,
          possibleDuplicatesOnly: false,
          includeArchived: false,
        });
        return;
      case "ready":
        setFilters({
          validationStatus: "valid",
          availabilityStatus: undefined,
          unassignedOnly: false,
          possibleDuplicatesOnly: false,
        });
        return;
      case "processing":
        setFilters({ validationStatus: "pending", availabilityStatus: undefined });
        return;
      case "duplicates":
        setFilters({
          possibleDuplicatesOnly: true,
          validationStatus: undefined,
          availabilityStatus: undefined,
        });
        return;
      case "invalid":
        setFilters({ validationStatus: "invalid", availabilityStatus: undefined });
        return;
      case "missing":
        setFilters({ availabilityStatus: "missing", validationStatus: undefined });
        return;
      case "unassigned":
        setFilters({ unassignedOnly: true });
        return;
      case "archived":
        setFilters({ includeArchived: true, validationStatus: undefined, availabilityStatus: undefined });
        return;
    }
  }

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {TILES.map((tile) => {
        const value = summary?.[tile.key];
        const active = isActive(tile.key);
        return (
          <button
            key={tile.key}
            type="button"
            onClick={() => apply(tile.key)}
            className={cn(
              "flex items-center gap-1.5 rounded-full border px-3 py-1 text-xs font-medium transition-colors",
              active
                ? "border-primary bg-primary/10 text-primary"
                : "border-border bg-surface text-muted-foreground hover:bg-surface-hover",
              tile.tone === "danger" && !active && value ? "text-danger" : "",
              tile.tone === "warning" && !active && value ? "text-warning" : "",
            )}
          >
            <span className="font-mono-data">{value === undefined ? "–" : formatInteger(value)}</span>
            {tile.label}
          </button>
        );
      })}
    </div>
  );
}
