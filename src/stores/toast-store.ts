import { create } from "zustand";

export type ToastVariant = "info" | "success" | "warning" | "error";

export interface ToastItem {
  id: string;
  title: string;
  description?: string;
  variant: ToastVariant;
}

interface ToastState {
  toasts: ToastItem[];
  dismiss: (id: string) => void;
}

export const useToastStore = create<ToastState>((set) => ({
  toasts: [],
  dismiss: (id) => set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) })),
}));

let counter = 0;

/** Fire-and-forget toast notification. Auto-dismisses after 5s. */
export function toast(input: Omit<ToastItem, "id">): void {
  const id = `toast-${Date.now()}-${counter++}`;
  useToastStore.setState((state) => ({ toasts: [...state.toasts, { ...input, id }] }));
  setTimeout(() => useToastStore.getState().dismiss(id), 5000);
}
