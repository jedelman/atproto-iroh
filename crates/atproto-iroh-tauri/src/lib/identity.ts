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

export async function resolveSelfAuthorHex(api: Pick<Client, "nodeDid">): Promise<string | null> {
  return stripDidPrefix(await api.nodeDid());
}
