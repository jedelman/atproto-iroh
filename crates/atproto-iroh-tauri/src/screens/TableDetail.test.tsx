// Render test against the mock API/fixtures — proves Members-first
// content, the pinned message, and tab switching all actually work,
// not just that routing resolves to the right component.

import { render, screen, waitFor } from "@testing-library/react";
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
