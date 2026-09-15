import {
  Activity,
  BarChart3,
  Calendar,
  type LucideIcon,
  Film,
  ListVideo,
  MessageSquare,
  Radio,
  Settings,
  Sun,
  Workflow,
} from "lucide-react";

export interface NavItem {
  label: string;
  path: string;
  icon: LucideIcon;
}

export interface NavGroup {
  label: string;
  items: NavItem[];
}

/**
 * Single source of truth for the app's navigation (section 23). Both the
 * Sidebar and the Command Palette read from this list, so "go to X" never
 * drifts out of sync with what's actually in the sidebar.
 */
export const NAV_GROUPS: NavGroup[] = [
  {
    label: "Operate",
    items: [
      { label: "Today", path: "/today", icon: Sun },
      { label: "Queue", path: "/queue", icon: ListVideo },
      { label: "Content", path: "/content", icon: Film },
      { label: "Calendar", path: "/calendar", icon: Calendar },
    ],
  },
  {
    label: "Network",
    items: [
      { label: "Channels", path: "/channels", icon: Radio },
      { label: "Comments", path: "/comments", icon: MessageSquare },
    ],
  },
  {
    label: "Insights",
    items: [{ label: "Analytics", path: "/analytics", icon: BarChart3 }],
  },
  {
    label: "System",
    items: [
      { label: "Activity", path: "/activity", icon: Activity },
      { label: "Automation", path: "/automation", icon: Workflow },
      { label: "Settings", path: "/settings", icon: Settings },
    ],
  },
];

export const ALL_NAV_ITEMS: NavItem[] = NAV_GROUPS.flatMap((group) => group.items);
