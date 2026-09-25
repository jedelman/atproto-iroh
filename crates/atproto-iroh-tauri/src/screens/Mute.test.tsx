import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { Mute } from "./Mute";

function renderMute() {
  return render(
    <MemoryRouter>
      <Mute />
    </MemoryRouter>,
  );
}

describe("Mute", () => {
  it("shows an empty state, then mutes and unmutes an author", async () => {
    renderMute();
    await waitFor(() => expect(screen.getByText("Nobody muted.")).toBeInTheDocument());

    await userEvent.type(screen.getByPlaceholderText("Author id (hex)"), "deadbeef{Enter}");
    expect(await screen.findByText("deadbeef")).toBeInTheDocument();
    expect(screen.queryByText("Nobody muted.")).not.toBeInTheDocument();

    await userEvent.click(screen.getByText("Unmute"));
    expect(await screen.findByText("Nobody muted.")).toBeInTheDocument();
  });
});
