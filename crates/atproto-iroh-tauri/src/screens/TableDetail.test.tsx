// Render test against the mock API/fixtures — proves Members-first
// content, the pinned message, and tab switching all actually work,
// not just that routing resolves to the right component.

import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";
import { TableDetail } from "./TableDetail";
import { AUTHORS, TABLES } from "../mocks/fixtures";
import { api } from "../api";

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

  it("replies to a message and shows the reply-to indicator", async () => {
    // Weekend Hikers, not Garden — mockClient's message list is
    // module-level state that persists across tests in this file (same
    // hazard the pin test above already works around), and this test's
    // own reply text would otherwise leak a second, ambiguous match of
    // Garden's fixture text into a later test's assertions.
    const hikersTableId = TABLES[1].id;
    renderTable(hikersTableId);
    await waitFor(() => expect(screen.getByText("Weekend Hikers")).toBeInTheDocument());

    // Found via --sequence.shuffle: if the pin test above already ran
    // and pinned this same message, its text appears twice on the page
    // (the pinned-welcome banner and the Messages list), so looking it
    // up by text is ambiguous depending on run order. The "Reply"
    // button only ever exists on the real message card, never the
    // pinned banner, so it's an unambiguous anchor regardless of pin
    // state.
    const card = screen.getByText("Reply").closest("div")!;
    await userEvent.click(within(card).getByText("Reply"));

    expect(screen.getByText(/Replying to Sequoia/)).toBeInTheDocument();

    await userEvent.type(screen.getByPlaceholderText("Say something…"), "Got it, thanks!");
    await userEvent.click(screen.getByText("Send"));

    // The reply composer's own "Replying to" chip clears after a
    // successful send — checked via its cancel button rather than the
    // text alone, since the new message's own reply-to indicator
    // legitimately keeps showing "Replying to Sequoia" elsewhere on the
    // page.
    expect(screen.queryByLabelText("Cancel reply")).not.toBeInTheDocument();

    const newCard = (await screen.findByText("Got it, thanks!")).closest("div")!;
    expect(newCard).toHaveTextContent(/Replying to Sequoia.*Made it to the overlook/);
  });

  it("switches to the Decisions tab and shows a real decision", async () => {
    renderTable(gardenTableId);
    await waitFor(() => expect(screen.getByText("The Garden Table")).toBeInTheDocument());

    await userEvent.click(screen.getByText("Decisions"));
    expect(screen.getByText("Move tool-shed hours to weekends")).toBeInTheDocument();
    expect(screen.getByText("Open")).toBeInTheDocument();
  });

  afterEach(async () => {
    // mockClient's mute list is module-level state, not reset between
    // tests — undo this test's own mute so it can't leak into a later
    // one in this file.
    for (const hex of await api.listMuted()) await api.unmuteAuthor(hex);
  });

  it("hides a muted author's messages from the Messages tab", async () => {
    await api.muteAuthor(AUTHORS.devon);
    renderTable(gardenTableId);
    await waitFor(() => expect(screen.getByText("The Garden Table")).toBeInTheDocument());

    // Devon's tomato message should be gone entirely — both from the
    // Messages tab and from the Pinned-messages section above the tabs
    // (Marisol pinned it; a code-review pass found the pinned section
    // used to bypass the mute filter, showing a muted author's message
    // in full even though the same content was correctly hidden below).
    expect(
      screen.queryAllByText("Tomatoes are going wild this week, come take some before the raccoons do."),
    ).toHaveLength(0);
    // Marisol's welcome message, from a non-muted author, still shows.
    expect(screen.getByText(/Welcome! Water the beds/)).toBeInTheDocument();
  });

  it("also hides a pin when the pinner (not the message's author) is muted", async () => {
    // Marisol pinned Devon's tomato message — muting Marisol should
    // hide the pin even though Devon's own message stays visible in
    // the Messages tab.
    await api.muteAuthor(AUTHORS.marisol);
    renderTable(gardenTableId);
    await waitFor(() => expect(screen.getByText("The Garden Table")).toBeInTheDocument());

    expect(screen.queryByText(/Pinned by Marisol/)).not.toBeInTheDocument();
    expect(
      screen.getByText("Tomatoes are going wild this week, come take some before the raccoons do."),
    ).toBeInTheDocument();
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

  // Weekend Hikers, not Garden, for all three governance-form tests
  // below — Garden is the one Table with fixture proposals other tests
  // assert exact counts/text against (e.g. "shows a real decision"'s
  // getByText("Open")), and mockClient's proposals list is module-level
  // state that persists across tests in this file; proposing on Garden
  // would leak an extra "Open" pill into those tests depending on run
  // order (confirmed by --sequence.shuffle, same hazard this file's
  // other tests already work around for messages/tags/pins).

  it("proposes removing a co-signer and shows the class-specific label", async () => {
    const hikersTableId = TABLES[1].id;
    renderTable(hikersTableId);
    await waitFor(() => expect(screen.getByText("Weekend Hikers")).toBeInTheDocument());
    await userEvent.click(screen.getByText("Decisions"));

    await userEvent.selectOptions(screen.getByDisplayValue("Poll"), "Remove a co-signer");
    // Every Hikers member is already governance-eligible (fixtures.ts's
    // GOVERNANCE_STATE), so the admit dropdown would show "No eligible
    // members" — remove is the one that actually has candidates there.
    await userEvent.selectOptions(screen.getByDisplayValue("Choose a member…"), "Sequoia");
    await userEvent.click(screen.getByText("Propose"));

    expect(await screen.findByText("Remove Sequoia as co-signer")).toBeInTheDocument();
    const card = screen.getByText("Remove Sequoia as co-signer").closest("div")!;
    expect(card).toHaveTextContent("Remove co-signer");
    expect(card).toHaveTextContent("Sequoia");
  });

  it("shows no eligible members to admit when everyone already is", async () => {
    const hikersTableId = TABLES[1].id;
    renderTable(hikersTableId);
    await waitFor(() => expect(screen.getByText("Weekend Hikers")).toBeInTheDocument());
    await userEvent.click(screen.getByText("Decisions"));

    await userEvent.selectOptions(screen.getByDisplayValue("Poll"), "Admit a co-signer");
    expect(screen.getByDisplayValue("No eligible members")).toBeInTheDocument();
  });

  it("clears a stale member selection when switching decision type", async () => {
    // Code-review finding: picking a member under "Remove a co-signer"
    // then switching to "Admit a co-signer" used to leave the old
    // selection in state even though it's no longer a visible option
    // (Hikers' every member is already eligible, so the admit dropdown
    // has none) — Propose stayed enabled and would have silently
    // submitted the stale hex under the new type.
    const hikersTableId = TABLES[1].id;
    renderTable(hikersTableId);
    await waitFor(() => expect(screen.getByText("Weekend Hikers")).toBeInTheDocument());
    await userEvent.click(screen.getByText("Decisions"));

    await userEvent.selectOptions(screen.getByDisplayValue("Poll"), "Remove a co-signer");
    await userEvent.selectOptions(screen.getByDisplayValue("Choose a member…"), "Sequoia");
    await userEvent.selectOptions(screen.getByDisplayValue("Remove a co-signer"), "Admit a co-signer");

    expect(screen.getByDisplayValue("No eligible members")).toBeInTheDocument();
    expect(screen.getByText("Propose")).toBeDisabled();
  });

  it("proposes a policy change and shows the class-specific label", async () => {
    const hikersTableId = TABLES[1].id;
    renderTable(hikersTableId);
    await waitFor(() => expect(screen.getByText("Weekend Hikers")).toBeInTheDocument());
    await userEvent.click(screen.getByText("Decisions"));

    await userEvent.selectOptions(screen.getByDisplayValue("Poll"), "Change policy");
    await userEvent.click(screen.getByText("Propose"));

    expect(await screen.findByText("Change policy: admitting co-signers")).toBeInTheDocument();
    const card = screen.getByText("Change policy: admitting co-signers").closest("div")!;
    expect(card).toHaveTextContent("Change policy");
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

  it("tags an existing photo and sees the tag chip appear", async () => {
    // Weekend Hikers, not Garden — the fixture photo (Sequoia's "The
    // overlook, 7:12am") lives there, and tagging it doesn't collide
    // with the upload test above, which is scoped to Garden.
    const hikersTableId = TABLES[1].id;
    renderTable(hikersTableId);
    await waitFor(() => expect(screen.getByText("Weekend Hikers")).toBeInTheDocument());
    await userEvent.click(screen.getByText("Photos"));

    const card = screen.getByText("The overlook, 7:12am").closest("div")!;
    await userEvent.click(within(card).getByText("+ Tag"));
    await userEvent.type(within(card).getByPlaceholderText("tag name"), "scenic{Enter}");

    expect(await within(card).findByText("#scenic")).toBeInTheDocument();
  });

  it("tags a shared-doc revision and sees the tag chip appear", async () => {
    // New Parents Crew, not Garden — a single, non-conflicting revision
    // (fixtures.ts's DOC_REVISIONS), so there's exactly one "+ Tag"
    // button to find, no conflict banner or multi-revision noise.
    const parentsTableId = TABLES[2].id;
    renderTable(parentsTableId);
    await waitFor(() => expect(screen.getByText("New Parents Crew")).toBeInTheDocument());
    await userEvent.click(screen.getByText("Shared doc"));

    // Two matches for this text: the textarea (pre-filled with the
    // latest revision) and the History row's own <p> — the tag list
    // lives on the latter.
    const matches = screen.getAllByText(/Week of the 28th/);
    const card = matches[matches.length - 1].closest("div")!;
    await userEvent.click(within(card).getByText("+ Tag"));
    await userEvent.type(within(card).getByPlaceholderText("tag name"), "schedule{Enter}");

    expect(await within(card).findByText("#schedule")).toBeInTheDocument();
  });

  it("shows the no-pin encouragement, then pins a message and sees it appear", async () => {
    // Weekend Hikers has one message and zero pins in the fixture data —
    // the one Table in this dataset that actually exercises the
    // DESIGN_BRIEF.md §5 "no-welcome-pin shouldn't feel broken" state,
    // rather than always landing on Garden's already-pinned case.
    const hikersTableId = TABLES[1].id;
    renderTable(hikersTableId);
    await waitFor(() => expect(screen.getByText("Weekend Hikers")).toBeInTheDocument());

    expect(screen.getByText(/Nothing pinned yet/)).toBeInTheDocument();

    const messageText = "Made it to the overlook before the fog rolled in. Worth the 6am start.";
    const card = screen.getByText(messageText).closest("div")!;
    await userEvent.click(within(card).getByRole("button", { name: "Pin this message" }));

    // "Pinned by" names the pinner (Theo — SELF_AUTHOR_HEX, whoever
    // clicked the button), not the message's own author (Sequoia).
    expect(await screen.findByText(/Pinned by Theo/)).toBeInTheDocument();
    expect(screen.queryByText(/Nothing pinned yet/)).not.toBeInTheDocument();
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

describe("TableDetail — identity, naming, and refresh", () => {
  it("pins a message sent moments ago and resolves it to real text", async () => {
    // The device bug: the optimistic message's subject was built from
    // the node DID, the real one from the iroh-docs author key, so a
    // fresh pin pointed at a subject nothing matched and the strip read
    // "hasn't synced yet". The mock now keeps the two keys distinct, so
    // mixing them up fails here instead of only on a phone.
    const id = await api.createNamespaceWithProfile("Pat", "individual", null, "Pin Check");
    renderTable(id);
    await waitFor(() => expect(screen.getByText("Pin Check")).toBeInTheDocument());

    await userEvent.type(screen.getByPlaceholderText("Say something…"), "Welcome, read me first");
    await userEvent.click(screen.getByText("Send"));
    await screen.findByText("Welcome, read me first");

    await userEvent.click(screen.getByLabelText("Pin this message"));

    await screen.findByText(/Pinned by Pat/);

    // Remount so everything comes from the backend — the optimistic card
    // and its pin share whatever subject the client computed, so the
    // mismatch only surfaces once the real message replaces it.
    cleanup();
    renderTable(id);
    const pinnedHeading = await screen.findByText(/Pinned by Pat/);
    const pinnedCard = pinnedHeading.closest("div")!.parentElement!;
    await waitFor(() => expect(pinnedCard).toHaveTextContent("Welcome, read me first"));
    expect(pinnedCard).not.toHaveTextContent("hasn't synced yet");
  });

  it("lets a founder name a table that was created without one", async () => {
    const id = await api.createNamespaceWithProfile("Pat", "individual", null);
    renderTable(id);

    await userEvent.click(await screen.findByText("Name this table"));
    await userEvent.type(screen.getByLabelText("Table name"), "Old Friends");
    await userEvent.click(screen.getByText("Save"));

    expect(await screen.findByText("Old Friends")).toBeInTheDocument();
    expect(screen.getByText("Rename")).toBeInTheDocument();
    expect((await api.listTables()).find((t) => t.id === id)?.name).toBe("Old Friends");
  });

  it("offers no rename to someone who didn't found the table", async () => {
    renderTable(gardenTableId);
    await waitFor(() => expect(screen.getByText("The Garden Table")).toBeInTheDocument());
    expect(screen.queryByText("Rename")).not.toBeInTheDocument();
    expect(screen.queryByText("Name this table")).not.toBeInTheDocument();
  });

  it("doesn't overwrite unsaved typing when a new revision arrives", async () => {
    const id = await api.createNamespaceWithProfile("Pat", "individual", null, "Doc Check");
    await api.docSave(id, "notes", "first version");
    renderTable(id);
    await waitFor(() => expect(screen.getByText("Doc Check")).toBeInTheDocument());
    await userEvent.click(screen.getByText("Shared doc"));

    const box = await screen.findByDisplayValue("first version");
    await userEvent.clear(box);
    await userEvent.type(box, "my half-written edit");

    // Someone else's save lands, and the app refreshes (a foreground
    // tick runs the same refresh the interval does).
    await api.docSave(id, "notes", "their newer version");
    document.dispatchEvent(new Event("visibilitychange"));

    // The history shows theirs; the box keeps mine.
    expect(await screen.findByText("their newer version")).toBeInTheDocument();
    expect(box).toHaveValue("my half-written edit");
  });
});
