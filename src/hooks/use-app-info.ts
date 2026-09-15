import { useEffect, useState } from "react";

import { systemApi } from "@/lib/tauri";
import type { AppInfo } from "@/types/domain";

export function useAppInfo() {
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    systemApi
      .getAppInfo()
      .then(setAppInfo)
      .catch(() => setError(true));
  }, []);

  return { appInfo, error };
}
