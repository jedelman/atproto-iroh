// Render smoke test against the mock API (auto-selected in jsdom, no
// window.__TAURI__) backed by the canonical fixtures — proves the Feed
// screen actually renders real content from the dataset, not just that
// it compiles.

import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { Feed } from "./Feed";

// Feed links each Table's avatar to /table/:id (react-router's <Link>),
// which throws outside a Router context — MemoryRouter is the
// lightweight one for tests, no real navigation needed.
function renderFeed() {
  return render(
    <MemoryRouter>
      <Feed />
    </MemoryRouter>,
  );
}

describe("Feed", () => {
  it("renders every table, the pinned excerpt, and a decision's status", async () => {
    renderFeed();

    // Each table's name renders twice by design (the "your people" strip
    // and its filter chip) — assert presence via getAllByText, not
    // uniqueness, which isn't the property being tested here.
    await waitFor(() => expect(screen.getAllByText("The Garden Table").length).toBeGreaterThan(0));

    expect(screen.getAllByText("Weekend Hikers").length).toBeGreaterThan(0);
    expect(screen.getAllByText("New Parents Crew").length).toBeGreaterThan(0);

    // The featured pin is Marisol's *second* pinned message (the tomato
    // one), matching pins.test.ts's "the later pin wins" assertion.
    // Scoped to the pinned card itself, not just "this text exists
    // somewhere on the page" — the excerpt text also happens to appear
    // in the message's own feed card, so a looser assertion here would
    // pass even if pinExcerpt() silently fell back to "(pinned)" (a
    // real bug this exact test caught once already, via a screenshot,
    // not the test itself — tightened afterward so it would catch it
    // on its own next time).
    const pinnedHeading = screen.getByText(/Pinned in The Garden Table/);
    const pinnedCard = pinnedHeading.closest("div")!.parentElement!;
    expect(pinnedCard).toHaveTextContent(/Tomatoes are going wild/);
    expect(pinnedCard).not.toHaveTextContent("(pinned)");

    expect(screen.getByText("Move tool-shed hours to weekends")).toBeInTheDocument();
    expect(screen.getByText("Open")).toBeInTheDocument();
  });

  it("filters to one table when its chip is clicked", async () => {
    const { default: userEvent } = await import("@testing-library/user-event");
    renderFeed();
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
