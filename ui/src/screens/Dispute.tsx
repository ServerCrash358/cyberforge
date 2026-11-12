// Dispute: a business says "this is our real number" (PRD bonus). One dispute per number.
import { useState } from "react";
import { Button, Field, INPUT, Notice } from "../components/ui";
import { ApiError, api } from "../lib/api";
import { detect } from "../lib/extract";
import { useSettings } from "../lib/settings";

type Outcome = "ok" | "missing" | "bad_value" | "limit" | "rejected" | "offline" | null;

export function Dispute() {
  const { tr, label, guard } = useSettings();
  const [value, setValue] = useState("");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [proof, setProof] = useState("");
  const [outcome, setOutcome] = useState<Outcome>(null);

  const send = async () => {
    const detected = detect(value);
    if (!(name.trim() && email.trim() && proof.trim())) return setOutcome("missing");
    if (detected === null) return setOutcome("bad_value");
    try {
      await guard(() =>
        api.challenge({
          identifier_type: detected.type,
          identifier_value: value.trim(),
          claimant_name: name.trim(),
          contact_email: email.trim(),
          proof_note: proof.trim(),
        }),
      );
      setOutcome("ok");
    } catch (e) {
      if (e instanceof ApiError)
        setOutcome(e.status === 429 ? "limit" : e.status === 0 ? "offline" : "rejected");
    }
  };

  const messages: Record<Exclude<Outcome, null>, [tone: "good" | "warn" | "bad", text: string]> = {
    ok: ["good", `✅ ${tr("challenge_ok")}`],
    missing: ["warn", tr("challenge_missing")],
    bad_value: ["warn", tr("report_bad_value")],
    limit: ["warn", tr("challenge_limit")],
    rejected: ["bad", tr("report_rejected")],
    offline: ["warn", `📡 ${tr("unreachable")}`],
  };
  const fields = [
    ["dispute-value", "📋", "challenge_value", value, setValue],
    ["dispute-name", "🏪", "challenge_name", name, setName],
    ["dispute-email", "✉️", "challenge_email", email, setEmail],
  ] as const;

  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        void send();
      }}
    >
      <p>{tr("challenge_intro")}</p>
      {fields.map(([id, icon, key, current, set]) => (
        <Field key={id} id={id} label={label(icon, tr(key))}>
          <input
            id={id}
            type={id === "dispute-email" ? "email" : "text"}
            value={current}
            onChange={(e) => set(e.target.value)}
            className={INPUT}
          />
        </Field>
      ))}
      <Field id="dispute-proof" label={label("📄", tr("challenge_proof"))}>
        <textarea
          id="dispute-proof"
          value={proof}
          rows={3}
          maxLength={2000}
          onChange={(e) => setProof(e.target.value)}
          className={INPUT}
        />
      </Field>
      <Button type="submit" className="w-full text-lg">
        {label("📨", tr("challenge_submit"))}
      </Button>
      {outcome && <Notice tone={messages[outcome][0]}>{messages[outcome][1]}</Notice>}
    </form>
  );
}
