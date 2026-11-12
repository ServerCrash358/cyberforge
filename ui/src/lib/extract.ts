// Finds phone numbers, UPI IDs, links and SMS sender IDs in a pasted message.
// Each identifier once, in order of first appearance, in standard form. Links and UPI IDs are
// taken out of the text before looking for numbers, so their digits are not read as phones.
// Cases in shared/vectors/extract.json.
import { type IdentifierType, normalise } from "./normalise";

export interface Found {
  type: IdentifierType;
  value: string;
}

const URL_PATTERN =
  /(?:https?:\/\/|www\.)[^\s<>"']+|\b[a-z0-9][a-z0-9-]*(?:\.[a-z0-9-]+)*\.(?:in|com|net|org|co|info|online|xyz|app|link)(?:\/[^\s<>"']*)?/gi;
const UPI_PATTERN = /\b[\w.-]{2,256}@[A-Za-z][A-Za-z0-9]{1,63}\b(?!\.)/g;
const SENDER_PATTERN = /\b[A-Za-z]{2}-[A-Za-z]{6}(?:-[A-Za-z])?\b/g;
const PHONE_PATTERN = /\(?\+?\d[\d\s().-]{1,18}\d/g;
const TRAILING = /[.,;:!?)\]}'"]+$/;

export function extract(text: string): Found[] {
  const found: { at: number; type: IdentifierType; value: string }[] = [];
  let remaining = text;

  const take = (pattern: RegExp, type: IdentifierType) => {
    for (const match of remaining.matchAll(pattern)) {
      const value = normalise(type, match[0].replace(TRAILING, ""));
      if (value === null) continue;
      const start = match.index;
      found.push({ at: start, type, value });
      remaining =
        remaining.slice(0, start) + " ".repeat(match[0].length) + remaining.slice(start + match[0].length);
    }
  };
  take(UPI_PATTERN, "upi");
  take(URL_PATTERN, "url");
  take(SENDER_PATTERN, "sender_id");
  take(PHONE_PATTERN, "phone");

  const seen = new Set<string>();
  return found
    .sort((a, b) => a.at - b.at)
    .filter((f) => {
      const key = `${f.type} ${f.value}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .map(({ type, value }) => ({ type, value }));
}

/** The type and standard form of one typed value, or null when it is none of the four. */
export function detect(raw: string): Found | null {
  const text = raw.trim();
  for (const type of ["upi", "sender_id", "phone", "url"] as const) {
    const value = normalise(type, text);
    if (value !== null) return { type, value };
  }
  return null;
}
