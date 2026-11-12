// The same shared vectors the API passes, so the website and the API agree on standard forms.
import { describe, expect, it } from "vitest";
import extractCases from "../../../shared/vectors/extract.json";
import normaliseCases from "../../../shared/vectors/normalise.json";
import { detect, extract } from "./extract";
import { type IdentifierType, normalise } from "./normalise";

describe("normalise", () => {
  it.each(normaliseCases.cases)("$type $input", ({ type, input, expected }) => {
    expect(normalise(type as IdentifierType, input)).toBe(expected);
  });
});

describe("extract", () => {
  it.each(extractCases.cases)("$text", ({ text, expected }) => {
    expect(extract(text)).toEqual(expected);
  });
});

describe("detect", () => {
  it("names the type of a single value", () => {
    expect(detect(" 98765-43210 ")).toEqual({ type: "phone", value: "+919876543210" });
    expect(detect("Ravi12@OKAXIS")).toEqual({ type: "upi", value: "ravi12@okaxis" });
    expect(detect("VM-HDFCBK")).toEqual({ type: "sender_id", value: "HDFCBK" });
    expect(detect("sbi-kyc.in/pay/")).toEqual({ type: "url", value: "https://sbi-kyc.in/pay" });
    expect(detect("hello")).toBeNull();
  });
});
