// The "grid of STICKER_IDS as selectable buttons" pattern — Onboarding's
// first-run picker and ProfileEdit's avatar picker each reimplemented this
// grid separately (found in a /simplify pass), differing only in sticker
// size and grid gap.

import { Sticker, STICKER_IDS, type StickerId } from "./Sticker";

export function StickerPicker({
  value,
  onChange,
  size = 44,
  gap = 14,
}: {
  value: StickerId;
  onChange: (id: StickerId) => void;
  size?: number;
  gap?: number;
}) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap }}>
      {STICKER_IDS.map((id) => (
        <button
          key={id}
          onClick={() => onChange(id)}
          aria-label={`Choose ${id} sticker`}
          aria-pressed={value === id}
          style={{ background: "none", border: "none", padding: 0, display: "flex", justifyContent: "center" }}
        >
          <Sticker id={id} size={size} ring={value === id} />
        </button>
      ))}
    </div>
  );
}
