// Pure pieces of the screens: report times, the address router and the How it works text.
import { describe, expect, it } from "vitest";
import { screenFromHash } from "../App";
import { renderBlocks } from "./How";
import { istIso, occurredAt } from "./Report";

const NOW = new Date("2026-11-12T15:30:00Z"); // 21:00 in India

describe("report times", () => {
  it("writes India time with the +05:30 offset, to the second", () => {
    expect(istIso(new Date("2026-11-12T15:30:45.678Z"))).toBe("2026-11-12T21:00:45+05:30");
  });

  it("turns the quick picks into times", () => {
    expect(istIso(occurredAt("now", "", NOW))).toBe("2026-11-12T21:00:00+05:30");
    expect(istIso(occurredAt("today", "", NOW))).toBe("2026-11-12T18:00:00+05:30");
    expect(istIso(occurredAt("yesterday", "", NOW))).toBe("2026-11-11T21:00:00+05:30");
    expect(istIso(occurredAt("pick", "2026-11-10T09:15", NOW))).toBe("2026-11-10T09:15:00+05:30");
  });

  it("never lets a picked time be in the future or invalid", () => {
    expect(occurredAt("pick", "2027-01-01T00:00", NOW)).toEqual(NOW);
    expect(occurredAt("pick", "nonsense", NOW)).toEqual(NOW);
  });

  it("keeps 'earlier today' on today just after midnight", () => {
    const early = new Date("2026-11-11T19:00:00Z"); // 00:30 in India
    expect(istIso(occurredAt("today", "", early))).toBe("2026-11-12T00:00:00+05:30");
  });
});

describe("addresses", () => {
  it("maps #/name to a screen and anything else to Check", () => {
    expect(screenFromHash("#/map")).toBe("map");
    expect(screenFromHash("#licence")).toBe("licence");
    expect(screenFromHash("")).toBe("check");
    expect(screenFromHash("#/admin")).toBe("check");
  });
});

describe("How it works text", () => {
  it("renders paragraphs and bullet lists", () => {
    const blocks = renderBlocks("**Title**\n\n- one\n- **two**");
    expect(blocks).toHaveLength(2);
  });
});
