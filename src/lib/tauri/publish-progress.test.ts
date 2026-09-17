import { describe, expect, it, vi } from "vitest";

const listenMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

describe("subscribeToPublishProgress", () => {
  it("subscribes to the exact event name the backend emits", async () => {
    const { subscribeToPublishProgress } = await import("./publish-progress");
    const unlisten = vi.fn();
    listenMock.mockResolvedValueOnce(unlisten);

    await subscribeToPublishProgress(() => {});

    expect(listenMock).toHaveBeenCalledWith("publish-progress", expect.any(Function));
  });

  it("unwraps the event payload before calling the handler", async () => {
    const { subscribeToPublishProgress } = await import("./publish-progress");
    const handler = vi.fn();
    listenMock.mockImplementationOnce((_name: string, callback: (event: { payload: unknown }) => void) => {
      callback({ payload: { publication_id: "pub-1", bytes_uploaded: 10 } });
      return Promise.resolve(vi.fn());
    });

    await subscribeToPublishProgress(handler);

    expect(handler).toHaveBeenCalledWith({ publication_id: "pub-1", bytes_uploaded: 10 });
  });

  it("returns the real unlisten function so callers can clean up", async () => {
    const { subscribeToPublishProgress } = await import("./publish-progress");
    const unlisten = vi.fn();
    listenMock.mockResolvedValueOnce(unlisten);

    const result = await subscribeToPublishProgress(() => {});

    expect(result).toBe(unlisten);
  });
});
