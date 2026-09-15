import { invoke } from "./client";
import type { ISODateTime, Publication, UUID } from "@/types/domain";
import type { BulkScheduleResult, CalendarPublication } from "@/types/scheduling";

export const schedulerApi = {
  scheduleAt: (publicationId: UUID, at: ISODateTime) =>
    invoke<Publication>("schedule_publication", { publicationId, at }),
  unschedule: (publicationId: UUID) => invoke<Publication>("unschedule_publication", { publicationId }),
  rescheduleToDate: (publicationId: UUID, newDate: string) =>
    invoke<Publication>("reschedule_publication_to_date", { publicationId, newDate }),
  autoSchedule: (publicationId: UUID) => invoke<Publication>("auto_schedule_publication", { publicationId }),
  autoScheduleChannel: (channelId: UUID, horizonDays?: number) =>
    invoke<BulkScheduleResult>("auto_schedule_channel", { channelId, horizonDays: horizonDays ?? null }),
  fillScheduleGaps: (channelId: UUID) => invoke<BulkScheduleResult>("fill_schedule_gaps", { channelId }),
  rebuildChannelSchedule: (channelId: UUID, horizonDays?: number) =>
    invoke<BulkScheduleResult>("rebuild_channel_schedule", { channelId, horizonDays: horizonDays ?? null }),
  calendarRange: (workspaceId: UUID, channelId: UUID | null, start: string, end: string) =>
    invoke<CalendarPublication[]>("get_calendar_range", { workspaceId, channelId, start, end }),
  listDue: (workspaceId: UUID) => invoke<Publication[]>("list_due_publications", { workspaceId }),
};
