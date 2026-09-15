import { render, screen } from "@testing-library/react";
import { Inbox } from "lucide-react";
import { describe, expect, it } from "vitest";

import { EmptyState } from "./empty-state";

describe("EmptyState", () => {
  it("renders the title and description", () => {
    render(<EmptyState icon={Inbox} title="No videos yet" description="Import a video to get started." />);

    expect(screen.getByText("No videos yet")).toBeInTheDocument();
    expect(screen.getByText("Import a video to get started.")).toBeInTheDocument();
  });

  it("omits the description paragraph when none is given", () => {
    render(<EmptyState icon={Inbox} title="Queue is empty" />);

    expect(screen.getByText("Queue is empty")).toBeInTheDocument();
    expect(screen.queryByText(/import/i)).not.toBeInTheDocument();
  });
});
