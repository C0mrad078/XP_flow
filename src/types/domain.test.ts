import { describe, expect, it } from "vitest";

import { isPublicationOverdue, type Publication } from "./domain";

function samplePublication(overrides: Partial<Publication> = {}): Publication {
  return {
    id: "pub-1",
    workspace_id: "ws-1",
    video_id: "video-1",
    channel_id: "channel-1",
    platform_account_id: null,
    platform: "youtube",
    status: "scheduled",
    title: "Sample",
    description: null,
    hashtags: [],
    priority: "normal",
    locked: false,
    scheduled_at: null,
    published_at: null,
    remote_id: null,
    retry_count: 0,
    last_error: null,
    created_at: "2024-01-01T00:00:00Z",
    updated_at: "2024-01-01T00:00:00Z",
    ...overrides,
  };
}

describe("isPublicationOverdue", () => {
  const now = new Date("2024-06-15T12:00:00Z");

  it("is false when not scheduled", () => {
    expect(isPublicationOverdue(samplePublication({ status: "queued", scheduled_at: null }), now)).toBe(
      false,
    );
  });

  it("is true for a Scheduled publication whose time has passed", () => {
    const publication = samplePublication({ status: "scheduled", scheduled_at: "2024-06-15T10:00:00Z" });
    expect(isPublicationOverdue(publication, now)).toBe(true);
  });

  it("is false for a Scheduled publication still in the future", () => {
    const publication = samplePublication({ status: "scheduled", scheduled_at: "2024-06-15T14:00:00Z" });
    expect(isPublicationOverdue(publication, now)).toBe(false);
  });

  it("is false once the status has moved past Scheduled, even if the time has passed", () => {
    const publication = samplePublication({ status: "uploading", scheduled_at: "2024-06-15T10:00:00Z" });
    expect(isPublicationOverdue(publication, now)).toBe(false);
  });
});
