// Resolving "who am I" as a bare author hex — every self-identifying
// spot in this frontend (Composer, PhotosPanel, DecisionsPanel,
// ProfileEdit, Feed's header avatar, Join's profile-on-join write)
// needs the same thing: nodeDid() returns "did:iroh:<hex>", but every
// other Client method (author_hex params, ProfileView.author_hex, the
// mute list, etc.) wants the bare hex. Found duplicated inline at six
// call sites during a code-review pass — pulled out here so a future
// change to the did:iroh prefix format only needs fixing in one place.

import type { Client } from "../api/client";

export function stripDidPrefix(did: string | null | undefined): string | null {
  return did ? did.replace(/^did:iroh:/, "") : null;
}

// A node's own DID is invariant for the life of the session, but
// nodeDid() is a real Tauri IPC round-trip (tauriClient.ts's
// invoke("node_did")) — a code-review pass found it re-invoked on
// every single self-identifying action (every message send, every
// upload, every mount of a self-aware panel). Cached per `api` object
// identity (a `WeakMap`, not a module-level singleton) so a test's own
// mock `api` instance gets its own cache rather than leaking a stale
// value across tests that construct different mocks.
const cache = new WeakMap<object, Promise<string | null>>();

export async function resolveSelfAuthorHex(api: Pick<Client, "nodeDid">): Promise<string | null> {
  const cached = cache.get(api);
  if (cached) return cached;
  const promise = (async () => stripDidPrefix(await api.nodeDid()))();
  cache.set(api, promise);
  const hex = await promise;
  // A null result means there's no identity yet (node_did's own doc
  // comment: `None` before an identity is loaded) — don't pin that
  // "no self" answer forever, since it can become real once one loads;
  // only a resolved hex is safe to treat as permanent for the session.
  if (hex === null) cache.delete(api);
  return hex;
}
