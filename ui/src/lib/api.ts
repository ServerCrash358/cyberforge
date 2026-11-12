// Talks to the API on the same origin. Every 403 LICENSE_EXPIRED becomes LicenceExpired, so each
// screen shows the PRD's expired message the same way; a network failure becomes status 0.
import type { IdentifierType } from "./normalise";

export type Verdict = "KNOWN_SCAM" | "SUSPICIOUS" | "NO_REPORTS" | "VERIFIED_BUSINESS";

export interface Evidence {
  occurred_at: string;
  channel: string;
  scam_type: string;
  pincode: string;
}

export interface Lookup {
  verdict: Verdict;
  report_count: number;
  distinct_reporters: number;
  first_seen: string | null;
  last_seen: string | null;
  scam_type: string | null;
  display_value: string;
  evidence: Evidence[];
  challenged: boolean;
}

export interface Area {
  pincode: string;
  locality: string;
  city: string;
  lat: number;
  lng: number;
}

export interface Cluster extends Area {
  count: number;
  top_scam_type: string;
  severity: "CRIT" | "HIGH" | "MED";
}

export type Alert =
  | { type: "SPIKE"; pincode: string; window: string; count: number; baseline_per_day: number }
  | { type: "NEW_TYPE"; pincode: string; scam_type: string; window: string };

export type LiveAlert = Alert & { locality: string | null };

export interface ActivityEvent {
  at: string;
  kind: string;
  pincode: string | null;
  locality: string | null;
  scam_type: string | null;
  channel: string | null;
  identifier_type: string | null;
  display: string | null;
  detail: string | null;
  count: number;
}

export interface Feed {
  events: ActivityEvent[];
  alerts: LiveAlert[];
}

export interface Licence {
  licensee: string | null;
  expires_at: string | null;
  valid: boolean;
}

export interface ReportBody {
  report_id: string;
  occurred_at: string;
  reported_at: string;
  reporter_id: string;
  channel: string;
  scam_type: string;
  identifier_type: IdentifierType;
  identifier_value: string;
  pincode: string;
  locality: string;
  city: string;
  note: string;
  has_screenshot: boolean;
  amount_lost_inr: number;
}

export interface ChallengeBody {
  identifier_type: IdentifierType;
  identifier_value: string;
  claimant_name: string;
  contact_email: string;
  proof_note: string;
}

export class LicenceExpired extends Error {}

export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

async function call<T>(method: "GET" | "POST", path: string, body?: unknown): Promise<T> {
  let response: Response;
  try {
    response = await fetch(path, {
      method,
      headers: body === undefined ? {} : { "content-type": "application/json" },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
  } catch {
    throw new ApiError(0, "server unreachable");
  }
  const reply: unknown = await response.json().catch(() => ({}));
  const error = typeof reply === "object" && reply !== null && "error" in reply ? String(reply.error) : "";
  if (response.status === 403 && error === "LICENSE_EXPIRED") throw new LicenceExpired();
  if (!response.ok) throw new ApiError(response.status, error || "request failed");
  return reply as T;
}

const query = (params: Record<string, string | number>) =>
  new URLSearchParams(Object.entries(params).map(([k, v]) => [k, String(v)])).toString();

export const api = {
  lookup: (type: IdentifierType, value: string) => call<Lookup>("GET", `/lookup?${query({ type, value })}`),
  report: (body: ReportBody) => call<{ case_id: string; merged: boolean }>("POST", "/reports", body),
  map: (pincode: string, days: number, radius_km: number) =>
    call<{ clusters: Cluster[] }>("GET", `/map?${query({ pincode, days, radius_km })}`).then(
      (r) => r.clusters,
    ),
  alerts: (pincode: string) =>
    call<{ alerts: Alert[] }>("GET", `/alerts?${query({ pincode })}`).then((r) => r.alerts),
  challenge: (body: ChallengeBody) => call<{ challenge_id: string }>("POST", "/challenges", body),
  licence: () => call<Licence>("GET", "/license"),
  uploadLicence: (doc: unknown) => call<Licence>("POST", "/license", doc),
  areas: () => call<{ areas: Area[] }>("GET", "/app/areas").then((r) => r.areas),
  activity: () => call<Feed>("GET", "/app/activity"),
};
