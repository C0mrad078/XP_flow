import type { ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const previewMetadataMock = vi.hoisted(() => vi.fn());

vi.mock("@/lib/tauri", () => ({
  publishingApi: { previewMetadata: previewMetadataMock },
}));

const { TikTokConsentDialog, BulkTikTokApprovalDialog } = await import("./tiktok-consent-dialog");
import type { Publication } from "@/types/domain";

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

function samplePublication(overrides: Partial<Publication> = {}): Publication {
  return {
    id: "pub-1",
    workspace_id: "ws-1",
    video_id: "video-1",
    channel_id: "channel-1",
    platform_account_id: null,
    platform: "tiktok",
    status: "scheduled",
    title: "Amazing goal",
    description: null,
    hashtags: [],
    priority: "normal",
    locked: false,
    scheduled_at: null,
    published_at: null,
    remote_id: null,
    retry_count: 0,
    last_error: null,
    last_error_code: null,
    created_at: "2024-01-01T00:00:00Z",
    updated_at: "2024-01-01T00:00:00Z",
    ...overrides,
  };
}

describe("TikTokConsentDialog", () => {
  it("shows the publication title and caption once loaded", async () => {
    previewMetadataMock.mockResolvedValueOnce({
      title: "Rendered caption #shorts",
      description: "",
      hashtags: [],
      provider_options: { privacy_level: "SELF_ONLY", disable_duet: true },
    });

    render(
      <TikTokConsentDialog
        publication={samplePublication()}
        open
        onOpenChange={() => {}}
        onApprove={() => {}}
        isApproving={false}
      />,
      { wrapper },
    );

    expect(screen.getByText("Amazing goal")).toBeInTheDocument();
    expect(await screen.findByText("Rendered caption #shorts")).toBeInTheDocument();
    expect(await screen.findByText("SELF_ONLY")).toBeInTheDocument();
    expect(await screen.findByText("Disabled")).toBeInTheDocument();
  });

  it("calls onApprove when the approve button is clicked", async () => {
    previewMetadataMock.mockResolvedValueOnce({
      title: "Caption",
      description: "",
      hashtags: [],
      provider_options: {},
    });
    const onApprove = vi.fn();

    render(
      <TikTokConsentDialog
        publication={samplePublication()}
        open
        onOpenChange={() => {}}
        onApprove={onApprove}
        isApproving={false}
      />,
      { wrapper },
    );

    fireEvent.click(await screen.findByText("Approve for publishing"));
    expect(onApprove).toHaveBeenCalledTimes(1);
  });

  it("disables both actions while approving", () => {
    previewMetadataMock.mockResolvedValueOnce({
      title: "Caption",
      description: "",
      hashtags: [],
      provider_options: {},
    });

    render(
      <TikTokConsentDialog
        publication={samplePublication()}
        open
        onOpenChange={() => {}}
        onApprove={() => {}}
        isApproving
      />,
      { wrapper },
    );

    expect(screen.getByText("Approve for publishing").closest("button")).toBeDisabled();
    expect(screen.getByText("Cancel").closest("button")).toBeDisabled();
  });
});

describe("BulkTikTokApprovalDialog", () => {
  const publications = [
    samplePublication({ id: "pub-1", title: "Clip one", channel_id: "channel-1" }),
    samplePublication({ id: "pub-2", title: "Clip two", channel_id: "channel-2" }),
  ];
  const channelNames = new Map([
    ["channel-1", "Football BR"],
    ["channel-2", "Football US"],
  ]);

  it("defaults to every publication selected", () => {
    const onApprove = vi.fn();
    render(
      <BulkTikTokApprovalDialog
        publications={publications}
        open
        onOpenChange={() => {}}
        onApprove={onApprove}
        isApproving={false}
        channelNames={channelNames}
      />,
    );

    expect(screen.getByText("Approve 2 selected")).toBeInTheDocument();
    fireEvent.click(screen.getByText("Approve 2 selected"));
    expect(onApprove).toHaveBeenCalledWith(["pub-1", "pub-2"]);
  });

  it("excludes an unchecked publication from the approval call", () => {
    const onApprove = vi.fn();
    render(
      <BulkTikTokApprovalDialog
        publications={publications}
        open
        onOpenChange={() => {}}
        onApprove={onApprove}
        isApproving={false}
        channelNames={channelNames}
      />,
    );

    fireEvent.click(screen.getByText("Clip one"));
    expect(screen.getByText("Approve 1 selected")).toBeInTheDocument();
    fireEvent.click(screen.getByText("Approve 1 selected"));
    expect(onApprove).toHaveBeenCalledWith(["pub-2"]);
  });

  it("disables the approve action when nothing is selected", () => {
    render(
      <BulkTikTokApprovalDialog
        publications={publications}
        open
        onOpenChange={() => {}}
        onApprove={() => {}}
        isApproving={false}
        channelNames={channelNames}
      />,
    );

    fireEvent.click(screen.getByText("Clip one"));
    fireEvent.click(screen.getByText("Clip two"));
    expect(screen.getByText("Approve 0 selected").closest("button")).toBeDisabled();
  });
});
