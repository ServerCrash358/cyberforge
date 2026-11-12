// Every string exists in both languages with the same placeholders, and none uses a banned word.
import { describe, expect, it } from "vitest";
import { t } from "./i18n";
import { BANNED_WORDS, SCAM_EN, SCAM_HI, STRINGS } from "./strings";

const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe("strings", () => {
  it.each(Object.entries(STRINGS))("%s has English and Hindi with the same placeholders", (_, value) => {
    expect(value.en.trim()).not.toBe("");
    expect(value.hi.trim()).not.toBe("");
    expect(placeholders(value.hi)).toEqual(placeholders(value.en));
  });

  it("never says a banned word, in either language", () => {
    const everything = JSON.stringify([STRINGS, SCAM_EN, SCAM_HI]).toLowerCase();
    for (const word of BANNED_WORDS) expect(everything).not.toContain(word.toLowerCase());
  });

  it("uses the PRD wording for the count", () => {
    expect(t("reported_by", "en", { n: 4 })).toBe("Reported by 4 people near you");
    expect(t("reported_by", "hi", { n: 4 })).toContain("4");
  });

  it("leaves an unknown placeholder visible rather than inventing a value", () => {
    expect(t("reported_by", "en")).toBe("Reported by {n} people near you");
  });
});
