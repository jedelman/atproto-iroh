// No bundler, no npm dependency — `withGlobalTauri: true` in
// tauri.conf.json injects `window.__TAURI__` directly, so this file is
// plain, static JS the webview can load with no build step at all.
// Deliberate: this reference client is meant to stay easy to read and
// easy to replace, not to accumulate a frontend toolchain nobody asked
// for.
const { invoke } = window.__TAURI__.core;

const didEl = document.getElementById("did");
const namespaceEl = document.getElementById("namespace");

document.getElementById("spawn").addEventListener("click", async () => {
  didEl.textContent = "spawning…";
  try {
    const did = await invoke("spawn_node");
    didEl.textContent = did;
  } catch (err) {
    didEl.textContent = `error: ${err}`;
  }
});

document
  .getElementById("create-namespace")
  .addEventListener("submit", async (event) => {
    event.preventDefault();
    const name = document.getElementById("name").value;
    const category = document.getElementById("category").value;
    namespaceEl.textContent = "creating…";
    try {
      const namespaceId = await invoke("create_namespace_with_profile", {
        name,
        category,
      });
      namespaceEl.textContent = namespaceId;
      await refreshNamespaces();
    } catch (err) {
      namespaceEl.textContent = `error: ${err}`;
    }
  });

const namespaceListEl = document.getElementById("namespace-list");

async function refreshNamespaces() {
  namespaceListEl.innerHTML = "";
  try {
    const namespaces = await invoke("list_namespaces");
    for (const id of namespaces) {
      const li = document.createElement("li");
      li.textContent = id;
      li.className = "value";
      namespaceListEl.appendChild(li);
    }
  } catch (err) {
    const li = document.createElement("li");
    li.textContent = `error: ${err}`;
    namespaceListEl.appendChild(li);
  }
}

document
  .getElementById("refresh-namespaces")
  .addEventListener("click", refreshNamespaces);

const ticketOutEl = document.getElementById("ticket-out");
const qrOutEl = document.getElementById("qr-out");

document
  .getElementById("share-namespace")
  .addEventListener("submit", async (event) => {
    event.preventDefault();
    const namespaceId = document.getElementById("share-namespace-id").value;
    const mode = document.getElementById("share-mode").value;
    ticketOutEl.value = "creating ticket…";
    qrOutEl.innerHTML = "";
    try {
      ticketOutEl.value = await invoke("share_namespace", {
        namespaceId,
        mode,
      });
    } catch (err) {
      ticketOutEl.value = `error: ${err}`;
    }
  });

// QR is generated on demand, not automatically on every ticket — a
// deliberate extra step before turning a capability into something
// physically postable (see index.html's hint on Write tickets).
document.getElementById("show-qr").addEventListener("click", async () => {
  const ticket = ticketOutEl.value;
  if (!ticket || ticket.startsWith("error") || ticket.endsWith("…")) {
    qrOutEl.textContent = "create a ticket first";
    return;
  }
  qrOutEl.textContent = "rendering…";
  try {
    // ticket_to_qr returns raw SVG markup; safe to inject directly since
    // it's this app's own Rust code generating it, not untrusted input.
    qrOutEl.innerHTML = await invoke("ticket_to_qr", { ticket });
  } catch (err) {
    qrOutEl.textContent = `error: ${err}`;
  }
});

const joinedEl = document.getElementById("joined");

document
  .getElementById("join-namespace")
  .addEventListener("submit", async (event) => {
    event.preventDefault();
    const ticket = document.getElementById("ticket-in").value.trim();
    joinedEl.textContent = "joining… (full history sync can take a moment)";
    try {
      const namespaceId = await invoke("join_namespace", { ticket });
      joinedEl.textContent = namespaceId;
      await refreshNamespaces();
    } catch (err) {
      joinedEl.textContent = `error: ${err}`;
    }
  });

// --- Members / Profile ---------------------------------------------

const membersOutEl = document.getElementById("members-out");
const profileStatusEl = document.getElementById("profile-status");

document
  .getElementById("load-members")
  .addEventListener("submit", async (event) => {
    event.preventDefault();
    const namespaceId = document.getElementById("members-namespace-id").value;
    membersOutEl.innerHTML = "";
    try {
      const members = await invoke("list_profiles", { namespaceId });
      if (members.length === 0) {
        membersOutEl.textContent = "(no profiles yet)";
        return;
      }
      for (const m of members) {
        const li = document.createElement("li");
        const p = m.profile;
        li.textContent = `${m.author_hex.slice(0, 8)}…  ${p.name} (${p.category})${p.neighborhood ? " — " + p.neighborhood : ""}`;
        membersOutEl.appendChild(li);
      }
    } catch (err) {
      membersOutEl.textContent = `error: ${err}`;
    }
  });

document
  .getElementById("update-profile")
  .addEventListener("submit", async (event) => {
    event.preventDefault();
    const namespaceId = document.getElementById("members-namespace-id").value;
    const name = document.getElementById("profile-name").value;
    const category = document.getElementById("profile-category").value;
    const neighborhood = document.getElementById("profile-neighborhood").value || null;
    profileStatusEl.textContent = "saving…";
    try {
      await invoke("update_profile", {
        namespaceId,
        name,
        category,
        neighborhood,
        description: null,
        governanceEligible: null,
      });
      profileStatusEl.textContent = `saved at ${new Date().toLocaleTimeString()}`;
    } catch (err) {
      profileStatusEl.textContent = `error: ${err}`;
    }
  });

// --- Mute ------------------------------------------------------------

const muteStatusEl = document.getElementById("mute-status");
const mutedOutEl = document.getElementById("muted-out");

async function refreshMuted() {
  mutedOutEl.innerHTML = "";
  try {
    const muted = await invoke("list_muted");
    for (const authorHex of muted) {
      const li = document.createElement("li");
      li.textContent = authorHex;
      mutedOutEl.appendChild(li);
    }
  } catch (err) {
    mutedOutEl.textContent = `error: ${err}`;
  }
}

document.getElementById("mute-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  const authorHex = document.getElementById("mute-author-hex").value;
  muteStatusEl.textContent = "muting…";
  try {
    await invoke("mute_author", { authorHex });
    muteStatusEl.textContent = "muted";
    await refreshMuted();
  } catch (err) {
    muteStatusEl.textContent = `error: ${err}`;
  }
});

document.getElementById("unmute-btn").addEventListener("click", async () => {
  const authorHex = document.getElementById("mute-author-hex").value;
  muteStatusEl.textContent = "unmuting…";
  try {
    await invoke("unmute_author", { authorHex });
    muteStatusEl.textContent = "unmuted";
    await refreshMuted();
  } catch (err) {
    muteStatusEl.textContent = `error: ${err}`;
  }
});

refreshMuted();

// --- Shared doc --------------------------------------------------------

const docStatusEl = document.getElementById("doc-status");
const docHistoryEl = document.getElementById("doc-history");

document.getElementById("write-text").addEventListener("submit", async (event) => {
  event.preventDefault();
  const namespaceId = document.getElementById("doc-namespace-id").value;
  const docId = document.getElementById("doc-id").value;
  const text = document.getElementById("doc-text").value;
  docStatusEl.textContent = "saving…";
  try {
    // doc_save writes a new revision rather than overwriting the last
    // one — see namespace::save_document_revision's doc comment for why
    // this replaced write_text/put_text here (a confirmed, silent
    // data-loss case under concurrent offline edits).
    const rev = await invoke("doc_save", { namespaceId, docId, text });
    docStatusEl.textContent = `saved revision ${rev} at ${new Date().toLocaleTimeString()}`;
  } catch (err) {
    docStatusEl.textContent = `error: ${err}`;
  }
});

document.getElementById("doc-load").addEventListener("click", async () => {
  const namespaceId = document.getElementById("doc-namespace-id").value;
  const docId = document.getElementById("doc-id").value;
  docStatusEl.textContent = "loading…";
  try {
    const result = await invoke("doc_load", { namespaceId, docId });
    if (result === null) {
      document.getElementById("doc-text").value = "";
      docStatusEl.textContent = "(no revisions of this doc yet)";
    } else {
      const [rev, authorHex, text] = result;
      document.getElementById("doc-text").value = text;
      docStatusEl.textContent = `loaded revision ${rev} (by ${authorHex.slice(0, 8)}…)`;
    }
  } catch (err) {
    docStatusEl.textContent = `error: ${err}`;
  }
});

document.getElementById("doc-history-btn").addEventListener("click", async () => {
  const namespaceId = document.getElementById("doc-namespace-id").value;
  const docId = document.getElementById("doc-id").value;
  docHistoryEl.textContent = "";
  try {
    const revisions = await invoke("doc_history", { namespaceId, docId });
    if (revisions.length === 0) {
      docHistoryEl.textContent = "(no revisions yet)";
      return;
    }
    for (const [rev, authorHex, text] of revisions) {
      const li = document.createElement("li");
      li.textContent = `${rev} — ${authorHex.slice(0, 8)}…: ${text}`;
      docHistoryEl.appendChild(li);
    }
  } catch (err) {
    docHistoryEl.textContent = `error: ${err}`;
  }
});

// --- Messages ------------------------------------------------------

const msgStatusEl = document.getElementById("msg-status");
const msgOutEl = document.getElementById("msg-out");

async function refreshMessages() {
  const namespaceId = document.getElementById("msg-namespace-id").value;
  msgOutEl.textContent = "";
  try {
    const messages = await invoke("list_messages", { namespaceId });
    if (messages.length === 0) {
      msgOutEl.textContent = "(no messages yet)";
      return;
    }
    for (const m of messages) {
      const li = document.createElement("li");
      const replyNote = m.reply_to ? ` (reply to ${m.reply_to})` : "";
      li.textContent = `${m.author_hex.slice(0, 8)}…: ${m.text}${replyNote} `;
      const tagBtn = document.createElement("button");
      tagBtn.type = "button";
      tagBtn.textContent = "Tag";
      tagBtn.addEventListener("click", () => {
        document.getElementById("tag-namespace-id").value = namespaceId;
        document.getElementById("tag-subject").value = m.subject;
      });
      li.appendChild(tagBtn);
      msgOutEl.appendChild(li);
    }
  } catch (err) {
    msgOutEl.textContent = `error: ${err}`;
  }
}

document.getElementById("send-message").addEventListener("submit", async (event) => {
  event.preventDefault();
  const namespaceId = document.getElementById("msg-namespace-id").value;
  const text = document.getElementById("msg-text").value;
  const replyToAuthorHex = document.getElementById("msg-reply-author").value || null;
  const replyToRkey = document.getElementById("msg-reply-rkey").value || null;
  msgStatusEl.textContent = "sending…";
  try {
    const rkey = await invoke("send_message", {
      namespaceId,
      text,
      replyToAuthorHex,
      replyToRkey,
    });
    msgStatusEl.textContent = `sent ${rkey}`;
    document.getElementById("msg-text").value = "";
    await refreshMessages();
  } catch (err) {
    msgStatusEl.textContent = `error: ${err}`;
  }
});

document.getElementById("msg-refresh").addEventListener("click", refreshMessages);

// --- Tags ------------------------------------------------------------

const tagStatusEl = document.getElementById("tag-status");
const tagOutEl = document.getElementById("tag-out");

document.getElementById("tag-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  const namespaceId = document.getElementById("tag-namespace-id").value;
  const subject = document.getElementById("tag-subject").value;
  const label = document.getElementById("tag-label").value;
  tagStatusEl.textContent = "tagging…";
  try {
    await invoke("add_tag", { namespaceId, subject, label });
    tagStatusEl.textContent = `tagged at ${new Date().toLocaleTimeString()}`;
    document.getElementById("tag-label").value = "";
  } catch (err) {
    tagStatusEl.textContent = `error: ${err}`;
  }
});

document.getElementById("tag-load-btn").addEventListener("click", async () => {
  const namespaceId = document.getElementById("tag-namespace-id").value;
  const subject = document.getElementById("tag-subject").value;
  tagOutEl.innerHTML = "";
  try {
    const tags = await invoke("tags_for", { namespaceId, subject });
    if (tags.length === 0) {
      tagOutEl.textContent = "(no tags on this subject yet)";
      return;
    }
    for (const t of tags) {
      const li = document.createElement("li");
      li.textContent = `${t.label} — ${t.author_hex.slice(0, 8)}…`;
      tagOutEl.appendChild(li);
    }
  } catch (err) {
    tagOutEl.textContent = `error: ${err}`;
  }
});

// --- Inbox ---------------------------------------------------------

const inboxStatusEl = document.getElementById("inbox-status");

document
  .getElementById("submit-inbox")
  .addEventListener("submit", async (event) => {
    event.preventDefault();
    const namespaceId = document.getElementById("inbox-namespace-id").value;
    const prefix = document.getElementById("inbox-prefix").value;
    const text = document.getElementById("inbox-text").value;
    inboxStatusEl.textContent = "submitting…";
    try {
      const key = await invoke("submit_to_inbox", { namespaceId, prefix, text });
      inboxStatusEl.textContent = `submitted at ${key}`;
      document.getElementById("inbox-text").value = "";
    } catch (err) {
      inboxStatusEl.textContent = `error: ${err}`;
    }
  });

// --- Governance --------------------------------------------------------

const govEligibleEl = document.getElementById("gov-eligible");
const proposalsOutEl = document.getElementById("proposals-out");

async function loadGovernance(namespaceId) {
  proposalsOutEl.textContent = "loading…";
  try {
    const [state, proposals] = await Promise.all([
      invoke("governance_state", { namespaceId }),
      invoke("list_proposals", { namespaceId }),
    ]);
    govEligibleEl.textContent =
      state.eligible_hex.length === 0
        ? "(no governance-eligible members found — see the hint above)"
        : `eligible: ${state.eligible_hex.map((h) => h.slice(0, 12) + "…").join(", ")}`;
    renderProposals(namespaceId, proposals);
  } catch (err) {
    proposalsOutEl.textContent = `error: ${err}`;
  }
}

document
  .getElementById("load-governance")
  .addEventListener("submit", async (event) => {
    event.preventDefault();
    await loadGovernance(document.getElementById("gov-namespace-id").value);
  });

const SIGNAL_TYPES = ["consent", "stand_aside", "block", "abstain", "exit"];

function renderProposals(namespaceId, proposals) {
  proposalsOutEl.innerHTML = "";
  if (proposals.length === 0) {
    proposalsOutEl.textContent = "(no proposals yet)";
    return;
  }
  for (const p of proposals) {
    const details = document.createElement("details");
    details.open = true;

    const summary = document.createElement("summary");
    summary.className = "value";
    const statusLabel =
      p.status.state === "open"
        ? `open, ${p.status.blockers.length} block(s) so far`
        : p.status.state === "ratified"
        ? "ratified"
        : `blocked (${p.status.blockers.length})`;
    summary.textContent = `[${p.proposal.class}] ${p.proposal.title} — ${statusLabel}`;
    details.appendChild(summary);

    details.appendChild(renderJson(p.proposal));

    const form = document.createElement("form");
    form.className = "signal-form";
    const select = document.createElement("select");
    for (const t of SIGNAL_TYPES) {
      const option = document.createElement("option");
      option.value = t;
      option.textContent = t;
      select.appendChild(option);
    }
    const text = document.createElement("input");
    text.type = "text";
    text.placeholder = "optional note";
    const submit = document.createElement("button");
    submit.type = "submit";
    submit.textContent = "Signal";
    form.append(select, text, submit);

    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      submit.disabled = true;
      try {
        await invoke("create_signal", {
          namespaceId,
          proposalAuthorHex: p.author_hex,
          proposalRkey: p.rkey,
          signalType: select.value,
          text: text.value || null,
        });
        await loadGovernance(namespaceId);
      } catch (err) {
        alert(`error signaling: ${err}`);
        submit.disabled = false;
      }
    });
    details.appendChild(form);

    proposalsOutEl.appendChild(details);
  }
}

document
  .getElementById("create-proposal")
  .addEventListener("submit", async (event) => {
    event.preventDefault();
    const statusEl = document.getElementById("proposal-status");
    const namespaceId = document.getElementById("gov-namespace-id").value;
    if (!namespaceId) {
      statusEl.textContent = "load a namespace id above first";
      return;
    }
    const policyChangeRaw = document
      .getElementById("proposal-policy-change")
      .value.trim();
    let policyChange = null;
    if (policyChangeRaw) {
      try {
        policyChange = JSON.parse(policyChangeRaw);
      } catch (err) {
        statusEl.textContent = `invalid policy change JSON: ${err}`;
        return;
      }
    }
    statusEl.textContent = "posting…";
    try {
      await invoke("create_proposal", {
        namespaceId,
        title: document.getElementById("proposal-title").value,
        description: document.getElementById("proposal-description").value || null,
        class: document.getElementById("proposal-class").value,
        deadlineHours: Number(document.getElementById("proposal-deadline-hours").value),
        subjectMemberHex: document.getElementById("proposal-subject-member").value || null,
        policyChange,
      });
      statusEl.textContent = "posted";
      await loadGovernance(namespaceId);
    } catch (err) {
      statusEl.textContent = `error: ${err}`;
    }
  });

// --- Inspector -------------------------------------------------------
// Naive on purpose: no per-record-type rendering, just a generic
// recursive view over whatever dump_namespace hands back. Adding a new
// record type anywhere in atproto-iroh-core needs zero changes here.

const inspectorOutEl = document.getElementById("inspector-out");

document
  .getElementById("inspect-namespace")
  .addEventListener("submit", async (event) => {
    event.preventDefault();
    const namespaceId = document.getElementById("inspect-namespace-id").value;
    inspectorOutEl.textContent = "reading…";
    try {
      const entries = await invoke("dump_namespace", { namespaceId });
      renderEntries(entries);
    } catch (err) {
      inspectorOutEl.textContent = `error: ${err}`;
    }
  });

function renderEntries(entries) {
  inspectorOutEl.innerHTML = "";
  if (entries.length === 0) {
    inspectorOutEl.textContent = "(no entries synced yet)";
    return;
  }
  for (const entry of entries) {
    const details = document.createElement("details");
    details.open = true;

    const summary = document.createElement("summary");
    summary.className = "value";
    const when = new Date(entry.timestamp_micros / 1000).toLocaleString();
    summary.textContent = `${entry.key}  —  ${entry.author_hex.slice(0, 12)}…  —  ${when}`;
    details.appendChild(summary);

    details.appendChild(renderValue(entry.content));
    inspectorOutEl.appendChild(details);
  }
}

// Generic recursive renderer for whatever dump_namespace's `content`
// field contains: `{kind: "json", value: <anything>}`,
// `{kind: "text", value: "..."}` (freeform put_text writes), or
// `{kind: "raw", value: {hex: "..."}}`. Walks arbitrary JSON structure —
// this is the part that makes it "reflective": it never assumes a
// shape, so it never needs updating when a new record type — or a
// freeform document, or an inbox submission — shows up.
function renderValue(node) {
  if (node && node.kind === "text") {
    const pre = document.createElement("pre");
    pre.className = "value";
    pre.textContent = node.value;
    return pre;
  }
  if (node && node.kind === "raw") {
    const pre = document.createElement("pre");
    pre.className = "value";
    pre.textContent = node.value.hex
      ? `(raw, not JSON or text) ${node.value.hex}`
      : "(content not synced yet)";
    return pre;
  }
  return renderJson(node.value);
}

function renderJson(value) {
  if (value === null || typeof value !== "object") {
    const span = document.createElement("span");
    span.className = "value";
    span.textContent = JSON.stringify(value);
    return span;
  }
  const list = document.createElement("ul");
  const entries = Array.isArray(value)
    ? value.map((v, i) => [i, v])
    : Object.entries(value);
  for (const [key, v] of entries) {
    const li = document.createElement("li");
    const label = document.createElement("strong");
    label.textContent = `${key}: `;
    li.appendChild(label);
    li.appendChild(renderJson(v));
    list.appendChild(li);
  }
  return list;
}
