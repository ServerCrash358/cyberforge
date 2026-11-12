// How live-panel alerts and events read, in English and Hindi.
import { describe, expect, it } from "vitest";
import type { ActivityEvent } from "../lib/api";
import { type Lang, type StringKey, t } from "../lib/i18n";
import { alertText, eventDetails, eventTitle } from "./LivePanel";

const tr = (lang: Lang) => (key: StringKey, values?: Record<string, string | number>) => t(key, lang, values);
const event = (over: Partial<ActivityEvent>): ActivityEvent => ({
  at: "2026-11-12T21:17:30+05:30",
  kind: "report",
  pincode: "560102",
  locality: "HSR Layout",
  scam_type: "fake-kyc-link",
  channel: "Phone call",
  identifier_type: "phone",
  display: "98XXXXX210",
  detail: null,
  count: 1,
  ...over,
});

describe("live panel wording", () => {
  it("describes each kind of event", () => {
    const en = tr("en");
    expect(eventTitle(event({}), en)).toBe("Report counted");
    expect(eventTitle(event({ kind: "merged", count: 3 }), en)).toBe(
      "Repeat from the same person or connection, counted once (3)",
    );
    expect(eventTitle(event({ kind: "burst", count: 40 }), en)).toBe(
      "Coordinated burst: 40 reports counted as one person",
    );
    expect(eventTitle(event({ kind: "rejected", detail: "unknown area" }), en)).toBe(
      "Junk report rejected: unknown area",
    );
    expect(eventTitle(event({ kind: "rejected", detail: "invalid phone" }), tr("hi"))).toContain(
      "सही फ़ोन नंबर नहीं",
    );
  });

  it("shows channel, type, place, the masked value and the time, never a reporter", () => {
    const parts = eventDetails(event({}), tr("en"), "en");
    expect(parts).toEqual(["📞 Phone call", "🏦 Fake KYC link", "📍 HSR Layout", "98XXXXX210", "21:17:30"]);
  });

  it("words spikes and new types", () => {
    const spike = {
      type: "SPIKE" as const,
      pincode: "560034",
      window: "24h",
      count: 11,
      baseline_per_day: 1.57,
    };
    expect(alertText({ ...spike, locality: "Koramangala" }, tr("en"), "en")).toBe(
      "📈 Koramangala: 11 reports in 24 hours, usually about 1.57 a day",
    );
    const fresh = { type: "NEW_TYPE" as const, pincode: "560102", scam_type: "sextortion", window: "24h" };
    expect(alertText({ ...fresh, locality: null }, tr("en"), "en")).toBe(
      "🆕 560102: reported suspicious activity, Video call blackmail",
    );
  });
});
