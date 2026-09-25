// Render test against the mock API/fixtures — proves the form prefills
// from the real self profile and that saving actually calls through to
// the Client, not just that the route resolves.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { ProfileEdit } from "./ProfileEdit";
import { TABLES } from "../mocks/fixtures";

const gardenTableId = TABLES[0].id;

function renderProfileEdit(id: string) {
  return render(
    <MemoryRouter initialEntries={[`/table/${id}/profile`]}>
      <Routes>
        <Route path="/table/:id/profile" element={<ProfileEdit />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("ProfileEdit", () => {
  it("prefills from the real self profile in this Table", async () => {
    renderProfileEdit(gardenTableId);
    await waitFor(() => expect(screen.getByDisplayValue("Theo")).toBeInTheDocument());
  });

  it("edits the name and saves, showing confirmation", async () => {
    // Weekend Hikers, not Garden: the save persists in mockClient's
    // module-level state, so editing Garden renamed "Theo" out from under
    // the prefill test above whenever --sequence.shuffle ran this first.
    renderProfileEdit(TABLES[1].id);
    await waitFor(() => expect(screen.getByDisplayValue("Theo")).toBeInTheDocument());

    const nameInput = screen.getByDisplayValue("Theo");
    await userEvent.clear(nameInput);
    await userEvent.type(nameInput, "Theodora");
    await userEvent.click(screen.getByText("Save"));

    expect(await screen.findByText("Saved")).toBeInTheDocument();
  });
});
