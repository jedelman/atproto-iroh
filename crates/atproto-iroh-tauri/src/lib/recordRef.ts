// Mirrors atproto-iroh-core's records::record_ref/parse_record_ref —
// "{author_hex}/{collection}/{rkey}", the one reference format that
// names its collection (Tag.subject can point at any record in any
// collection, so it has to).

export function recordRef(authorHex: string, collection: string, rkey: string): string {
  return `${authorHex}/${collection}/${rkey}`;
}

export function parseRecordRef(
  s: string,
): { authorHex: string; collection: string; rkey: string } | null {
  const parts = s.split("/");
  if (parts.length < 3) return null;
  const [authorHex, collection, ...rest] = parts;
  return { authorHex, collection, rkey: rest.join("/") };
}
