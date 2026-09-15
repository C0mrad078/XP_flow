import { useState } from "react";
import { Bell, HardDrive, Info, Palette, SlidersHorizontal, User } from "lucide-react";

import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";

import { AboutSection } from "./sections/about-section";
import { AdvancedSection } from "./sections/advanced-section";
import { AppearanceSection } from "./sections/appearance-section";
import { GeneralSection } from "./sections/general-section";
import { NotificationsSection } from "./sections/notifications-section";
import { StorageSection } from "./sections/storage-section";
import { SettingsNav, type SettingsSection } from "./settings-nav";

const SECTIONS: SettingsSection[] = [
  { id: "general", label: "General", icon: User },
  { id: "appearance", label: "Appearance", icon: Palette },
  { id: "storage", label: "Storage", icon: HardDrive },
  { id: "notifications", label: "Notifications", icon: Bell },
  { id: "advanced", label: "Advanced", icon: SlidersHorizontal },
  { id: "about", label: "About", icon: Info },
];

export function SettingsPage() {
  const [active, setActive] = useState("general");

  return (
    <PageContainer>
      <PageHeader title="Settings" description="Configure XP FLOW for how you work." />

      <div className="flex gap-8">
        <SettingsNav sections={SECTIONS} active={active} onSelect={setActive} />
        <div className="min-w-0 flex-1">
          {active === "general" && <GeneralSection />}
          {active === "appearance" && <AppearanceSection />}
          {active === "storage" && <StorageSection />}
          {active === "notifications" && <NotificationsSection />}
          {active === "advanced" && <AdvancedSection />}
          {active === "about" && <AboutSection />}
        </div>
      </div>
    </PageContainer>
  );
}
