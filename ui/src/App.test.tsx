// The whole app against a fake API: checking a message, the expired screen, and Hindi.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import type { Licence, Lookup } from "./lib/api";
import { SettingsProvider } from "./lib/settings";

const AREAS = [{ pincode: "560102", locality: "HSR Layout", city: "Bengaluru", lat: 12.9116, lng: 77.6473 }];
const SCAM: Lookup = {
  verdict: "KNOWN_SCAM",
  report_count: 5,
  distinct_reporters: 4,
  first_seen: "2026-11-10T10:00:00+05:30",
  last_seen: "2026-11-12T10:00:00+05:30",
  scam_type: "fake-kyc-link",
  display_value: "+919876543210",
  evidence: [
    {
      occurred_at: "2026-11-12T10:00:00+05:30",
      channel: "SMS or WhatsApp",
      scam_type: "fake-kyc-link",
      pincode: "560102",
    },
  ],
  challenged: false,
};

let licence: Licence;
let requests: string[];

function reply(body: unknown, status = 200) {
  return Promise.resolve(
    new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } }),
  );
}

beforeEach(() => {
  licence = { licensee: "Demo Bank", expires_at: "2027-11-12T10:00:00+05:30", valid: true };
  requests = [];
  location.hash = "";
  localStorage.clear();
  vi.stubGlobal("fetch", (input: string) => {
    requests.push(input);
    const path = input.split("?")[0];
    if (path === "/license") return reply(licence);
    if (!licence.valid) return reply({ error: "LICENSE_EXPIRED" }, 403);
    if (path === "/app/areas") return reply({ areas: AREAS });
    if (path === "/app/activity") return reply({ events: [], alerts: [] });
    if (path === "/lookup") return reply(SCAM);
    return reply({ error: "not found" }, 404);
  });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const renderApp = () =>
  render(
    <SettingsProvider>
      <App />
    </SettingsProvider>,
  );

describe("the app", () => {
  it("checks a pasted message and shows one card per identifier", async () => {
    renderApp();
    fireEvent.change(screen.getByLabelText(/Paste a number/), {
      target: { value: "Call +91 98765 43210 now" },
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "🔍 Check" }));
    });
    expect(await screen.findByText("Known scam")).toBeTruthy();
    expect(screen.getByText("Reported by 4 people near you")).toBeTruthy();
    expect(requests).toContain("/lookup?type=phone&value=%2B919876543210");
  });

  it("says 'no identifier found' for text with none", async () => {
    renderApp();
    fireEvent.change(screen.getByLabelText(/Paste a number/), { target: { value: "hello there" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "🔍 Check" }));
    });
    expect(await screen.findByText(/No phone number, UPI ID, link or SMS sender found/)).toBeTruthy();
  });

  it("shows the expired screen and asks only /license while locked", async () => {
    licence = { ...licence, valid: false };
    renderApp();
    expect((await screen.findAllByText(/Licence expired. Contact your provider./)).length).toBeGreaterThan(0);
    await waitFor(() => expect(requests.length).toBeGreaterThan(0));
    expect(requests.every((r) => r === "/license")).toBe(true);
  });

  it("switches to Hindi", async () => {
    renderApp();
    fireEvent.click(screen.getByRole("button", { name: "हिंदी" }));
    expect(await screen.findByRole("heading", { level: 1, name: /जांचें/ })).toBeTruthy();
    expect(document.documentElement.lang).toBe("hi");
  });
});
