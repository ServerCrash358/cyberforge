// The app: header, the screen chosen by the address (#/check, #/report, ...), the live panel
// beside it, and the menu. Hash addresses never clash with the API's own routes like /map.
import { type ReactNode, useEffect, useState } from "react";
import { Header, Nav, SCREENS, type Screen } from "./components/Header";
import { LivePanel } from "./components/LivePanel";
import { Notice } from "./components/ui";
import { useSettings } from "./lib/settings";
import { PAGE_ICONS } from "./lib/strings";
import { AreaMap } from "./screens/AreaMap";
import { Check } from "./screens/Check";
import { Dispute } from "./screens/Dispute";
import { How } from "./screens/How";
import { Licence } from "./screens/Licence";
import { Report } from "./screens/Report";

const VIEWS: Record<Screen, () => ReactNode> = {
  check: Check,
  report: Report,
  map: AreaMap,
  challenge: Dispute,
  licence: Licence,
  how: How,
};

export function screenFromHash(hash: string): Screen {
  const name = hash.replace(/^#\/?/, "");
  return (SCREENS as readonly string[]).includes(name) ? (name as Screen) : "check";
}

export function App() {
  const { tr, expired } = useSettings();
  const [screen, setScreen] = useState(() => screenFromHash(location.hash));

  useEffect(() => {
    const follow = () => setScreen(screenFromHash(location.hash));
    addEventListener("hashchange", follow);
    return () => removeEventListener("hashchange", follow);
  }, []);
  useEffect(() => {
    document.title = `block 128 · ${tr(`nav_${screen}`)}`;
    scrollTo(0, 0);
  }, [screen, tr]);

  const View = VIEWS[screen];
  return (
    <div className="min-h-screen pb-20 sm:pb-8">
      <Header />
      <Nav screen={screen} />
      <main className="mx-auto grid max-w-6xl gap-8 px-4 py-6 lg:grid-cols-[minmax(0,1fr)_22rem]">
        <div className="flex min-w-0 flex-col gap-4">
          <h1 className="text-2xl font-bold sm:text-3xl">
            <span aria-hidden="true">{PAGE_ICONS[screen]}</span> {tr(`nav_${screen}`)}
          </h1>
          {expired && screen !== "licence" ? <Notice tone="bad">🔒 {tr("expired")}</Notice> : <View />}
        </div>
        <div className="lg:sticky lg:top-32 lg:self-start">
          <LivePanel />
        </div>
      </main>
    </div>
  );
}
