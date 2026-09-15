import { type FormEvent, useState } from "react";
import { Sparkles } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { toast } from "@/stores/toast-store";
import { useWorkspaceStore } from "@/stores/workspace-store";
import { isAppError } from "@/types/domain";

export function OnboardingScreen() {
  const createWorkspace = useWorkspaceStore((state) => state.createWorkspace);
  const [name, setName] = useState("");
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!name.trim() || submitting) return;

    setSubmitting(true);
    try {
      await createWorkspace(name.trim());
    } catch (error) {
      toast({
        variant: "error",
        title: "Couldn't create workspace",
        description: isAppError(error) ? error.user_message : "Please try again.",
      });
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="flex h-screen w-screen items-center justify-center bg-background px-6">
      <div className="flex w-full max-w-sm flex-col gap-6">
        <div className="flex flex-col items-center gap-3 text-center">
          <div className="flex size-12 items-center justify-center rounded-xl bg-primary text-primary-foreground">
            <Sparkles className="size-6" />
          </div>
          <div>
            <h1 className="text-display text-foreground">Welcome to XP FLOW</h1>
            <p className="text-body-small mt-1 text-muted-foreground">
              Let's set up your first workspace. You can rename it later.
            </p>
          </div>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col gap-3">
          <div className="flex flex-col gap-1.5">
            <label htmlFor="workspace-name" className="text-label text-foreground">
              Workspace name
            </label>
            <Input
              id="workspace-name"
              autoFocus
              placeholder="e.g. My Content Network"
              value={name}
              onChange={(e) => setName(e.target.value)}
              maxLength={80}
            />
          </div>
          <Button type="submit" disabled={!name.trim() || submitting} className="w-full">
            {submitting ? "Creating…" : "Create workspace"}
          </Button>
        </form>
      </div>
    </div>
  );
}
