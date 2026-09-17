import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const subscribeMock = vi.hoisted(() => vi.fn());

vi.mock("@/lib/tauri/publish-progress", () => ({
  subscribeToPublishProgress: subscribeMock,
}));

const { usePublishProgressListener, usePublicationProgress } = await import("./use-publish-progress");
const { usePublishProgressStore } = await import("@/stores/publish-progress-store");

describe("usePublishProgressListener", () => {
  beforeEach(() => {
    usePublishProgressStore.setState({ progress: {} });
    subscribeMock.mockReset();
  });

  it("subscribes once on mount and records incoming events into the shared store", async () => {
    let handler: ((event: unknown) => void) | undefined;
    const unlisten = vi.fn();
    subscribeMock.mockImplementation((cb: (event: unknown) => void) => {
      handler = cb;
      return Promise.resolve(unlisten);
    });

    renderHook(() => usePublishProgressListener());

    await waitFor(() => expect(subscribeMock).toHaveBeenCalledTimes(1));
    handler?.({
      publication_id: "pub-1",
      attempt_id: "a1",
      platform: "youtube",
      bytes_uploaded: 42,
      bytes_total: 100,
      percentage: 42,
      phase: "uploading",
    });

    expect(usePublishProgressStore.getState().progress["pub-1"]?.bytes_uploaded).toBe(42);
  });

  it("unsubscribes on unmount", async () => {
    const unlisten = vi.fn();
    subscribeMock.mockResolvedValueOnce(unlisten);

    const { unmount } = renderHook(() => usePublishProgressListener());
    await waitFor(() => expect(subscribeMock).toHaveBeenCalledTimes(1));
    unmount();

    await waitFor(() => expect(unlisten).toHaveBeenCalledTimes(1));
  });

  it("does not throw when no Tauri runtime is available", async () => {
    subscribeMock.mockRejectedValueOnce(new Error("no runtime"));
    expect(() => renderHook(() => usePublishProgressListener())).not.toThrow();
  });
});

describe("usePublicationProgress", () => {
  beforeEach(() => {
    usePublishProgressStore.setState({ progress: {} });
  });

  it("returns undefined when no event has been recorded for the publication", () => {
    const { result } = renderHook(() => usePublicationProgress("pub-1"));
    expect(result.current).toBeUndefined();
  });

  it("returns undefined for a null publication id", () => {
    usePublishProgressStore.getState().record({
      publication_id: "pub-1",
      attempt_id: "a1",
      platform: "youtube",
      bytes_uploaded: 1,
      bytes_total: 10,
      percentage: 10,
      phase: "uploading",
    });
    const { result } = renderHook(() => usePublicationProgress(null));
    expect(result.current).toBeUndefined();
  });

  it("returns the recorded event for a matching publication id", () => {
    usePublishProgressStore.getState().record({
      publication_id: "pub-1",
      attempt_id: "a1",
      platform: "tiktok",
      bytes_uploaded: 5,
      bytes_total: 10,
      percentage: 50,
      phase: "uploading",
    });
    const { result } = renderHook(() => usePublicationProgress("pub-1"));
    expect(result.current?.bytes_uploaded).toBe(5);
  });
});
