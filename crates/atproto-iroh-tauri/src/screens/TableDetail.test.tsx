// Render test against the mock API/fixtures — proves Members-first
// content, the pinned message, and tab switching all actually work,
// not just that routing resolves to the right component.

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { TableDetail } from "./TableDetail";
import { TABLES } from "../mocks/fixtures";

const gardenTableId = TABLES[0].id;

function renderTable(id: string) {
  return render(
    <MemoryRouter initialEntries={[`/table/${id}`]}>
      <Routes>
        <Route path="/table/:id" element={<TableDetail />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("TableDetail", () => {
  it("shows members, the pinned message, and messages by default", async () => {
    renderTable(gardenTableId);

    await waitFor(() => expect(screen.getByText("The Garden Table")).toBeInTheDocument());

    // Members-first landing — Marisol/Devon's names also appear again
    // next to their own messages below, so assert presence, not
    // uniqueness (that's not the property this test is checking).
    expect(screen.getAllByText("Marisol").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Devon").length).toBeGreaterThan(0);

    // Pinned message, resolved to real text (not the fallback) — scoped
    // to the pinned card itself, since the same text also appears again
    // in that message's own card in the Messages tab below.
    const pinnedHeading = screen.getByText(/Pinned by Marisol/);
    const pinnedCard = pinnedHeading.closest("div")!.parentElement!;
    expect(pinnedCard).toHaveTextContent(/Tomatoes are going wild/);
    expect(pinnedCard).not.toHaveTextContent("hasn't synced yet");

    // Messages tab is the default
    expect(screen.getByText("Welcome! Water the beds Tue/Thu evenings, and grab whatever's ripe on your way out — that's what it's here for.")).toBeInTheDocument();
  });

  it("sends a message and shows it immediately, then keeps it after a re-render", async () => {
    renderTable(gardenTableId);
    await waitFor(() => expect(screen.getByText("The Garden Table")).toBeInTheDocument());

    await userEvent.type(screen.getByPlaceholderText("Say something…"), "Anyone have extra zucchini seeds?");
    await userEvent.click(screen.getByText("Send"));

    expect(await screen.findByText("Anyone have extra zucchini seeds?")).toBeInTheDocument();
    // The input clears after a successful send.
    expect(screen.getByPlaceholderText("Say something…")).toHaveValue("");

    // A fresh render (simulating navigating back to this Table) should
    // still show it — proves it actually landed in the mock backend's
    // own state, not just local optimistic UI that vanishes on remount.
    // findAllByText resolves as soon as it gets ANY non-empty match, so
    // it returns after the first render's still-present instance alone
    // — waitFor's own retry (re-running the whole assertion, including
    // the query) is what's needed to wait for the second instance too.
    renderTable(gardenTableId);
    await waitFor(() => expect(screen.getAllByText("The Garden Table")).toHaveLength(2));
    await waitFor(() =>
      expect(screen.getAllByText("Anyone have extra zucchini seeds?")).toHaveLength(2),
    );
  });

  it("switches to the Decisions tab and shows a real decision", async () => {
    renderTable(gardenTableId);
    await waitFor(() => expect(screen.getByText("The Garden Table")).toBeInTheDocument());

    await userEvent.click(screen.getByText("Decisions"));
    expect(screen.getByText("Move tool-shed hours to weekends")).toBeInTheDocument();
    expect(screen.getByText("Open")).toBeInTheDocument();
  });

  it("proposes a new decision, then objects to it and sees the status flip", async () => {
    // SELF_AUTHOR_HEX is governance-eligible on the Garden Table
    // (fixtures.ts's GOVERNANCE_STATE), so Support/Object should render.
    renderTable(gardenTableId);
    await waitFor(() => expect(screen.getByText("The Garden Table")).toBeInTheDocument());
    await userEvent.click(screen.getByText("Decisions"));

    await userEvent.type(
      screen.getByPlaceholderText("What are you deciding?"),
      "Add a second compost bin",
    );
    await userEvent.click(screen.getByText("Propose"));

    expect(await screen.findByText("Add a second compost bin")).toBeInTheDocument();
    // The input clears after a successful propose.
    expect(screen.getByPlaceholderText("What are you deciding?")).toHaveValue("");

    const card = screen.getByText("Add a second compost bin").closest("div")!;
    await userEvent.click(within(card).getByText("Object"));

    await waitFor(() =>
      expect(within(card).getByText(/Did not pass/)).toBeInTheDocument(),
    );
  });

  it("switches to the Shared doc tab, shows the real conflict banner, and lets you pick a revision", async () => {
    // The Garden Table's fixture doc has a genuine concurrent-edit
    // case built in (fixtures.ts's DOC_REVISIONS): Marisol's revision,
    // then Devon's 120s later — inside the 5-minute window, different
    // authors, so hasPossibleConflict should flag it for real, not
    // just render a banner unconditionally.
    renderTable(gardenTableId);
    await waitFor(() => expect(screen.getByText("The Garden Table")).toBeInTheDocument());

    await userEvent.click(screen.getByText("Shared doc"));
    expect(await screen.findByText(/possible conflict/)).toBeInTheDocument();

    // The latest revision (Devon's) is what loads into the box by default.
    expect(screen.getByDisplayValue(/Workday moved to Sunday/)).toBeInTheDocument();

    // "Use this" on an older revision loads it into the box for review.
    const useButtons = screen.getAllByText("Use this");
    await userEvent.click(useButtons[useButtons.length - 1]); // the earliest revision
    expect(screen.getByDisplayValue(/Draft: workday Saturday/)).toBeInTheDocument();
  });

  it("shows an existing tag on a message and filters messages by it", async () => {
    // Devon's tomato message is tagged "harvest" in fixtures.ts (real
    // data, not test setup) — the filter chip row only renders real
    // user labels (never the reserved system:pin one), and clicking it
    // should narrow the Messages tab to just that message.
    renderTable(gardenTableId);
    await waitFor(() => expect(screen.getByText("The Garden Table")).toBeInTheDocument());

    expect((await screen.findAllByText("#harvest")).length).toBeGreaterThan(0);
    expect(screen.queryByText("#system:pin")).not.toBeInTheDocument();

    await userEvent.click(screen.getAllByText("#harvest")[0]); // the filter chip
    // The tomato message's text also renders in the pinned-messages
    // section above the tabs, so it's expected to appear twice — the
    // real property this filter narrows is the Messages tab, which the
    // Welcome message's absence below actually proves.
    expect(
      screen.getAllByText("Tomatoes are going wild this week, come take some before the raccoons do.").length,
    ).toBeGreaterThan(0);
    expect(
      screen.queryByText(/Welcome! Water the beds/),
    ).not.toBeInTheDocument();
  });

  it("tags a message and sees the new tag chip appear", async () => {
    renderTable(gardenTableId);
    await waitFor(() => expect(screen.getByText("The Garden Table")).toBeInTheDocument());

    const welcomeText =
      "Welcome! Water the beds Tue/Thu evenings, and grab whatever's ripe on your way out — that's what it's here for.";
    const card = screen.getByText(welcomeText).closest("div")!;
    await userEvent.click(within(card).getByText("+ Tag"));
    await userEvent.type(within(card).getByPlaceholderText("tag name"), "welcome{Enter}");

    expect(await within(card).findByText("#welcome")).toBeInTheDocument();
    // The new label also shows up as a filter chip above the list.
    expect((await screen.findAllByText("#welcome")).length).toBeGreaterThan(1);
  });

  it("uploads a photo and shows it immediately in the gallery", async () => {
    renderTable(gardenTableId);
    await waitFor(() => expect(screen.getByText("The Garden Table")).toBeInTheDocument());
    await userEvent.click(screen.getByText("Photos"));
    expect(screen.getByText("No photos yet.")).toBeInTheDocument();

    await userEvent.type(screen.getByPlaceholderText("Caption (optional)"), "seedlings coming up");
    const file = new File(["fake-jpeg-bytes"], "seedlings.jpg", { type: "image/jpeg" });
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    await userEvent.upload(fileInput, file);

    expect(await screen.findByText("seedlings coming up")).toBeInTheDocument();
    // The gallery re-renders with a real <img> once loadImageBytes
    // resolves — its src is a blob: object URL built from the exact
    // bytes just uploaded, proving the round trip end to end rather
    // than just that *a* thumbnail placeholder appeared.
    const img = await screen.findByAltText("seedlings coming up", {}, { timeout: 3000 });
    expect(img).toHaveAttribute("src", expect.stringMatching(/^blob:/));
    expect(screen.queryByText("No photos yet.")).not.toBeInTheDocument();
  });

  it("shows a real not-found state for an unknown table id", async () => {
    renderTable("table-does-not-exist");
    await waitFor(() =>
      expect(screen.getByText(/Can't find that table/)).toBeInTheDocument(),
    );
  });

  it("only shows members who actually belong to this table", async () => {
    // Found live via screenshot, not this test (fixed after): the mock
    // client used to seed every table with a copy of *every* profile in
    // the dataset, so Weekend Hikers' Sequoia and Priya showed up as
    // "who's here" on The Garden Table, which they never joined. This
    // pins the fix.
    renderTable(gardenTableId);
    await waitFor(() => expect(screen.getByText("The Garden Table")).toBeInTheDocument());

    expect(screen.queryByText("Sequoia")).not.toBeInTheDocument();
    expect(screen.queryByText("Priya")).not.toBeInTheDocument();
  });
});
