// Check (home): paste a value or a whole message; one verdict card per identifier found.
import { useState } from "react";
import { Button, Notice } from "../components/ui";
import { VerdictCard } from "../components/VerdictCard";
import { ApiError, api, type Lookup } from "../lib/api";
import { extract, type Found } from "../lib/extract";
import { useSettings } from "../lib/settings";

type Result = Found & { lookup: Lookup };

export function Check() {
  const { tr, label, guard } = useSettings();
  const [text, setText] = useState("");
  const [results, setResults] = useState<Result[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [offline, setOffline] = useState(false);

  const check = async () => {
    setBusy(true);
    setOffline(false);
    try {
      const found = extract(text);
      const lookups = await Promise.all(found.map((f) => guard(() => api.lookup(f.type, f.value))));
      setResults(found.map((f, i) => ({ ...f, lookup: lookups[i] as Lookup })));
    } catch (e) {
      if (e instanceof ApiError) setOffline(true);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <label htmlFor="check-text" className="font-semibold">
        {tr("check_prompt")}
      </label>
      <textarea
        id="check-text"
        value={text}
        onChange={(e) => setText(e.target.value)}
        rows={4}
        className="w-full rounded-xl border border-line bg-white p-3 text-base outline-none focus:border-brand"
      />
      <Button onClick={check} disabled={busy || !text.trim()} className="w-full text-lg">
        {label("🔍", tr("check_button"))}
      </Button>
      {offline && <Notice tone="warn">📡 {tr("unreachable")}</Notice>}
      {results?.length === 0 && <Notice tone="warn">{tr("check_none_found")}</Notice>}
      {results?.map((r) => (
        <VerdictCard key={`${r.type}-${r.value}`} value={r.value} result={r.lookup} />
      ))}
    </div>
  );
}
