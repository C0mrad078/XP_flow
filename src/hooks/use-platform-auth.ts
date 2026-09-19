import { useCallback, useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { platformAuthApi } from "@/lib/tauri";
import type { Platform, UUID } from "@/types/domain";
import type { AuthFlowState } from "@/types/platform-auth";

/**
 * The single backend-authoritative source every Connect Account surface
 * reads (never independently re-derived per component — see
 * `ProviderConfigurationHealth`'s doc comment). Re-fetched on window
 * focus and on manual retry since broker reachability can change while
 * the dialog is open; not polled continuously, since this is a
 * connect-readiness check, not a live status stream.
 */
export function useProviderConfigurationHealth() {
  return useQuery({
    queryKey: ["provider-configuration-health"],
    queryFn: () => platformAuthApi.getConfigurationHealth(),
    staleTime: 30_000,
  });
}

/** Section 79: bounded polling only while an authorization dialog is
 * actually open — never a global "poll every account forever" loop. */
const POLL_INTERVAL_MS = 800;

function useInvalidateAfterAuth() {
  const queryClient = useQueryClient();
  return () => {
    queryClient.invalidateQueries({ queryKey: ["platform-accounts"] });
    queryClient.invalidateQueries({ queryKey: ["channel-overview"] });
  };
}

/**
 * Drives one connect/reconnect attempt end to end (section 44/79):
 * `start()` begins the flow and polls `poll_platform_connect_status`
 * until it reaches a terminal state, exposing the live `AuthFlowState`
 * for the "Connecting…" dialog. `cancel()` stops polling and tells the
 * backend to tear down whatever it opened.
 */
export function useConnectFlow() {
  const [state, setState] = useState<AuthFlowState | null>(null);
  const [platform, setPlatform] = useState<Platform | null>(null);
  const [sessionId, setSessionId] = useState<UUID | null>(null);
  const pollTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const invalidate = useInvalidateAfterAuth();

  const stopPolling = useCallback(() => {
    if (pollTimer.current) {
      clearTimeout(pollTimer.current);
      pollTimer.current = null;
    }
  }, []);

  // A ref-held indirection so the recursive tick can call "the current
  // poll" without referencing the `poll` binding from inside its own
  // initializer (self-referential useCallback bodies trip the
  // react-hooks exhaustive-deps/immutability check).
  const pollRef = useRef<(id: UUID) => void>(() => {});

  const poll = useCallback(
    (id: UUID) => {
      pollTimer.current = setTimeout(async () => {
        try {
          const next = await platformAuthApi.pollStatus(id);
          if (next) setState(next);
          if (next && (next.state === "connected" || next.state === "failed")) {
            stopPolling();
            invalidate();
            return;
          }
        } catch {
          // A transient IPC hiccup shouldn't kill the dialog — just retry.
        }
        pollRef.current(id);
      }, POLL_INTERVAL_MS);
    },
    [invalidate, stopPolling],
  );
  useEffect(() => {
    pollRef.current = poll;
  }, [poll]);

  const start = useCallback(
    async (forPlatform: Platform, begin: () => Promise<UUID>) => {
      stopPolling();
      setPlatform(forPlatform);
      setState({ state: "opening_browser" });
      try {
        const id = await begin();
        setSessionId(id);
        poll(id);
      } catch (error) {
        const message =
          typeof error === "object" && error !== null && "user_message" in error
            ? String((error as { user_message: unknown }).user_message)
            : "XP FLOW could not start the YouTube connection. Please try again.";
        setState({ state: "failed", code: "AUTH_START_FAILED", message });
      }
    },
    [poll, stopPolling],
  );

  const connect = useCallback(
    (workspaceId: UUID, channelId: UUID, platform: Platform) =>
      start(platform, () => platformAuthApi.beginConnect(workspaceId, channelId, platform)),
    [start],
  );

  const reconnect = useCallback(
    (accountId: UUID, platform: Platform, allowIdentityChange = false) =>
      start(platform, () => platformAuthApi.beginReconnect(accountId, allowIdentityChange)),
    [start],
  );

  const cancel = useCallback(async () => {
    stopPolling();
    if (sessionId) await platformAuthApi.cancel(sessionId);
    setState(null);
    setSessionId(null);
    setPlatform(null);
  }, [sessionId, stopPolling]);

  const reset = useCallback(() => {
    stopPolling();
    setState(null);
    setSessionId(null);
    setPlatform(null);
  }, [stopPolling]);

  useEffect(() => stopPolling, [stopPolling]);

  return { state, platform, connect, reconnect, cancel, reset };
}

export function useValidatePlatformAccount() {
  const invalidate = useInvalidateAfterAuth();
  return useMutation({
    mutationFn: (accountId: UUID) => platformAuthApi.validate(accountId),
    onSuccess: invalidate,
  });
}

export function useRefreshPlatformAccount() {
  const invalidate = useInvalidateAfterAuth();
  return useMutation({
    mutationFn: (accountId: UUID) => platformAuthApi.refresh(accountId),
    onSuccess: invalidate,
  });
}

export function useDisconnectPlatformAccount() {
  const invalidate = useInvalidateAfterAuth();
  return useMutation({
    mutationFn: (accountId: UUID) => platformAuthApi.disconnect(accountId),
    onSuccess: invalidate,
  });
}
