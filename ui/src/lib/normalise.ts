// The website's copy of the API normaliser, so the extractor can show standard forms.
// It passes every case in shared/vectors/normalise.json, like the API's. The API's answer is
// still the one that counts; this only shapes what the screen shows.
import { SHORT_CODES } from "./shared";

export type IdentifierType = "phone" | "upi" | "url" | "sender_id";
export const IDENTIFIER_TYPES: IdentifierType[] = ["phone", "upi", "url", "sender_id"];

const PHONE_CHARS = /^[0-9+\-.() ]+$/;
const TOLL_FREE = ["1800", "1860"];
const UPI = /^[a-z0-9._-]{2,256}@[a-z][a-z0-9]{1,63}$/;
const HOST = /^[a-z0-9-]+(\.[a-z0-9-]+)*\.[a-z]{2,63}$/;
const SENDER = /^(?:[A-Z]{2}-)?([A-Z]{6})(?:-[A-Z])?$/;

/** The standard form of `raw`, or null when it cannot be an identifier of this type. */
export function normalise(kind: IdentifierType, raw: string): string | null {
  switch (kind) {
    case "phone":
      return phone(raw);
    case "upi":
      return upi(raw);
    case "url":
      return url(raw);
    case "sender_id":
      return sender(raw);
  }
}

function indian(national: string): string | null {
  return national.length === 10 && "23456789".includes(national.charAt(0)) ? `+91${national}` : null;
}

function phone(raw: string): string | null {
  const text = raw.trim();
  if (!text || !PHONE_CHARS.test(text)) return null;
  const bare = text.replaceAll("(", "").replaceAll(")", "").trim();
  if (bare.slice(1).includes("+")) return null;
  const digits = bare.replace(/\D/g, "");
  if (bare.startsWith("+")) return digits.startsWith("91") ? indian(digits.slice(2)) : null;
  if (SHORT_CODES.has(digits)) return digits;
  if (TOLL_FREE.some((prefix) => digits.startsWith(prefix))) {
    const rest = digits.length - 4;
    return rest >= 4 && rest < 8 ? digits : null;
  }
  if (digits.startsWith("0091")) return indian(digits.slice(4));
  if (digits.length === 12 && digits.startsWith("91")) return indian(digits.slice(2));
  if (digits.length === 11 && digits.startsWith("0")) return indian(digits.slice(1));
  return indian(digits);
}

function upi(raw: string): string | null {
  const value = raw.replace(/\s+/g, "").toLowerCase();
  return UPI.test(value) ? value : null;
}

function url(raw: string): string | null {
  const text = raw.trim();
  if (!text || /\s/.test(text)) return null;
  let rest = text;
  const scheme = text.indexOf("://");
  if (scheme >= 0) {
    if (!["http", "https"].includes(text.slice(0, scheme).toLowerCase())) return null;
    rest = text.slice(scheme + 3);
  }
  rest = rest.split("?")[0]?.split("#")[0] ?? "";
  const slash = rest.indexOf("/");
  let host = (slash >= 0 ? rest.slice(0, slash) : rest).toLowerCase();
  const path = slash >= 0 ? rest.slice(slash) : "";
  if (host.startsWith("www.") && host.slice(4).includes(".")) host = host.slice(4);
  if (!HOST.test(host)) return null;
  return `https://${host}${path.replace(/\/+$/, "")}`;
}

function sender(raw: string): string | null {
  return SENDER.exec(raw.trim().toUpperCase())?.[1] ?? null;
}
