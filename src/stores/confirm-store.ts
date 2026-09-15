import { create } from "zustand";

export interface ConfirmOptions {
  title: string;
  description: string;
  confirmLabel?: string;
  cancelLabel?: string;
  /** Renders the confirm button as a destructive action (red). */
  destructive?: boolean;
}

interface PendingConfirm extends ConfirmOptions {
  id: string;
  resolve: (confirmed: boolean) => void;
}

interface ConfirmState {
  pending: PendingConfirm | null;
  settle: (confirmed: boolean) => void;
}

export const useConfirmStore = create<ConfirmState>((set, get) => ({
  pending: null,
  settle: (confirmed) => {
    get().pending?.resolve(confirmed);
    set({ pending: null });
  },
}));

let counter = 0;

/**
 * Themed, Promise-based replacement for `window.confirm` (section 98 — a
 * Phase 2 known limitation this phase is required to fix). Resolves to
 * `true`/`false` exactly like the native API would, but renders through
 * XP FLOW's own Radix dialog (`<ConfirmDialogHost/>`, mounted once at the
 * app root) instead of native OS chrome. Usable from anywhere — no need
 * to thread a confirm function through props, mirroring `toast()`.
 */
export function confirmAction(options: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    useConfirmStore.setState({ pending: { ...options, id: `confirm-${Date.now()}-${counter++}`, resolve } });
  });
}
