import { invoke as tauriInvoke } from "@tauri-apps/api/core";

import { type AppError, isAppError } from "@/types/domain";

/**
 * The single choke point every Tauri `invoke` call in the app goes
 * through. Nothing outside `src/lib/tauri` may import `@tauri-apps/api`
 * directly — see `docs/development-guidelines.md`. This is what keeps the
 * React -> Rust boundary typed and lets a future remote backend swap in
 * behind the same function signatures.
 */
export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await tauriInvoke<T>(command, args);
  } catch (error) {
    throw normalizeError(error);
  }
}

function normalizeError(error: unknown): AppError {
  if (isAppError(error)) {
    return error;
  }

  return {
    code: "INTERNAL",
    user_message: "Something went wrong. Please try again.",
    developer_message: error instanceof Error ? error.message : String(error),
  };
}
