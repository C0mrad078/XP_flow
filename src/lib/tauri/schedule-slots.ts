import { invoke } from "./client";
import type { Platform, UUID } from "@/types/domain";
import type { ScheduleException, ScheduleSlot } from "@/types/scheduling";

export const scheduleSlotsApi = {
  list: (channelId: UUID) => invoke<ScheduleSlot[]>("list_schedule_slots", { channelId }),
  create: (channelId: UUID, platform: Platform | null, dayOfWeek: number, timeOfDay: string) =>
    invoke<ScheduleSlot>("create_schedule_slot", { channelId, platform, dayOfWeek, timeOfDay }),
  setActive: (id: UUID, isActive: boolean) =>
    invoke<ScheduleSlot>("set_schedule_slot_active", { id, isActive }),
  remove: (id: UUID) => invoke<void>("delete_schedule_slot", { id }),
  copyDay: (channelId: UUID, fromDay: number, toDays: number[]) =>
    invoke<ScheduleSlot[]>("copy_schedule_day", { channelId, fromDay, toDays }),
  addException: (channelId: UUID, date: string, reason: string | null) =>
    invoke<ScheduleException>("add_schedule_exception", { channelId, date, reason }),
  removeException: (id: UUID) => invoke<void>("remove_schedule_exception", { id }),
  listExceptions: (channelId: UUID, from: string, to: string) =>
    invoke<ScheduleException[]>("list_schedule_exceptions", { channelId, from, to }),
};
