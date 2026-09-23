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

  it("joins a table with a valid pasted ticket and lands on it", async () => {
    renderJoin();
    await userEvent.type(screen.getByPlaceholderText("Paste the ticket text here"), "mock-ticket-bookclub");
    await userEvent.click(screen.getByText("Join with this ticket"));

    await waitFor(() => expect(screen.getByText("Thursday Book Club")).toBeInTheDocument());
  });

  it("carries the Onboarding name/sticker into the newly joined table's profile", async () => {
    // Found in review: joining used to call only joinNamespace, never
    // updateProfile, so the Onboarding pick never reached a real Table
    // and a new member showed up as "Someone" until they separately
    // visited Profile edit. useProfileDraft is localStorage-backed —
    // seeding it here is what a completed Onboarding screen would have
    // already done.
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
