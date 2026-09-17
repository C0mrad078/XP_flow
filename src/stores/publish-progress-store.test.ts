import { beforeEach, describe, expect, it } from "vitest";

import { usePublishProgressStore } from "./publish-progress-store";
import type { PublishProgressEvent } from "@/types/publishing";

function sampleEvent(overrides: Partial<PublishProgressEvent> = {}): PublishProgressEvent {
  return {
    publication_id: "pub-1",
    attempt_id: "attempt-1",
    platform: "youtube",
    bytes_uploaded: 1024,
    bytes_total: 2048,
    percentage: 50,
    phase: "uploading",
    ...overrides,
  };
}

describe("usePublishProgressStore", () => {
  beforeEach(() => {
    usePublishProgressStore.setState({ progress: {} });
  });

  it("starts empty", () => {
    expect(usePublishProgressStore.getState().progress).toEqual({});
  });

  it("records an event keyed by publication_id", () => {
    usePublishProgressStore.getState().record(sampleEvent());
    expect(usePublishProgressStore.getState().progress["pub-1"]).toMatchObject({ bytes_uploaded: 1024 });
  });

  it("overwrites with the latest event for the same publication", () => {
    usePublishProgressStore.getState().record(sampleEvent({ bytes_uploaded: 1024 }));
    usePublishProgressStore.getState().record(sampleEvent({ bytes_uploaded: 2000 }));
    expect(usePublishProgressStore.getState().progress["pub-1"]?.bytes_uploaded).toBe(2000);
  });

  it("keeps progress for different publications independent", () => {
    usePublishProgressStore.getState().record(sampleEvent({ publication_id: "pub-1" }));
    usePublishProgressStore.getState().record(sampleEvent({ publication_id: "pub-2", bytes_uploaded: 500 }));
    expect(usePublishProgressStore.getState().progress["pub-1"]?.bytes_uploaded).toBe(1024);
    expect(usePublishProgressStore.getState().progress["pub-2"]?.bytes_uploaded).toBe(500);
  });

  it("clear removes only the targeted publication", () => {
    usePublishProgressStore.getState().record(sampleEvent({ publication_id: "pub-1" }));
    usePublishProgressStore.getState().record(sampleEvent({ publication_id: "pub-2" }));
    usePublishProgressStore.getState().clear("pub-1");
    expect(usePublishProgressStore.getState().progress["pub-1"]).toBeUndefined();
    expect(usePublishProgressStore.getState().progress["pub-2"]).toBeDefined();
  });

  it("clear is a harmless no-op for a publication with no recorded progress", () => {
    const before = usePublishProgressStore.getState().progress;
    usePublishProgressStore.getState().clear("never-seen");
    expect(usePublishProgressStore.getState().progress).toBe(before);
  });
});
