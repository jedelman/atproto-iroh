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

// --- Shared doc --------------------------------------------------------

const docStatusEl = document.getElementById("doc-status");

document.getElementById("write-text").addEventListener("submit", async (event) => {
  event.preventDefault();
  const namespaceId = document.getElementById("doc-namespace-id").value;
  const key = document.getElementById("doc-key").value;
  const text = document.getElementById("doc-text").value;
  docStatusEl.textContent = "saving…";
  try {
    await invoke("write_text", { namespaceId, key, text });
    docStatusEl.textContent = `saved at ${new Date().toLocaleTimeString()}`;
  } catch (err) {
    docStatusEl.textContent = `error: ${err}`;
  }
});

document.getElementById("doc-load").addEventListener("click", async () => {
  const namespaceId = document.getElementById("doc-namespace-id").value;
  const key = document.getElementById("doc-key").value;
  docStatusEl.textContent = "loading…";
  try {
    const text = await invoke("read_text", { namespaceId, key });
    document.getElementById("doc-text").value = text ?? "";
    docStatusEl.textContent =
      text === null ? "(nothing at that key yet)" : "loaded";
  } catch (err) {
    docStatusEl.textContent = `error: ${err}`;
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
