# atproto-iroh-tauri

Reference client for `atproto-iroh-core` — see the root `CLAUDE.md`'s
"Client" section for why Tauri, and `../../SPEC.md` for the protocol
this is a UI on top of.

## What's real here

- `src-tauri/` is a genuine Tauri 2 app: `atproto-iroh-core` is a normal
  path dependency, no IPC or FFI boundary between UI-adjacent code and
  protocol logic.
- Six commands, each a direct call into the core crate: `spawn_node`
  (generates a `did:iroh` identity and starts a real `iroh` node —
  deliberately not done at app launch; see `main.rs`'s comment on why),
  `node_did`, `create_namespace_with_profile` (creates a namespace and
  publishes a `NodeProfile` into it), `list_namespaces`, `share_namespace`
  (`Node::share`, unwrapped to a plain paste-able ticket string), and
  `join_namespace` (`Node::join`, which SPEC.md §6 item 10 confirmed
  backfills full history, not just future writes — a real join).
  `AppState.docs` holds every namespace this node currently has open, so
  a later command can name one by id without re-deriving it from a
  ticket each time.
- `dist/` is plain HTML/CSS/JS — no bundler, no `node_modules`, no
  framework. `tauri.conf.json` sets `withGlobalTauri: true` so
  `window.__TAURI__` is injected directly; `main.js` calls `invoke()`
  against it with zero build step. Deliberate, not a placeholder waiting
  for a real frontend stack — see the styling philosophy note below.

## What's not built yet

This proves the wiring, not a usable app. Missing, in roughly the order
a real client would need them:

- **Governance.** `fold.rs`/`propose`/`signal` aren't called from any
  command yet — no way to see a namespace's `Proposal`s, post one, or
  signal on one from the UI. The most obviously missing piece now that
  sharing/joining is wired up — two people can be in the same namespace
  but have no way to actually govern it from this client.
- **Persistence.** `Node::spawn` uses `Docs::memory()` — nothing survives
  a restart. Needs `Docs::persistent` plus somewhere sensible to put the
  data directory (see `mute.rs`'s `default_path` for the XDG-ish
  convention already established elsewhere in the core crate; this
  should probably follow the same one).
- **Mute UI.** `mute::MuteList` exists and is tested; nothing here reads
  or writes it.
- **Any error/loading state beyond `textContent = "error: ..."`.** Fine
  for proving the wiring, not fine for anyone else to use.

## Building

**Verified, not just written.** `cargo build -p atproto-iroh-tauri`
compiles clean (zero warnings) as of this scaffold — Tauri's Linux
backend needs `webkit2gtk-4.1` and `gtk3` development headers available
to `pkg-config` at build time (`apt-get install libwebkit2gtk-4.1-dev
libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev` on Debian/Ubuntu;
see [Tauri's prerequisites docs](https://tauri.app/start/prerequisites/)
for other platforms), which the container this was scaffolded in didn't
have until they were installed specifically to run this check — noted
because "it builds" should mean it was actually built, not assumed from
the source looking right. macOS/Windows ship their own webviews and
don't need this step. `icons/icon.png` is a flat placeholder color, not
real artwork — `tauri::generate_context!()` requires *some* icon to
exist at that path even with `bundle.active: false`, found by hitting
the error, not anticipated.

Actually running it (`cargo tauri dev`) needs `tauri-cli`
(`cargo install tauri-cli --version "^2"`) and, in a container/CI
context, a display — not attempted here, since a headless environment
has nothing to show a window on regardless of whether the binary itself
is correct.

## Styling philosophy

Repo is MIT-licensed specifically so a rougher-but-functional reference
UI is a fine place to start (Jason's framing, kept verbatim in spirit):
*"I spent my time in the trenches pushing pixels, I've earned some
inconsistencies. If people want pretty, they can make it pretty."*
`dist/styles.css` is intentionally undesigned — system fonts, no visual
identity, nothing here should be read as this project's design
direction, because it doesn't have one yet on purpose.
