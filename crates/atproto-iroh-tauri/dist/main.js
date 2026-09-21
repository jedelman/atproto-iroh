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
    } catch (err) {
      namespaceEl.textContent = `error: ${err}`;
    }
  });
