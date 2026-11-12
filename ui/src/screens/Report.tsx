// Report, icon first: how it reached you, when, the identifier, the kind of scam, your area.
// One screen, no login. Builds the PRD Appendix A body: times in +05:30, the session's random
// device id as reporter_id, no ip_prefix (the server keeps only a keyed hash of the connection).
import { useEffect, useState } from "react";
import { Button, Field, INPUT, Notice, Pills } from "../components/ui";
import { ApiError, api } from "../lib/api";
import { detect } from "../lib/extract";
import { channelLabel, scamLabel } from "../lib/i18n";
import { useSettings } from "../lib/settings";
import { CHANNEL_IDS, SCAM_TYPE_IDS } from "../lib/shared";
import { CHANNEL_ICONS, SCAM_ICONS, WHEN_ICONS } from "../lib/strings";

const WHENS = ["now", "today", "yesterday", "pick"] as const;
type When = (typeof WHENS)[number];
const IST_MS = (5 * 60 + 30) * 60 * 1000;

/** An ISO time with the +05:30 offset, to the second, as the PRD examples write it. */
export function istIso(when: Date): string {
  return `${new Date(when.getTime() + IST_MS).toISOString().slice(0, 19)}+05:30`;
}

/** When the incident happened, from the quick picks; a picked date and time is India time. */
export function occurredAt(choice: When, picked: string, now: Date = new Date()): Date {
  if (choice === "today") {
    const istNow = new Date(now.getTime() + IST_MS);
    const midnight = Date.UTC(istNow.getUTCFullYear(), istNow.getUTCMonth(), istNow.getUTCDate()) - IST_MS;
    return new Date(Math.max(midnight, now.getTime() - 3 * 3600 * 1000));
  }
  if (choice === "yesterday") return new Date(now.getTime() - 24 * 3600 * 1000);
  if (choice === "pick" && picked) {
    const when = new Date(`${picked}:00+05:30`);
    return Number.isNaN(when.getTime()) ? now : new Date(Math.min(when.getTime(), now.getTime()));
  }
  return now;
}

type Outcome = "ok" | "missing" | "bad_value" | "slow_down" | "rejected" | "offline" | null;

export function Report() {
  const { tr, lang, label, guard, deviceId, areas, myArea, setMyArea, prefill, setPrefill } = useSettings();
  const [channel, setChannel] = useState<string | null>(null);
  const [when, setWhen] = useState<When>("now");
  const [picked, setPicked] = useState("");
  const [value, setValue] = useState("");
  const [scamType, setScamType] = useState<string | null>(null);
  const [pincode, setPincode] = useState(myArea ?? "");
  const [note, setNote] = useState("");
  const [outcome, setOutcome] = useState<Outcome>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (prefill) {
      setValue(prefill);
      setPrefill("");
    }
  }, [prefill, setPrefill]);

  const send = async () => {
    const area = areas.find((a) => a.pincode === pincode);
    const detected = detect(value);
    if (!(channel && scamType && area)) return setOutcome("missing");
    if (detected === null) return setOutcome("bad_value");
    setMyArea(area.pincode);
    setBusy(true);
    try {
      await guard(() =>
        api.report({
          report_id: `web_${crypto.randomUUID().replaceAll("-", "")}`,
          occurred_at: istIso(occurredAt(when, picked)),
          reported_at: istIso(new Date()),
          reporter_id: deviceId,
          channel,
          scam_type: scamType,
          identifier_type: detected.type,
          identifier_value: value.trim(),
          pincode: area.pincode,
          locality: area.locality,
          city: area.city,
          note: note.trim(),
          has_screenshot: false,
          amount_lost_inr: 0,
        }),
      );
      setOutcome("ok");
      setValue("");
      setNote("");
    } catch (e) {
      if (e instanceof ApiError) {
        setOutcome(e.status === 429 ? "slow_down" : e.status === 0 ? "offline" : "rejected");
      }
    } finally {
      setBusy(false);
    }
  };

  const messages: Record<Exclude<Outcome, null>, [tone: "good" | "warn" | "bad", text: string]> = {
    ok: ["good", `✅ ${tr("report_ok")}`],
    missing: ["warn", tr("report_missing")],
    bad_value: ["warn", tr("report_bad_value")],
    slow_down: ["bad", tr("report_slow_down")],
    rejected: ["bad", tr("report_rejected")],
    offline: ["warn", `📡 ${tr("unreachable")}`],
  };

  return (
    <div className="flex flex-col gap-5">
      <p className="text-sm text-muted">🔒 {tr("report_privacy")}</p>
      <Pills
        legend={tr("report_channel")}
        options={CHANNEL_IDS}
        value={channel}
        onChange={setChannel}
        render={(c) => label((CHANNEL_ICONS as Record<string, string>)[c] ?? "❓", channelLabel(c, lang))}
      />
      <Pills
        legend={tr("report_when")}
        options={WHENS}
        value={when}
        onChange={setWhen}
        render={(w) => label(WHEN_ICONS[w], tr(`when_${w}`))}
      />
      {when === "pick" && (
        <Field id="report-day" label={tr("when_pick")}>
          <input
            id="report-day"
            type="datetime-local"
            value={picked}
            max={istIso(new Date()).slice(0, 16)}
            onChange={(e) => setPicked(e.target.value)}
            className={INPUT}
          />
        </Field>
      )}
      <Field id="report-value" label={label("📋", tr("report_what"))}>
        <input
          id="report-value"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          autoComplete="off"
          className={INPUT}
        />
      </Field>
      <Pills
        legend={tr("report_type")}
        options={SCAM_TYPE_IDS}
        value={scamType}
        onChange={setScamType}
        render={(s) => label((SCAM_ICONS as Record<string, string>)[s] ?? "❓", scamLabel(s, lang))}
      />
      <Field id="report-area" label={label("📍", tr("report_area"))}>
        <select
          id="report-area"
          value={pincode}
          onChange={(e) => setPincode(e.target.value)}
          className={INPUT}
        >
          <option value="" disabled>
            📍
          </option>
          {areas.map((a) => (
            <option key={a.pincode} value={a.pincode}>
              {a.locality}, {a.city}
            </option>
          ))}
        </select>
      </Field>
      <Field id="report-note" label={label("✍️", tr("report_note"))}>
        <textarea
          id="report-note"
          value={note}
          maxLength={1000}
          rows={3}
          onChange={(e) => setNote(e.target.value)}
          className={INPUT}
        />
      </Field>
      <Button onClick={send} disabled={busy} className="w-full text-lg">
        {label("📣", tr("report_submit"))}
      </Button>
      {outcome && <Notice tone={messages[outcome][0]}>{messages[outcome][1]}</Notice>}
    </div>
  );
}
