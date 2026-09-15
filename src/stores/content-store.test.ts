import { beforeEach, describe, expect, it } from "vitest";

import { useContentStore } from "./content-store";

describe("useContentStore filters", () => {
  beforeEach(() => {
    useContentStore.setState({
      filters: {
        search: "",
        unassignedOnly: false,
        possibleDuplicatesOnly: false,
        includeArchived: false,
        sort: "newest_imported",
        page: 3,
      },
      selectedIds: [],
    });
  });

  it("resets to page 0 when a filter changes without an explicit page", () => {
    useContentStore.getState().setFilters({ search: "neymar" });
    expect(useContentStore.getState().filters.page).toBe(0);
    expect(useContentStore.getState().filters.search).toBe("neymar");
  });

  it("keeps the requested page when the patch explicitly sets one", () => {
    useContentStore.getState().setFilters({ page: 2 });
    expect(useContentStore.getState().filters.page).toBe(2);
  });

  it("merges rather than replaces the filters object", () => {
    useContentStore.getState().setFilters({ search: "clip" });
    useContentStore.getState().setFilters({ unassignedOnly: true });
    const { filters } = useContentStore.getState();
    expect(filters.search).toBe("clip");
    expect(filters.unassignedOnly).toBe(true);
  });
});

describe("useContentStore selection", () => {
  beforeEach(() => {
    useContentStore.setState({ selectedIds: [] });
  });

  it("toggles a video id in and out of the selection", () => {
    const { toggleSelected } = useContentStore.getState();
    toggleSelected("video-1");
    expect(useContentStore.getState().selectedIds).toEqual(["video-1"]);
    toggleSelected("video-1");
    expect(useContentStore.getState().selectedIds).toEqual([]);
  });

  it("clearSelection empties the selection regardless of size", () => {
    useContentStore.getState().selectMany(["a", "b", "c"]);
    useContentStore.getState().clearSelection();
    expect(useContentStore.getState().selectedIds).toEqual([]);
  });
});
