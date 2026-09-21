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

document
  .getElementById("share-namespace")
  .addEventListener("submit", async (event) => {
    event.preventDefault();
    const namespaceId = document.getElementById("share-namespace-id").value;
    const mode = document.getElementById("share-mode").value;
    ticketOutEl.value = "creating ticket…";
    try {
      ticketOutEl.value = await invoke("share_namespace", {
        namespaceId,
        mode,
      });
    } catch (err) {
      ticketOutEl.value = `error: ${err}`;
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
