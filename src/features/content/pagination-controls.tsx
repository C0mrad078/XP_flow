import { ChevronLeft, ChevronRight } from "lucide-react";

import { Button } from "@/components/ui/button";
import { formatInteger } from "@/lib/formatting/number";

export function PaginationControls({
  page,
  pageSize,
  total,
  onPageChange,
}: {
  page: number;
  pageSize: number;
  total: number;
  onPageChange: (page: number) => void;
}) {
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const start = total === 0 ? 0 : page * pageSize + 1;
  const end = Math.min(total, (page + 1) * pageSize);

  if (total <= pageSize) return null;

  return (
    <div className="flex items-center justify-between border-t border-border pt-3">
      <p className="text-body-small text-muted-foreground">
        {formatInteger(start)}–{formatInteger(end)} of {formatInteger(total)}
      </p>
      <div className="flex items-center gap-2">
        <Button variant="outline" size="sm" disabled={page <= 0} onClick={() => onPageChange(page - 1)}>
          <ChevronLeft />
          Previous
        </Button>
        <span className="text-body-small text-muted-foreground">
          Page {page + 1} of {formatInteger(pageCount)}
        </span>
        <Button
          variant="outline"
          size="sm"
          disabled={page + 1 >= pageCount}
          onClick={() => onPageChange(page + 1)}
        >
          Next
          <ChevronRight />
        </Button>
      </div>
    </div>
  );
}
