// The built-in sticker/avatar set (DESIGN_BRIEF.md: "even bots need cute
// stickers", profile customization as part of first-run onboarding, not
// a settings afterthought). Each sticker is an organic asymmetric blob
// (irregular border-radius, on purpose — "warmth over polish, a little
// imperfect" is one of .impeccable.md's Design Principles, not just a
// slogan) with a simple face, drawn in CSS rather than shipped as image
// assets: zero asset pipeline, and it's exactly the placeholder-quality
// the /shape mockup artifact used. Real illustration work is still a
// named open item — this is a faithful, working stand-in, not the final
// art.

export const STICKER_IDS = [
  "accent",
  "accent-strong",
  "gold",
  "sage",
  "rose",
  "plum",
  "sky",
] as const;

export type StickerId = (typeof STICKER_IDS)[number];

const BLOB_SHAPES = [
  "46% 54% 50% 50%/54% 46% 52% 48%",
  "56% 44% 58% 42%/44% 56% 42% 58%",
  "48% 52% 54% 46%/58% 42% 52% 48%",
  "44% 56% 50% 50%/52% 48% 54% 46%",
  "52% 48% 46% 54%/48% 54% 46% 52%",
  "50% 50% 44% 56%/56% 44% 50% 50%",
];

function isStickerId(value: string): value is StickerId {
  return (STICKER_IDS as readonly string[]).includes(value);
}

/** Picks one of a fixed set of blob shapes deterministically from a
 * string (a sticker id, or an author hex) so the same person's sticker
 * always renders with the same silhouette, not a new random one per
 * render. Not cryptographic — just enough spread to look varied. */
function shapeFor(seed: string): string {
  let hash = 0;
  for (let i = 0; i < seed.length; i++) hash = (hash * 31 + seed.charCodeAt(i)) | 0;
  return BLOB_SHAPES[Math.abs(hash) % BLOB_SHAPES.length];
}

export function Sticker({
  id,
  size = 44,
  ring = false,
}: {
  id: string | null | undefined;
  size?: number;
  ring?: boolean;
}) {
  const stickerId: StickerId = id && isStickerId(id) ? id : "accent";
  const color = `var(--${stickerId})`;
  const shape = shapeFor(stickerId + size);
  const eye = Math.max(3, Math.round(size * 0.11));
  const mouthW = Math.round(size * 0.36);
  const mouthH = Math.round(size * 0.18);

  return (
    <div
      role="img"
      aria-label={`Sticker: ${stickerId}`}
      style={{
        width: size,
        height: size,
        borderRadius: shape,
        background: color,
        position: "relative",
        flexShrink: 0,
        boxShadow: ring
          ? `0 0 0 2px var(--bg), 0 0 0 ${Math.max(3, size * 0.06)}px var(--accent)`
          : undefined,
      }}
    >
      <div
        style={{
          position: "absolute",
          left: size * 0.3,
          top: size * 0.37,
          width: eye,
          height: eye,
          borderRadius: "50%",
          background: "var(--ink)",
        }}
      />
      <div
        style={{
          position: "absolute",
          right: size * 0.3,
          top: size * 0.37,
          width: eye,
          height: eye,
          borderRadius: "50%",
          background: "var(--ink)",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: (size - mouthW) / 2,
          top: size * 0.55,
          width: mouthW,
          height: mouthH,
          border: `${Math.max(1.5, size * 0.035)}px solid var(--ink)`,
          borderTop: "none",
          borderRadius: `0 0 ${mouthW}px ${mouthW}px`,
        }}
      />
    </div>
  );
}
