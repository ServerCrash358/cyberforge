// One identifier's lookup result: verdict, masked value, how many people, recent reports, actions.
import type { Lookup } from "../lib/api";
import { channelLabel, scamLabel } from "../lib/i18n";
import { shortDate, useSettings } from "../lib/settings";
import { VERDICT_ICONS } from "../lib/strings";
import { Button, Card, Notice } from "./ui";

const VERDICT_STYLE: Record<Lookup["verdict"], string> = {
  KNOWN_SCAM: "border-l-crit",
  SUSPICIOUS: "border-l-high",
  NO_REPORTS: "border-l-line",
  VERIFIED_BUSINESS: "border-l-brand",
};

export function VerdictCard({ value, result }: { value: string; result: Lookup }) {
  const { tr, lang, label, areas, setPrefill } = useSettings();
  const { verdict } = result;
  const people = result.distinct_reporters;
  const verified = verdict === "VERIFIED_BUSINESS";
  const place = (pincode: string) => areas.find((a) => a.pincode === pincode)?.locality ?? pincode;

  return (
    <Card className={`border-l-8 ${VERDICT_STYLE[verdict]}`}>
      <h2 className="text-2xl font-bold">
        <span aria-hidden="true">{VERDICT_ICONS[verdict]}</span> {tr(`verdict_${verdict}`)}
      </h2>
      <p className="mt-1 break-all font-mono text-lg font-semibold">{result.display_value}</p>
      <div className="mt-3 flex flex-col gap-2">
        {verified && <Notice tone="good">{tr("verified_note")}</Notice>}
        {!verified && people === 0 && <Notice>{tr("no_reports_note")}</Notice>}
        {!verified && people > 0 && (
          <>
            <p className="font-semibold">
              {people === 1 ? tr("reported_by_one") : tr("reported_by", { n: people })}
            </p>
            <p className="text-sm text-muted">
              {tr("seen_between", { first: shortDate(result.first_seen), last: shortDate(result.last_seen) })}
            </p>
          </>
        )}
        {result.challenged && <Notice tone="warn">🏪 {tr("challenged_badge")}</Notice>}
      </div>
      {!verified && result.evidence.length > 0 && (
        <div className="mt-4">
          <h3 className="font-semibold">{tr("recent_reports")}</h3>
          <ul className="mt-1 flex flex-col gap-1 text-sm text-muted">
            {result.evidence.map((item) => (
              <li key={`${item.occurred_at}-${item.pincode}-${item.scam_type}`}>
                {[
                  shortDate(item.occurred_at),
                  channelLabel(item.channel, lang),
                  scamLabel(item.scam_type, lang),
                  place(item.pincode),
                ].join(" · ")}
              </li>
            ))}
          </ul>
        </div>
      )}
      <div className="mt-4 grid gap-2 sm:grid-cols-3">
        {!verified && (
          <Button
            onClick={() => {
              setPrefill(value);
              location.hash = "#/report";
            }}
          >
            {label("📣", tr("got_this_too"))}
          </Button>
        )}
        <a
          href="tel:1930"
          className="flex min-h-11 items-center justify-center rounded-xl border border-line px-3 font-semibold hover:border-brand"
        >
          {label("📞", tr("call_1930"))}
        </a>
        <a
          href="https://cybercrime.gov.in"
          target="_blank"
          rel="noreferrer"
          className="flex min-h-11 items-center justify-center rounded-xl border border-line px-3 text-center font-semibold hover:border-brand"
        >
          {label("🛡️", tr("report_portal"))}
        </a>
      </div>
    </Card>
  );
}
