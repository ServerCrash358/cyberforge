// The "block 128" brand with the language switch and icon-only mode, and the screen menu.
import { LANGS, type Lang } from "../lib/i18n";
import { useSettings } from "../lib/settings";
import { PAGE_ICONS } from "../lib/strings";

export const SCREENS = ["check", "report", "map", "challenge", "licence", "how"] as const;
export type Screen = (typeof SCREENS)[number];

export function Header() {
  const { lang, setLang, iconsOnly, setIconsOnly, tr } = useSettings();
  return (
    <header className="sticky top-0 z-[1000] border-b border-line bg-white/95 backdrop-blur">
      <div className="mx-auto flex max-w-6xl items-center justify-between gap-3 px-4 py-3">
        <a href="#/check" className="text-2xl font-extrabold tracking-tight" aria-label="block 128, home">
          <span className="text-ink">block</span> <span className="text-brand">128</span>
        </a>
        <div className="flex items-center gap-2">
          <fieldset className="flex rounded-full border border-line p-0.5" aria-label="Language">
            {(Object.keys(LANGS) as Lang[]).map((code) => (
              <button
                key={code}
                type="button"
                aria-pressed={lang === code}
                onClick={() => setLang(code)}
                className={`min-h-9 rounded-full px-3 text-sm ${lang === code ? "bg-brand font-semibold" : ""}`}
              >
                {LANGS[code]}
              </button>
            ))}
          </fieldset>
          <button
            type="button"
            aria-pressed={iconsOnly}
            onClick={() => setIconsOnly(!iconsOnly)}
            title={tr("icons_only")}
            className={`min-h-9 rounded-full border px-3 text-sm ${
              iconsOnly ? "border-brand bg-brand-tint font-semibold" : "border-line"
            }`}
          >
            🖼️<span className="hidden sm:inline"> {tr("icons_only")}</span>
          </button>
        </div>
      </div>
    </header>
  );
}

/** The screen menu: a bar at the bottom of a phone, a row of tabs under the header on a wider screen. */
export function Nav({ screen }: { screen: Screen }) {
  const { iconsOnly, tr } = useSettings();
  return (
    <nav
      aria-label="Screens"
      className="fixed inset-x-0 bottom-0 z-[1000] border-t border-line bg-white sm:static sm:border-t-0 sm:border-b"
    >
      <ul className="mx-auto grid max-w-6xl grid-cols-6 sm:flex sm:gap-1 sm:px-4 sm:pb-2">
        {SCREENS.map((name) => {
          const here = name === screen;
          return (
            <li key={name} className="min-w-0">
              <a
                href={`#/${name}`}
                aria-current={here ? "page" : undefined}
                className={`flex min-h-14 w-full flex-col items-center justify-center gap-0.5 px-1 text-[11px] sm:min-h-10 sm:flex-row sm:gap-2 sm:rounded-full sm:px-3 sm:text-sm ${
                  here
                    ? "font-semibold text-brand-dark sm:bg-brand-tint sm:text-ink"
                    : "text-muted hover:text-ink"
                }`}
              >
                <span className="text-xl sm:text-base" aria-hidden="true">
                  {PAGE_ICONS[name]}
                </span>
                <span className={iconsOnly ? "sr-only" : "block max-w-full truncate"}>
                  {tr(`nav_${name}`)}
                </span>
              </a>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
