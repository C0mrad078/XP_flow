import { ShieldCheck } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { usePublicationMetadata } from "@/hooks/use-publishing";
import type { Publication } from "@/types/domain";

/**
 * Section 45: what the user must see before a TikTok publication can be
 * approved — never a bare "Approve" button with no context. Every value
 * here comes from the same rendered-metadata resolution the engine
 * itself will use at execution time (docs/metadata-templates.md) —
 * never a guess about what will actually be sent.
 */
function ProviderOption({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-3 text-body-small">
      <span className="text-muted-foreground">{label}</span>
      <span className="text-foreground">{value}</span>
    </div>
  );
}

function describeProviderOptions(providerOptions: Record<string, unknown>) {
  const privacy = typeof providerOptions.privacy_level === "string" ? providerOptions.privacy_level : null;
  const bool = (key: string) => providerOptions[key] === true;
  return {
    privacy: privacy ?? "Account default (most restrictive allowed)",
    comments: bool("disable_comment") ? "Disabled" : "Allowed",
    duet: bool("disable_duet") ? "Disabled" : "Allowed",
    stitch: bool("disable_stitch") ? "Disabled" : "Allowed",
  };
}

export interface TikTokConsentDialogProps {
  publication: Publication;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onApprove: () => void;
  isApproving: boolean;
}

/** Single-publication approval (section 45) — the mandatory detail view
 * before a TikTok publication can ever be approved. */
export function TikTokConsentDialog({
  publication,
  open,
  onOpenChange,
  onApprove,
  isApproving,
}: TikTokConsentDialogProps) {
  const metadata = usePublicationMetadata(open ? publication.id : null);
  const options = metadata.data ? describeProviderOptions(metadata.data.provider_options) : null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Approve for TikTok</DialogTitle>
          <DialogDescription>
            TikTok requires your explicit approval of this exact content before XP FLOW can publish it. If the
            caption or these settings change afterward, approval is required again.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-3 rounded-md border border-border bg-surface p-3">
          <div>
            <p className="text-caption font-medium uppercase tracking-wider text-muted-foreground">Video</p>
            <p className="mt-1 text-body-small text-foreground">{publication.title}</p>
          </div>
          <div>
            <p className="text-caption font-medium uppercase tracking-wider text-muted-foreground">Caption</p>
            <p className="mt-1 whitespace-pre-wrap text-body-small text-foreground">
              {metadata.isLoading ? "Loading…" : (metadata.data?.title ?? publication.title)}
            </p>
          </div>
          {options && (
            <div className="flex flex-col gap-1.5 border-t border-border pt-3">
              <ProviderOption label="Privacy" value={options.privacy} />
              <ProviderOption label="Comments" value={options.comments} />
              <ProviderOption label="Duet" value={options.duet} />
              <ProviderOption label="Stitch" value={options.stitch} />
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isApproving}>
            Cancel
          </Button>
          <Button onClick={onApprove} disabled={isApproving}>
            <ShieldCheck className="size-3.5" />
            Approve for publishing
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export interface BulkTikTokApprovalDialogProps {
  publications: Publication[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onApprove: (selectedIds: string[]) => void;
  isApproving: boolean;
  channelNames: Map<string, string>;
}

/** Bulk approval (section 47) — a plain summary + per-item opt-out
 * checklist, not a hidden "approve everything forever" toggle (section
 * 48: explicit, per-publication approval semantics are preserved even
 * in bulk — this still records one real consent row per publication). */
export function BulkTikTokApprovalDialog({
  publications,
  open,
  onOpenChange,
  onApprove,
  isApproving,
  channelNames,
}: BulkTikTokApprovalDialogProps) {
  const [excluded, setExcluded] = useState<Set<string>>(new Set());
  const selectedCount = publications.length - excluded.size;

  function toggle(id: string) {
    setExcluded((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Approve {publications.length} TikTok publications</DialogTitle>
          <DialogDescription>
            Each publication is approved individually against its own current caption — this records one real
            approval per item, not a blanket future toggle.
          </DialogDescription>
        </DialogHeader>

        <div className="flex max-h-72 flex-col gap-2 overflow-y-auto rounded-md border border-border p-2">
          {publications.map((publication) => (
            <label
              key={publication.id}
              className="flex items-center gap-2 rounded-md px-2 py-1.5 text-body-small hover:bg-surface-elevated"
            >
              <input
                type="checkbox"
                checked={!excluded.has(publication.id)}
                onChange={() => toggle(publication.id)}
                className="size-3.5"
              />
              <span className="min-w-0 flex-1 truncate">{publication.title}</span>
              <span className="text-caption text-muted-foreground">
                {channelNames.get(publication.channel_id) ?? "Unknown channel"}
              </span>
            </label>
          ))}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isApproving}>
            Cancel
          </Button>
          <Button
            onClick={() => onApprove(publications.filter((p) => !excluded.has(p.id)).map((p) => p.id))}
            disabled={isApproving || selectedCount === 0}
          >
            <ShieldCheck className="size-3.5" />
            Approve {selectedCount} selected
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
