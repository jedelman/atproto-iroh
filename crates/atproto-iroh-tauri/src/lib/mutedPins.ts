// A pin is visible only if neither the person who pinned it nor the
// pinned message's own author is muted — Feed.tsx (the featured
// excerpt) and TableDetail.tsx (the pinned-message card) both need this
// exact rule and originally reimplemented it separately; a code-review
// pass found the duplication (and, earlier, that neither screen applied
// the rule at all — see each call site's own history).

import type { TagView } from "../api/types";

export function isPinVisible(
  pin: Pick<TagView, "author_hex" | "subject">,
  resolveMessageAuthorHex: (subject: string) => string | undefined,
  mutedAuthors: Set<string>,
): boolean {
  if (mutedAuthors.has(pin.author_hex)) return false;
  const messageAuthor = resolveMessageAuthorHex(pin.subject);
  return !messageAuthor || !mutedAuthors.has(messageAuthor);
}
