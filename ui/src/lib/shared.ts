// The shared/data contract files, read at build time: never retyped here.
import channels from "../../../shared/data/channels.json";
import scamTypes from "../../../shared/data/scam_types.json";
import shortCodes from "../../../shared/data/short_codes.json";

export const CHANNEL_IDS: string[] = channels.items.map((c) => c.id);
export const SCAM_TYPE_IDS: string[] = scamTypes.items.map((s) => s.id);
export const SHORT_CODES: ReadonlySet<string> = new Set(shortCodes.items.map((c) => String(c.value)));
