import { QueryClient } from "@tanstack/react-query";

/**
 * Backend-derived state (Content Library, Sources) goes through TanStack
 * Query rather than being copied into a Zustand store (section 71) —
 * Zustand is reserved for true UI/interaction state (selection, view
 * mode). IPC failures are almost never transient network blips, so a
 * single retry is enough.
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      staleTime: 15_000,
      refetchOnWindowFocus: false,
    },
  },
});
