// What every screen shares: language, icon-only mode, the anonymous device id, the area list,
// whether the licence has expired, and the identifier the Check screen hands to Report.
import { createContext, type ReactNode, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { type Area, api, LicenceExpired } from "./api";
import { type Lang, type StringKey, t } from "./i18n";

/** Browser storage can be blocked or throw (private windows); the app works without it. */
function stored(store: "local" | "session", key: string): string | null {
  try {
    return (store === "local" ? localStorage : sessionStorage).getItem(key);
  } catch {
    return null;
  }
}

function store(where: "local" | "session", key: string, value: string): void {
  try {
    (where === "local" ? localStorage : sessionStorage).setItem(key, value);
  } catch {
    // Not saved; the choice still holds until the page is closed.
  }
}

/** A random id for this browser session, sent as reporter_id. Linked to nothing (DECISIONS D57). */
function sessionDeviceId(): string {
  const existing = stored("session", "device_id");
  if (existing) return existing;
  const bytes = crypto.getRandomValues(new Uint8Array(8));
  const id = `d_${Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("")}`;
  store("session", "device_id", id);
  return id;
}

interface Settings {
  lang: Lang;
  setLang: (lang: Lang) => void;
  iconsOnly: boolean;
  setIconsOnly: (on: boolean) => void;
  tr: (key: StringKey, values?: Record<string, string | number>) => string;
  /** Icon and words, or the icon alone in icon-only mode. */
  label: (icon: string, text: string) => string;
  deviceId: string;
  areas: Area[];
  myArea: string | null;
  setMyArea: (pincode: string) => void;
  expired: boolean;
  /** Runs an API call; a locked licence switches the whole app to the expired screen. */
  guard: <T>(call: () => Promise<T>) => Promise<T>;
  setExpired: (expired: boolean) => void;
  prefill: string;
  setPrefill: (value: string) => void;
}

const SettingsContext = createContext<Settings | null>(null);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(stored("local", "lang") === "hi" ? "hi" : "en");
  const [iconsOnly, setIconsState] = useState(stored("local", "icons_only") === "1");
  const [myArea, setMyAreaState] = useState(stored("local", "my_area"));
  const [areas, setAreas] = useState<Area[]>([]);
  const [expired, setExpired] = useState(false);
  const [prefill, setPrefill] = useState("");
  const [deviceId] = useState(sessionDeviceId);

  const guard = useCallback(async <T,>(call: () => Promise<T>): Promise<T> => {
    try {
      const result = await call();
      setExpired(false);
      return result;
    } catch (e) {
      if (e instanceof LicenceExpired) setExpired(true);
      throw e;
    }
  }, []);

  // The area list loads once the licence is known to be valid, never as a refused request.
  const haveAreas = areas.length > 0;
  useEffect(() => {
    if (expired || haveAreas) return;
    api
      .licence()
      .then((licence) => (licence.valid ? guard(api.areas).then(setAreas) : setExpired(true)))
      .catch(() => undefined);
  }, [expired, haveAreas, guard]);

  const value = useMemo<Settings>(
    () => ({
      lang,
      setLang: (next) => {
        setLangState(next);
        store("local", "lang", next);
      },
      iconsOnly,
      setIconsOnly: (on) => {
        setIconsState(on);
        store("local", "icons_only", on ? "1" : "0");
      },
      tr: (key, values) => t(key, lang, values),
      label: (icon, text) => (iconsOnly ? icon : `${icon} ${text}`),
      deviceId,
      areas,
      myArea,
      setMyArea: (pincode) => {
        setMyAreaState(pincode);
        store("local", "my_area", pincode);
      },
      expired,
      guard,
      setExpired,
      prefill,
      setPrefill,
    }),
    [lang, iconsOnly, deviceId, areas, myArea, expired, guard, prefill],
  );
  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);
  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useSettings(): Settings {
  const settings = useContext(SettingsContext);
  if (settings === null) throw new Error("useSettings outside SettingsProvider");
  return settings;
}

/** "12 Nov 2026, 21:10" in India time, from an ISO time with an offset. */
export function shortDate(iso: string | null): string {
  if (!iso) return "-";
  return new Intl.DateTimeFormat("en-IN", {
    timeZone: "Asia/Kolkata",
    day: "2-digit",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(new Date(iso));
}

export function clockTime(iso: string): string {
  return new Intl.DateTimeFormat("en-IN", {
    timeZone: "Asia/Kolkata",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  }).format(new Date(iso));
}
