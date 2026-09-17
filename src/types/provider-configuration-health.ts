/**
 * Frontend mirror of `application::provider_configuration_health_service`
 * (Rust). This is the single backend-authoritative signal every Connect
 * Account surface reads to decide what its Connect action does — it is
 * never used to decide whether a platform is *shown*. All three
 * platforms are always rendered regardless of what this says.
 */
import type { Platform } from "./domain";

export type ProviderConfigurationStatus = "ready" | "configuration_required" | "broker_unavailable" | "error";

export interface ProviderConfigurationHealth {
  platform: Platform;
  available: boolean;
  status: ProviderConfigurationStatus;
  missing_configuration: string[];
  user_message: string | null;
}
