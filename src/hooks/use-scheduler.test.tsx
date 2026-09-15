import type { ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { CalendarPublication } from "@/types/scheduling";

const rescheduleToDateMock = vi.hoisted(() => vi.fn());

vi.mock("@/lib/tauri", () => ({
  schedulerApi: { rescheduleToDate: rescheduleToDateMock },
}));

// Imported after the mock so the hook module picks up the mocked API.
const { useReschedulePublicationToDate } = await import("./use-scheduler");

function calendarItem(overrides: Partial<CalendarPublication["publication"]> = {}): CalendarPublication {
  return {
    local_date: "2024-06-10",
    local_time: "09:00",
    publication: {
      id: "pub-1",
      workspace_id: "ws-1",
      video_id: "video-1",
      channel_id: "channel-1",
      platform_account_id: null,
      platform: "youtube",
      status: "scheduled",
      title: "Clip",
      description: null,
      hashtags: [],
      priority: "normal",
      locked: false,
      scheduled_at: "2024-06-10T09:00:00Z",
      published_at: null,
      remote_id: null,
      retry_count: 0,
      last_error: null,
      created_at: "2024-06-01T00:00:00Z",
      updated_at: "2024-06-01T00:00:00Z",
      ...overrides,
    },
  };
}

describe("useReschedulePublicationToDate", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    rescheduleToDateMock.mockReset();
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
  });

  function wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  }

  it("optimistically moves the chip to the new date before the request resolves", async () => {
    const original = [calendarItem()];
    queryClient.setQueryData(["calendar", "ws-1", null, "2024-06-01", "2024-06-30"], original);

    let resolveRequest!: (value: unknown) => void;
    rescheduleToDateMock.mockReturnValue(new Promise((resolve) => (resolveRequest = resolve)));

    const { result } = renderHook(() => useReschedulePublicationToDate(), { wrapper });

    act(() => {
      result.current.mutate({ publicationId: "pub-1", newDate: "2024-06-15" });
    });

    await waitFor(() => {
      const cached = queryClient.getQueryData<CalendarPublication[]>([
        "calendar",
        "ws-1",
        null,
        "2024-06-01",
        "2024-06-30",
      ]);
      expect(cached?.[0]?.local_date).toBe("2024-06-15");
    });

    resolveRequest(original[0]!.publication);
  });

  it("rolls back the optimistic move when the backend rejects it", async () => {
    const key = ["calendar", "ws-1", null, "2024-06-01", "2024-06-30"];
    const original = [calendarItem()];
    queryClient.setQueryData(key, original);
    rescheduleToDateMock.mockRejectedValue({
      code: "VALIDATION",
      user_message: "This publication is locked.",
      developer_message: "publication locked",
    });

    const { result } = renderHook(() => useReschedulePublicationToDate(), { wrapper });

    act(() => {
      result.current.mutate({ publicationId: "pub-1", newDate: "2024-06-15" });
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    const cached = queryClient.getQueryData<CalendarPublication[]>(key);
    expect(cached?.[0]?.local_date).toBe("2024-06-10");
  });
});
