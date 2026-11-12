// Licence: who the app is licensed to and until when; upload a licence file for the demo.
// GET and POST /license work even when the licence has expired, so this screen always works.
import { useCallback, useEffect, useState } from "react";
import { Card, Field, Notice } from "../components/ui";
import { ApiError, api, type Licence as LicenceState } from "../lib/api";
import { shortDate, useSettings } from "../lib/settings";

export function Licence() {
  const { tr, label, setExpired } = useSettings();
  const [current, setCurrent] = useState<LicenceState | null>(null);
  const [message, setMessage] = useState<[tone: "good" | "bad", text: string] | null>(null);

  const refresh = useCallback(() => {
    api
      .licence()
      .then((l) => {
        setCurrent(l);
        setExpired(!l.valid);
      })
      .catch(() => setMessage(["bad", `📡 ${tr("unreachable")}`]));
  }, [setExpired, tr]);

  useEffect(refresh, [refresh]);

  const upload = async (file: File) => {
    let doc: unknown;
    try {
      doc = JSON.parse(await file.text());
      if (typeof doc !== "object" || doc === null || Array.isArray(doc)) throw new Error("not an object");
    } catch {
      return setMessage(["bad", tr("licence_unreadable")]);
    }
    try {
      await api.uploadLicence(doc);
      setMessage(["good", `✅ ${tr("licence_ok")}`]);
      refresh();
    } catch (e) {
      const why = e instanceof ApiError ? e.message : String(e);
      setMessage(["bad", tr("licence_bad", { why })]);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <Card>
        {current === null || current.licensee === null ? (
          <h2 className="text-xl font-bold">🔒 {tr("licence_none")}</h2>
        ) : (
          <>
            <h2 className="text-xl font-bold">
              {current.valid ? "✅" : "🔒"} {tr(current.valid ? "licence_valid" : "licence_invalid")}
            </h2>
            <dl className="mt-2 grid grid-cols-[auto_1fr] gap-x-3 gap-y-1">
              <dt className="font-semibold">{tr("licence_to")}</dt>
              <dd>{current.licensee}</dd>
              <dt className="font-semibold">{tr("licence_until")}</dt>
              <dd>{shortDate(current.expires_at)}</dd>
            </dl>
            {!current.valid && (
              <div className="mt-3">
                <Notice tone="bad">{tr("expired")}</Notice>
              </div>
            )}
          </>
        )}
      </Card>
      <Field id="licence-file" label={label("📄", tr("licence_upload"))}>
        <input
          id="licence-file"
          type="file"
          accept=".json,application/json"
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) void upload(file);
            e.target.value = "";
          }}
          className="block w-full rounded-xl border border-dashed border-brand bg-brand-tint p-4 text-sm file:mr-3 file:rounded-lg file:border-0 file:bg-brand file:px-3 file:py-2 file:font-semibold"
        />
      </Field>
      {message && <Notice tone={message[0]}>{message[1]}</Notice>}
    </div>
  );
}
