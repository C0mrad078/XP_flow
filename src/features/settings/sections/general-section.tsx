import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { useSettingsStore } from "@/stores/settings-store";
import { useWorkspaceStore } from "@/stores/workspace-store";

export function GeneralSection() {
  const workspace = useWorkspaceStore((state) => state.workspace);
  const settings = useSettingsStore((state) => state.settings);
  const setLaunchOnStartup = useSettingsStore((state) => state.setLaunchOnStartup);

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardHeader>
          <CardTitle>Workspace</CardTitle>
          <CardDescription>The workspace XP FLOW is currently operating in.</CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-body font-medium text-foreground">{workspace?.name ?? "—"}</p>
          <p className="text-caption normal-case tracking-normal">
            Multiple workspaces are not yet supported — this is Phase 1's single local workspace.
          </p>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Startup</CardTitle>
        </CardHeader>
        <CardContent className="flex items-center justify-between">
          <div>
            <p className="text-body-small font-medium text-foreground">Launch XP FLOW at login</p>
            <p className="text-caption normal-case tracking-normal">
              Starts the app automatically when you sign in.
            </p>
          </div>
          <Switch checked={settings.launch_on_startup} onCheckedChange={setLaunchOnStartup} />
        </CardContent>
      </Card>
    </div>
  );
}
