// Looks up English and Hindi strings and labels. One dictionary, no i18n library.
import { CHANNEL_HI, SCAM_EN, SCAM_HI, STRINGS } from "./strings";

export type Lang = "en" | "hi";
export const LANGS: Record<Lang, string> = { en: "English", hi: "हिंदी" };
export type StringKey = keyof typeof STRINGS;

export function isKey(key: string): key is StringKey {
  return key in STRINGS;
}

/** The string for `key` in `lang`, with `{name}` placeholders filled in. */
export function t(key: StringKey, lang: Lang, values: Record<string, string | number> = {}): string {
  return STRINGS[key][lang].replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in values ? String(values[name]) : whole,
  );
}

export function channelLabel(channel: string, lang: Lang): string {
  if (lang === "en") return channel;
  return (CHANNEL_HI as Record<string, string>)[channel] ?? channel;
}

export function scamLabel(scamType: string, lang: Lang): string {
  const table: Record<string, string> = lang === "hi" ? SCAM_HI : SCAM_EN;
  return table[scamType] ?? table.other ?? scamType;
}
