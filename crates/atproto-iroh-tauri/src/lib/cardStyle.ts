// The "surface + 1px border + rounded corners" container was written
// inline at a dozen-plus call sites across Feed/TableDetail/Mute (found
// in a /simplify pass) — the same visual object (a card) every time,
// just with different padding/radius/border-color per site. Two base
// styles for the two radii actually in use; call sites spread one in
// and override padding/border as needed rather than redeclaring the
// surface/border pair each time.

import type { CSSProperties } from "react";

export const cardStyle: CSSProperties = {
  background: "var(--surface)",
  border: "1px solid var(--border)",
  borderRadius: 16,
};

export const rowCardStyle: CSSProperties = {
  background: "var(--surface)",
  border: "1px solid var(--border)",
  borderRadius: 12,
};
