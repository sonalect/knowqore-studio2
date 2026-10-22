import data from "./data/history-looks.json";
import type { BadgeVariant } from "./lessonValues";

/** The kinds of event the backend reports (`EventKind`, camelCase). */
export const EVENT_KINDS = [
  "created",
  "revision",
  "status",
  "curation",
  "containers",
  "amendment",
  "evidence",
  "choice",
  "removed",
  "unreadable",
  "search",
  "tasks",
  "anchors",
  "quiet",
] as const;
export type HistoryEventKind = (typeof EVENT_KINDS)[number];

/** What wrote a version (`Origin`). */
export const ORIGINS = ["learn", "ingest", "other"] as const;
export type HistoryOrigin = (typeof ORIGINS)[number];

/** How a chip reads (`Tone`). */
export const TONES = ["added", "removed", "neutral"] as const;
export type HistoryTone = (typeof TONES)[number];

/** A label and a Badge variant. */
export interface HistoryLook {
  label: string;
  badge: BadgeVariantOrColor;
}

/** The Badge variants the data file uses. */
export type BadgeVariantOrColor =
  | BadgeVariant
  | "yellow"
  | "pink"
  | "cyan"
  | "purple";

interface HistoryLooks {
  event: Record<HistoryEventKind, HistoryLook>;
  origin: Record<HistoryOrigin, HistoryLook>;
  tone: Record<HistoryTone, { badge: BadgeVariantOrColor }>;
}

const looks: HistoryLooks = data as HistoryLooks;

// The repository has no frontend test runner, so the table is checked when
// the module loads; a missing entry is logged.
for (const [group, known] of [
  ["event", EVENT_KINDS],
  ["origin", ORIGINS],
  ["tone", TONES],
] as const) {
  const table: Record<string, unknown> = looks[group];
  const missing = known.filter((name) => !(name in table));
  if (missing.length > 0) {
    console.error(
      `history-looks.json: no ${group} entry for ${missing.join(", ")}`,
    );
  }
}

/** The look of an event kind. */
export function eventLook(kind: HistoryEventKind): HistoryLook {
  return looks.event[kind] ?? { label: kind, badge: "neutral" };
}

/** The look of an origin. */
export function originLook(origin: HistoryOrigin): HistoryLook {
  return looks.origin[origin] ?? { label: origin, badge: "neutral" };
}

/** The Badge variant of a chip. */
export function toneBadge(tone: HistoryTone): BadgeVariantOrColor {
  return looks.tone[tone]?.badge ?? "neutral";
}
