// The pinned-message icon — Feed.tsx's featured excerpt and Table
// detail's own pinned-message card both used an identical inline SVG,
// found duplicated during a code-review pass. Size differs slightly
// between the two call sites (12px vs 11px), so it's a prop, not a
// hardcoded constant.

export function PinIcon({ size = 12 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="var(--accent)">
      <path d="M12 2l1.6 5.1L19 8l-4 3.6.9 5.4-3.9-2.6-3.9 2.6.9-5.4-4-3.6 5.4-.9z" />
    </svg>
  );
}
