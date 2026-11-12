// Map: area points sized by count and coloured by severity; days and radius; the area's alerts.
// Points are the areas' Appendix B points from the API, never anyone's own location.
import L from "leaflet";
import { useEffect, useRef, useState } from "react";
import { Card, Field, INPUT, Notice } from "../components/ui";
import { type Alert, ApiError, api, type Cluster } from "../lib/api";
import { scamLabel } from "../lib/i18n";
import { useSettings } from "../lib/settings";

const SEVERITY: Record<Cluster["severity"], [color: string, icon: string]> = {
  CRIT: ["#d1242f", "🔴"],
  HIGH: ["#e8710a", "🟠"],
  MED: ["#bf8700", "🟡"],
};

function ClusterMap({ clusters, centre }: { clusters: Cluster[]; centre: [number, number] }) {
  const box = useRef<HTMLDivElement>(null);
  const map = useRef<L.Map | null>(null);
  const layer = useRef<L.LayerGroup | null>(null);

  useEffect(() => {
    if (!box.current || map.current) return;
    map.current = L.map(box.current, { scrollWheelZoom: false }).setView(centre, 12);
    L.tileLayer("https://tile.openstreetmap.org/{z}/{x}/{y}.png", {
      maxZoom: 18,
      attribution: "&copy; OpenStreetMap contributors",
    }).addTo(map.current);
    layer.current = L.layerGroup().addTo(map.current);
    return () => {
      map.current?.remove();
      map.current = null;
    };
  }, [centre]);

  useEffect(() => {
    const group = layer.current;
    if (!group || !map.current) return;
    group.clearLayers();
    for (const c of clusters) {
      const [color] = SEVERITY[c.severity];
      L.circleMarker([c.lat, c.lng], {
        radius: 8 + 2 * Math.min(c.count, 20),
        color,
        fillColor: color,
        fillOpacity: 0.45,
        weight: 2,
      })
        .bindTooltip(`${c.locality}: ${c.count}`)
        .addTo(group);
    }
    map.current.setView(centre, map.current.getZoom());
  }, [clusters, centre]);

  return <div ref={box} className="h-72 w-full overflow-hidden rounded-2xl border border-line sm:h-96" />;
}

export function AreaMap() {
  const { tr, lang, label, guard, areas, myArea } = useSettings();
  const [pincode, setPincode] = useState(myArea ?? "");
  const [days, setDays] = useState(7);
  const [radius, setRadius] = useState(2);
  const [clusters, setClusters] = useState<Cluster[] | null>(null);
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [offline, setOffline] = useState(false);
  const chosen = pincode || areas[0]?.pincode || "";
  const area = areas.find((a) => a.pincode === chosen);

  useEffect(() => {
    if (!chosen) return;
    let alive = true;
    Promise.all([guard(() => api.map(chosen, days, radius)), guard(() => api.alerts(chosen))])
      .then(([found, live]) => {
        if (!alive) return;
        setClusters(found);
        setAlerts(live);
        setOffline(false);
      })
      .catch((e: unknown) => {
        if (alive && e instanceof ApiError) setOffline(true);
      });
    return () => {
      alive = false;
    };
  }, [chosen, days, radius, guard]);

  return (
    <div className="flex flex-col gap-4">
      <Field id="map-area" label={label("📍", tr("map_area"))}>
        <select id="map-area" value={chosen} onChange={(e) => setPincode(e.target.value)} className={INPUT}>
          {areas.map((a) => (
            <option key={a.pincode} value={a.pincode}>
              {a.locality}, {a.city}
            </option>
          ))}
        </select>
      </Field>
      <div className="grid gap-4 sm:grid-cols-2">
        <Field id="map-days" label={`${label("📅", tr("map_days"))}: ${days}`}>
          <input
            id="map-days"
            type="range"
            min={1}
            max={30}
            value={days}
            onChange={(e) => setDays(Number(e.target.value))}
            className="accent-brand"
          />
        </Field>
        <Field id="map-radius" label={`${label("📏", tr("map_radius"))}: ${radius}`}>
          <input
            id="map-radius"
            type="range"
            min={1}
            max={50}
            value={radius}
            onChange={(e) => setRadius(Number(e.target.value))}
            className="accent-brand"
          />
        </Field>
      </div>
      {offline && <Notice tone="warn">📡 {tr("unreachable")}</Notice>}
      {area && <ClusterMap clusters={clusters ?? []} centre={[area.lat, area.lng]} />}
      {clusters?.length === 0 && <Notice>{tr("map_none")}</Notice>}
      {clusters && clusters.length > 0 && (
        <ul className="flex flex-col gap-1">
          {clusters.map((c) => (
            <li key={c.pincode}>
              {SEVERITY[c.severity][1]} <strong>{c.locality}</strong> · {tr("map_count", { n: c.count })} ·{" "}
              {scamLabel(c.top_scam_type, lang)}
            </li>
          ))}
        </ul>
      )}
      <Card>
        <h2 className="mb-2 text-lg font-bold">🔔 {tr("alerts_title")}</h2>
        {alerts.length === 0 && <p className="text-sm text-muted">{tr("no_alerts")}</p>}
        <div className="flex flex-col gap-2">
          {alerts.map((a) => (
            <Notice key={a.type === "SPIKE" ? "spike" : a.scam_type} tone="warn">
              {a.type === "SPIKE"
                ? `📈 ${tr("alert_spike", { count: a.count, base: a.baseline_per_day })}`
                : `🆕 ${tr("alert_new", { kind: scamLabel(a.scam_type, lang) })}`}
            </Notice>
          ))}
        </div>
      </Card>
    </div>
  );
}
