// The camera path (getUserMedia) isn't testable in jsdom — no real
// camera, no real permission prompt — so this covers the paste-a-ticket
// fallback, which is real UI a person actually falls back to if the
// camera path errors, not a lesser-tested afterthought.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";
import { Join } from "./Join";
import { TableDetail } from "./TableDetail";

function renderJoin() {
  return render(
    <MemoryRouter initialEntries={["/join"]}>
      <Routes>
        <Route path="/join" element={<Join />} />
        <Route path="/table/:id" element={<TableDetail />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("Join", () => {
  afterEach(() => {
    localStorage.removeItem("atproto-iroh:profile-draft");
  });

  it("carries the Onboarding name/sticker into the newly joined table's profile", async () => {
    // Found in review: joining used to call only joinNamespace, never
    // updateProfile, so the Onboarding pick never reached a real Table
    // and a new member showed up as "Someone" until they separately
    // visited Profile edit. useProfileDraft is localStorage-backed —
    // seeding it here is what a completed Onboarding screen would have
    // already done.
    //
    // Runs before the other tests below that also join the book club
    // fixture on purpose: mockClient's profiles are module-level state
    // that persists across tests in this file, and a follow-up review
    // fix made this call correctly skip writing once a profile already
    // exists (to avoid clobbering a real member's data on a re-join) —
    // so this has to be the first join against that fixture, or it
    // would itself hit that skip and never see "Nia" appear.
    localStorage.setItem(
      "atproto-iroh:profile-draft",
      JSON.stringify({ name: "Nia", avatar: "sky" }),
    );
    renderJoin();
    await userEvent.type(screen.getByPlaceholderText("Paste the ticket text here"), "mock-ticket-bookclub");
    await userEvent.click(screen.getByText("Join with this ticket"));

    await waitFor(() => expect(screen.getByText("Thursday Book Club")).toBeInTheDocument());
    expect(await screen.findByText("Nia")).toBeInTheDocument();
  });

  it("joins a table with a valid pasted ticket and lands on it", async () => {
    renderJoin();
    await userEvent.type(screen.getByPlaceholderText("Paste the ticket text here"), "mock-ticket-bookclub");
    await userEvent.click(screen.getByText("Join with this ticket"));

    await waitFor(() => expect(screen.getByText("Thursday Book Club")).toBeInTheDocument());
  });

  it("re-joining an already-joined table doesn't clobber the existing profile", async () => {
    // Found in review: the profile-carrying fix above originally wrote
    // the draft's name/avatar unconditionally on every join, including
    // a re-join of a Table already belonged to (Join is a persistent,
    // always-reachable entry point) — silently overwriting a real
    // member's name/avatar/neighborhood/description back to onboarding
    // defaults. By this point in the file, self already has a "Nia"
    // profile in the book club table from the first test above; a
    // different draft name here should NOT replace it.
    localStorage.setItem(
      "atproto-iroh:profile-draft",
      JSON.stringify({ name: "Someone Else Entirely", avatar: "gold" }),
    );
    renderJoin();
    await userEvent.type(screen.getByPlaceholderText("Paste the ticket text here"), "mock-ticket-bookclub");
    await userEvent.click(screen.getByText("Join with this ticket"));

    await waitFor(() => expect(screen.getByText("Thursday Book Club")).toBeInTheDocument());
    expect(await screen.findByText("Nia")).toBeInTheDocument();
    expect(screen.queryByText("Someone Else Entirely")).not.toBeInTheDocument();
  });

  it("shows a real error for an invalid ticket, not a silent failure", async () => {
    renderJoin();
    await userEvent.type(screen.getByPlaceholderText("Paste the ticket text here"), "not-a-real-ticket");
    await userEvent.click(screen.getByText("Join with this ticket"));

    await waitFor(() => expect(screen.getByText(/Couldn't join/)).toBeInTheDocument());
  });

  it("disables the join button until something is pasted", () => {
    renderJoin();
    expect(screen.getByText("Join with this ticket")).toBeDisabled();
  });
});
