// Finding the message a pin/tag subject refers to within a Feed's item
// list — featuredPin and pinExcerpt in Feed.tsx both needed this exact
// lookup and originally reimplemented it separately.

import type { FeedItem } from "../hooks/useFeed";
import type { MessageView } from "../api/types";

export function findMessageBySubject(items: FeedItem[], subject: string): MessageView | undefined {
  const match = items.find((i) => i.kind === "message" && i.message.subject === subject);
  return match?.kind === "message" ? match.message : undefined;
}
