// The minimum real loop from USER_FLOW.md §2, against the mock: start a
// Table from nothing, get an invite, redeem it, land in the Table. Every
// earlier test began already inside a fixture Table, which is how the
// missing create/invite screens went unnoticed until a real install.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { CreateTable } from "./CreateTable";
import { Invite } from "./Invite";
import { TableDetail } from "./TableDetail";
import { api } from "../api";

function renderFlow() {
  return render(
    <MemoryRouter initialEntries={["/new"]}>
      <Routes>
        <Route path="/new" element={<CreateTable />} />
        <Route path="/table/:id/invite" element={<Invite />} />
        <Route path="/table/:id" element={<TableDetail />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("Start a table → invite → join", () => {
  it("won't start a table without a name", () => {
    renderFlow();
    expect(screen.getByText("Start it")).toBeDisabled();
  });

  it("starts a named table, shows a redeemable invite, and lands in the table as its founder", async () => {
    localStorage.setItem("atproto-iroh:profile-draft", JSON.stringify({ name: "Ada", avatar: "sage" }));
    renderFlow();

    await userEvent.type(screen.getByLabelText("What's it called?"), "Sunday dinner crew");
    await userEvent.click(screen.getByText("Start it"));

    expect(await screen.findByText("Now invite your people")).toBeInTheDocument();
    const codeBox = (await screen.findByLabelText("Invite code")) as HTMLTextAreaElement;
    const ticket = codeBox.value;
    expect(ticket).toMatch(/^mock-ticket-write-/);
    expect(screen.getByAltText("Invite code as a QR code")).toHaveAttribute("src", expect.stringMatching(/^data:image\/svg\+xml/));

    // The ticket redeems to the same Table, and the Table is listed by
    // its real name, not a truncated id.
    const joinedId = await api.joinNamespace(ticket);
    const tables = await api.listTables();
    expect(tables.find((t) => t.id === joinedId)?.name).toBe("Sunday dinner crew");

    await userEvent.click(screen.getByText("Go to the table"));
    await waitFor(() => expect(screen.getByText("Sunday dinner crew")).toBeInTheDocument());
    // The founder is a real member with a say — Founding, not just a profile.
    expect(screen.getByText("Ada")).toBeInTheDocument();
    expect(screen.getByText("can weigh in on decisions")).toBeInTheDocument();
    expect(screen.getByText("Invite")).toHaveAttribute("href", `/table/${joinedId}/invite`);
  });
});
