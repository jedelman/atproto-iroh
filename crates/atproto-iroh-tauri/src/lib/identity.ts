// Resolving "who am I" as a bare author hex — every self-identifying spot
// in this frontend (Composer, PhotosPanel, DecisionsPanel, ProfileEdit,
// Feed's header avatar, Join's profile-on-join check) compares against
// it.
//
// It must be the key that *signs* your records (`selfAuthorHex`), not
// the node's network identity (`nodeDid`). Those are two different keys
// on a real device — iroh-docs mints its own author — and this used to
// strip the `did:iroh:` prefix off `nodeDid` instead. The mock used one
// key for both, so it looked right in every test and screenshot; on a
// phone, a just-sent message was pinned under an address that would
// never exist ("the pinned message hasn't synced yet" with the message
// right below it), and a founder never saw Support / Object.

import type { Client } from "../api/client";

// Invariant for the session but a real IPC round-trip, so cached per
// `api` object (a WeakMap, so each test's mock gets its own cache).
const cache = new WeakMap<object, Promise<string | null>>();

export async function resolveSelfAuthorHex(api: Pick<Client, "selfAuthorHex">): Promise<string | null> {
  const cached = cache.get(api);
  if (cached) return cached;
  // Fails before the node is spawned — answer "no self yet" and don't
  // cache it, since it becomes real once the node is up.
  const promise = api.selfAuthorHex().catch(() => null);
  cache.set(api, promise);
  const hex = await promise;
  if (hex === null) cache.delete(api);
  return hex;
}
