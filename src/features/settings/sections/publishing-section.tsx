import { Pause, Play, Save } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import {
  useCreateHashtagSet,
  useCreateMetadataTemplate,
  useDeleteHashtagSet,
  useDeleteMetadataTemplate,
  useHashtagSets,
  useMetadataTemplates,
  usePausePublishing,
  usePublishingSettings,
  useResumePublishing,
  useUpdatePublishingSettings,
} from "@/hooks/use-publishing";

export function PublishingSection() {
  const { data: settings, isLoading } = usePublishingSettings();
  const update = useUpdatePublishingSettings();
  const pause = usePausePublishing();
  const resume = useResumePublishing();
  const [concurrency, setConcurrency] = useState<number | null>(null);
  const [grace, setGrace] = useState<number | null>(null);
  const templates = useMetadataTemplates();
  const hashtagSets = useHashtagSets();
  const createTemplate = useCreateMetadataTemplate();
  const deleteTemplate = useDeleteMetadataTemplate();
  const createHashtags = useCreateHashtagSet();
  const deleteHashtags = useDeleteHashtagSet();
  const [templateText, setTemplateText] = useState("{title} · {channel} {hashtags}");
  const [hashtagText, setHashtagText] = useState("#futebol #shorts");
  if (isLoading || !settings)
    return (
      <Card>
        <CardContent className="p-5 text-body-small text-muted-foreground">
          Loading publishing settings…
        </CardContent>
      </Card>
    );
  const max = concurrency ?? settings.max_concurrent_uploads;
  const graceValue = grace ?? settings.missed_schedule_grace_period_minutes;
  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardHeader>
          <CardTitle>Publishing engine</CardTitle>
          <CardDescription>
            Control the real upload worker without changing scheduled publications.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-body-small font-medium text-foreground">Publishing enabled</p>
              <p className="text-caption normal-case tracking-normal">
                Allows the scheduler to execute eligible publications.
              </p>
            </div>
            <Switch checked={settings.enabled} onCheckedChange={(enabled) => update.mutate({ enabled })} />
          </div>
          <div className="flex items-center justify-between rounded-md border border-border p-3">
            <div>
              <p className="text-body-small font-medium text-foreground">
                {settings.paused ? "Publishing paused" : "Publishing active"}
              </p>
              <p className="text-caption normal-case tracking-normal">
                Active uploads follow their safe backend lifecycle.
              </p>
            </div>
            {settings.paused ? (
              <Button size="sm" onClick={() => resume.mutate()} disabled={resume.isPending}>
                <Play className="size-3.5" />
                Resume
              </Button>
            ) : (
              <Button size="sm" variant="outline" onClick={() => pause.mutate()} disabled={pause.isPending}>
                <Pause className="size-3.5" />
                Pause publishing
              </Button>
            )}
          </div>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Execution policy</CardTitle>
          <CardDescription>Safe limits applied by the desktop worker.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <label className="flex items-center justify-between gap-4 text-body-small">
            <span>Maximum simultaneous uploads</span>
            <input
              className="h-9 w-20 rounded-md border border-border bg-surface px-2 text-center"
              type="number"
              min={1}
              max={10}
              value={max}
              onChange={(e) => setConcurrency(Math.max(1, Math.min(10, Number(e.target.value))))}
            />
          </label>
          <label className="flex items-center justify-between gap-4 text-body-small">
            <span>Missed schedule grace period (minutes)</span>
            <input
              className="h-9 w-24 rounded-md border border-border bg-surface px-2 text-center"
              type="number"
              min={0}
              max={1440}
              value={graceValue}
              onChange={(e) => setGrace(Math.max(0, Math.min(1440, Number(e.target.value))))}
            />
          </label>
          <label className="flex items-center justify-between gap-4 text-body-small">
            <span>Missed schedule policy</span>
            <select
              className="h-9 rounded-md border border-border bg-surface px-2"
              value={settings.missed_schedule_policy}
              onChange={(e) =>
                update.mutate({
                  missed_schedule_policy: e.target.value as typeof settings.missed_schedule_policy,
                })
              }
            >
              <option value="publish_within_grace">Publish within grace</option>
              <option value="needs_review">Needs review</option>
              <option value="skip">Skip execution</option>
            </select>
          </label>
          <Button
            size="sm"
            className="self-start"
            onClick={() =>
              update.mutate({ max_concurrent_uploads: max, missed_schedule_grace_period_minutes: graceValue })
            }
            disabled={update.isPending}
          >
            <Save className="size-3.5" />
            Save policy
          </Button>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Metadata templates</CardTitle>
          <CardDescription>
            Workspace defaults are the lowest template rung; publication overrides remain higher precedence.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <div className="flex gap-2">
            <input
              aria-label="Template text"
              className="h-9 min-w-0 flex-1 rounded-md border border-border bg-surface px-3 text-body-small"
              value={templateText}
              onChange={(e) => setTemplateText(e.target.value)}
            />
            <Button
              size="sm"
              onClick={() => createTemplate.mutate({ kind: "title", text: templateText })}
              disabled={createTemplate.isPending}
            >
              Add title template
            </Button>
          </div>
          {templates.data?.map((template) => (
            <div
              key={template.id}
              className="flex items-center justify-between gap-3 rounded-md border border-border px-3 py-2 text-body-small"
            >
              <span className="truncate">
                <span className="mr-2 text-caption uppercase text-muted-foreground">{template.kind}</span>
                {template.template_text}
              </span>
              <Button variant="ghost" size="sm" onClick={() => deleteTemplate.mutate(template.id)}>
                Delete
              </Button>
            </div>
          ))}
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Hashtag sets</CardTitle>
          <CardDescription>
            Reusable tags can be assigned by channel and platform through the metadata service.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <div className="flex gap-2">
            <input
              aria-label="Hashtags"
              className="h-9 min-w-0 flex-1 rounded-md border border-border bg-surface px-3 text-body-small"
              value={hashtagText}
              onChange={(e) => setHashtagText(e.target.value)}
            />
            <Button
              size="sm"
              onClick={() =>
                createHashtags.mutate({
                  name: "Default tags",
                  hashtags: hashtagText.split(/\s+/).filter(Boolean),
                })
              }
              disabled={createHashtags.isPending}
            >
              Add set
            </Button>
          </div>
          {hashtagSets.data?.map((set) => (
            <div
              key={set.id}
              className="flex items-center justify-between gap-3 rounded-md border border-border px-3 py-2 text-body-small"
            >
              <span className="truncate">
                <span className="mr-2 font-medium">{set.name}</span>
                {set.hashtags.join(" ")}
              </span>
              <Button variant="ghost" size="sm" onClick={() => deleteHashtags.mutate(set.id)}>
                Delete
              </Button>
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}
