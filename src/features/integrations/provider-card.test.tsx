import type { ComponentProps } from "react";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ProviderCard } from "./provider-card";
import type { Channel } from "@/lib/tauri";
import type { PlatformAccount } from "@/types/platform-auth";
import type { ProviderConfigurationHealth } from "@/types/provider-configuration-health";

const channelsById = new Map<string, Channel>();

function sampleAccount(overrides: Partial<PlatformAccount> = {}): PlatformAccount {
  return {
    id: "acct-1",
    workspace_id: "ws-1",
    channel_id: "channel-1",
    platform: "youtube",
    provider_account_id: "UC123",
    provider_connection_id: null,
    display_name: "My Channel",
    username_or_handle: "@mychannel",
    avatar_url: null,
    status: "connected",
    granted_scopes: [],
    capabilities: [],
    default_target: true,
    access_expires_at: null,
    refresh_expires_at: null,
    connected_at: "2024-01-01T00:00:00Z",
    last_validated_at: null,
    last_refreshed_at: null,
    last_error_code: null,
    last_error_message: null,
    created_at: "2024-01-01T00:00:00Z",
    updated_at: "2024-01-01T00:00:00Z",
    ...overrides,
  };
}

function health(overrides: Partial<ProviderConfigurationHealth> = {}): ProviderConfigurationHealth {
  return {
    platform: "youtube",
    available: true,
    status: "ready",
    missing_configuration: [],
    user_message: null,
    ...overrides,
  };
}

function renderCard(props: Partial<ComponentProps<typeof ProviderCard>> = {}) {
  return render(
    <ProviderCard
      platform="youtube"
      accounts={[]}
      channelsById={channelsById}
      health={undefined}
      healthLoading={false}
      healthQueryFailed={false}
      onConnect={vi.fn()}
      onManage={vi.fn()}
      onRetryHealth={vi.fn()}
      {...props}
    />,
  );
}

describe("ProviderCard", () => {
  it("shows 'Continue with Google' for a ready YouTube card", () => {
    renderCard({ health: health({ platform: "youtube", status: "ready" }) });
    expect(screen.getByRole("button", { name: /continue with google/i })).toBeEnabled();
    expect(screen.getByText("Ready to connect")).toBeInTheDocument();
  });

  it("shows a plain 'Connect account' for a ready TikTok/Kwai card", () => {
    renderCard({
      platform: "tiktok",
      health: health({ platform: "tiktok", status: "ready" }),
    });
    expect(screen.getByRole("button", { name: /connect account/i })).toBeEnabled();
  });

  it("renders a disabled Configure action and the missing config when configuration is required", () => {
    renderCard({
      platform: "tiktok",
      health: health({
        platform: "tiktok",
        available: false,
        status: "configuration_required",
        missing_configuration: ["TIKTOK_CLIENT_KEY"],
        user_message: "TikTok developer configuration is missing.",
      }),
    });
    expect(screen.getByRole("button", { name: /configure/i })).toBeDisabled();
    expect(screen.getByText("Configuration required")).toBeInTheDocument();
    expect(screen.getByText("TikTok developer configuration is missing.")).toBeInTheDocument();
    expect(screen.getByText("TIKTOK_CLIENT_KEY")).toBeInTheDocument();
  });

  it("renders a Retry action when the broker is unavailable, never hiding the card", () => {
    renderCard({
      platform: "kwai",
      health: health({
        platform: "kwai",
        available: false,
        status: "broker_unavailable",
        user_message: "Authentication service unavailable.",
      }),
    });
    expect(screen.getAllByText("Kwai").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: /retry/i })).toBeEnabled();
    expect(screen.getByText("Service unavailable")).toBeInTheDocument();
  });

  it("renders a skeleton, not an empty panel, while configuration health is loading", () => {
    const { container } = renderCard({ healthLoading: true, health: undefined });
    expect(container.querySelector(".animate-pulse")).not.toBeNull();
  });

  it("still renders the card with a Retry action when the configuration query itself failed", () => {
    renderCard({ health: undefined, healthQueryFailed: true });
    expect(screen.getAllByText("YouTube").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: /retry/i })).toBeEnabled();
  });

  it("lists a connected account and lets it be managed", () => {
    renderCard({
      accounts: [sampleAccount()],
      health: health({ status: "ready" }),
    });
    expect(screen.getByText("My Channel")).toBeInTheDocument();
    expect(screen.getByText("Manage")).toBeInTheDocument();
  });

  it("lists an account needing reauthorization alongside a working Connect action", () => {
    renderCard({
      accounts: [sampleAccount({ status: "reauth_required", display_name: "Needs reauth" })],
      health: health({ status: "ready" }),
    });
    expect(screen.getByText("Needs reauth")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /continue with google/i })).toBeEnabled();
  });
});
