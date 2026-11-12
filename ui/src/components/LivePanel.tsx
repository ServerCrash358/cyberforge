// The live panel beside every screen: alerts live right now, then everything that happened.
// It refreshes every few seconds. Numbers are always masked; no reporter detail ever appears.
import { useEffect, useState } from "react";
import { type ActivityEvent, ApiError, api, type Feed, LicenceExpired, type LiveAlert } from "../lib/api";
import { channelLabel, isKey, type Lang, type StringKey, scamLabel } from "../lib/i18n";
import { clockTime, useSettings } from "../lib/settings";
import { CHANNEL_ICONS, SCAM_ICONS } from "../lib/strings";

export const REFRESH_MS = 3000;
const EVENT_ICONS: Record<string, string> = {
  report: "✅",
  merged: "🔁",
  burst: "🚩",
  rejected: "⛔",
  rate_limited: "⏳",
  known_scam: "🔴",
  dispute: "🏪",
  alert: "🔔",
};

type Tr = (key: StringKey, values?: Record<string, string | number>) => string;

export function alertText(alert: LiveAlert, tr: Tr, lang: Lang): string {
  const area = alert.locality ?? alert.pincode;
  if (alert.type === "SPIKE") {
    return `📈 ${tr("alert_spike_short", { area, count: alert.count, base: alert.baseline_per_day })}`;
  }
  return `🆕 ${tr("alert_new_short", { area, kind: scamLabel(alert.scam_type, lang) })}`;
}

export function eventTitle(event: ActivityEvent, tr: Tr): string {
  const { kind, count } = event;
  let title: string;
  if (kind === "burst") {
    title = tr("ev_burst", { count });
  } else if (kind === "rejected") {
    const key = `reason_${event.detail}`;
    title = tr("ev_rejected", { reason: isKey(key) ? tr(key) : String(event.detail) });
  } else {
    const key = `ev_${kind}`;
    title = isKey(key) ? tr(key) : kind;
  }
  return count > 1 && kind !== "burst" ? `${title} (${count})` : title;
}

export function eventDetails(event: ActivityEvent, tr: Tr, lang: Lang): string[] {
  const parts: string[] = [];
  if (event.kind === "alert") {
    if (event.detail === "SPIKE") parts.push(`📈 ${tr("kind_spike")}`);
    else if (event.scam_type) parts.push(`🆕 ${scamLabel(event.scam_type, lang)}`);
  }
  if (event.channel) {
    const icon = (CHANNEL_ICONS as Record<string, string>)[event.channel] ?? "❓";
    parts.push(`${icon} ${channelLabel(event.channel, lang)}`);
  }
  if (event.scam_type && event.kind !== "alert") {
    const icon = (SCAM_ICONS as Record<string, string>)[event.scam_type] ?? "❓";
    parts.push(`${icon} ${scamLabel(event.scam_type, lang)}`);
  }
  if (event.locality) parts.push(`📍 ${event.locality}`);
  if (event.display) parts.push(event.display);
  parts.push(clockTime(event.at));
  return parts;
}

export function LivePanel() {
  const { tr, lang, guard, expired, setExpired } = useSettings();
  const [feed, setFeed] = useState<Feed | null>(null);
  const [offline, setOffline] = useState(false);

  useEffect(() => {
    let alive = true;
    // /license always answers, so while the licence is locked the panel asks only that, and the
    // site notices a renewed licence within one refresh without a single refused request.
    const load = async () => {
      try {
        const licence = await api.licence();
        if (!alive) return;
        setExpired(!licence.valid);
        setOffline(false);
        if (!licence.valid) return setFeed(null);
        const next = await guard(api.activity);
        if (alive) setFeed(next);
      } catch (e) {
        if (!alive) return;
        // A locked licence hides the data, including what was already on screen.
        if (e instanceof LicenceExpired) setFeed(null);
        if (e instanceof ApiError) setOffline(true);
      }
    };
    void load();
    const timer = setInterval(load, REFRESH_MS);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, [guard, setExpired]);

  return (
    <aside aria-label={tr("panel_alerts")} className="flex flex-col gap-3">
      <h2 className="text-lg font-bold">🔔 {tr("panel_alerts")}</h2>
      {expired && <p className="text-sm text-red-800">🔒 {tr("expired")}</p>}
      {offline && <p className="text-sm text-muted">📡 {tr("unreachable")}</p>}
      {feed && feed.alerts.length === 0 && <p className="text-sm text-muted">{tr("panel_no_alerts")}</p>}
      <ul className="flex flex-col gap-2">
        {feed?.alerts.map((alert) => (
          <li
            key={`${alert.type}-${alert.pincode}-${alert.type === "NEW_TYPE" ? alert.scam_type : ""}`}
            className="rounded-xl border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-950"
          >
            {alertText(alert, tr, lang)}
          </li>
        ))}
      </ul>
      <h2 className="mt-2 text-lg font-bold">🧾 {tr("panel_activity")}</h2>
      {feed && feed.events.length === 0 && <p className="text-sm text-muted">{tr("panel_quiet")}</p>}
      <ol className="flex max-h-[32rem] flex-col gap-2 overflow-y-auto pr-1" aria-live="polite">
        {feed?.events.map((event) => (
          <li key={`${event.at}-${event.kind}-${event.display}-${event.detail}`} className="text-sm">
            <p className="font-semibold">
              <span aria-hidden="true">{EVENT_ICONS[event.kind] ?? "•"}</span> {eventTitle(event, tr)}
            </p>
            <p className="text-muted">{eventDetails(event, tr, lang).join(" · ")}</p>
          </li>
        ))}
      </ol>
      <p className="text-xs text-muted">🔒 {tr("panel_privacy")}</p>
    </aside>
  );
}
