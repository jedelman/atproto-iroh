// Render smoke test against the mock API (auto-selected in jsdom, no
// window.__TAURI__) backed by the canonical fixtures — proves the Feed
// screen actually renders real content from the dataset, not just that
// it compiles.

import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Feed } from "./Feed";

describe("Feed", () => {
  it("renders every table, the pinned excerpt, and a decision's status", async () => {
    render(<Feed />);

    // Each table's name renders twice by design (the "your people" strip
    // and its filter chip) — assert presence via getAllByText, not
    // uniqueness, which isn't the property being tested here.
    await waitFor(() => expect(screen.getAllByText("The Garden Table").length).toBeGreaterThan(0));

    expect(screen.getAllByText("Weekend Hikers").length).toBeGreaterThan(0);
    expect(screen.getAllByText("New Parents Crew").length).toBeGreaterThan(0);

    // The featured pin is Marisol's *second* pinned message (the tomato
    // one), matching pins.test.ts's "the later pin wins" assertion.
    expect(screen.getByText(/Pinned in The Garden Table/)).toBeInTheDocument();
    expect(screen.getByText(/Tomatoes are going wild/)).toBeInTheDocument();

    expect(screen.getByText("Move tool-shed hours to weekends")).toBeInTheDocument();
    expect(screen.getByText("Open")).toBeInTheDocument();
  });

  it("filters to one table when its chip is clicked", async () => {
    const { default: userEvent } = await import("@testing-library/user-event");
    render(<Feed />);
    await waitFor(() => expect(screen.getAllByText("The Garden Table").length).toBeGreaterThan(0));

    const chips = screen.getAllByText("Weekend Hikers");
    // The strip avatar's label and the filter chip both render this
    // text; the chip is the <button> element among them.
    const chipButton = chips.find((el) => el.closest("button"));
    expect(chipButton).toBeTruthy();
    await userEvent.click(chipButton!.closest("button")!);

    expect(screen.queryByText("Move tool-shed hours to weekends")).not.toBeInTheDocument();
    expect(screen.getByText(/Made it to the overlook/)).toBeInTheDocument();
  });
});
