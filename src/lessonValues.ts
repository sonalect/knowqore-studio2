import data from "./data/lesson-values.json";

export type BadgeVariant = "neutral" | "blue" | "green" | "red";
export type DotVariant = "accent" | "success" | "warning" | "error" | "neutral";

export interface StatusLook {
  label: string;
  badge: BadgeVariant;
}

export interface DotLook {
  label: string;
  dot?: DotVariant;
}

interface LessonValues {
  status: Record<string, StatusLook>;
  kind: Record<string, DotLook>;
  transfer: Record<string, DotLook>;
  polarity: Record<string, DotLook>;
}

// Every value of the contract; `data/lesson-values.json` needs an entry for each.
const KNOWN = {
  status: ["candidate", "active", "retired"],
  kind: [
    "asset",
    "practice",
    "decision",
    "pitfall",
    "project_fact",
    "preference",
  ],
  transfer: ["global", "local"],
  polarity: ["prefer", "avoid"],
} as const;

/** The fields the Lessons can be filtered by. */
export type FilterField = keyof typeof KNOWN;

/** The values of a field in the contract's order. */
export function knownValues(field: FilterField): readonly string[] {
  return KNOWN[field];
}

/** The label of a value of a field; the value itself when the table does not know it. */
export function valueLabel(field: FilterField, value: string): string {
  return values[field][value]?.label ?? value;
}

const values: LessonValues = data as LessonValues;

// The repository has no frontend test runner, so the table is checked when
// the module loads; a missing entry is logged and that value shows as text.
for (const [group, known] of Object.entries(KNOWN)) {
  const table: Record<string, unknown> = values[group as keyof LessonValues];
  const missing = known.filter((value) => !(value in table));
  if (missing.length > 0) {
    console.error(
      `lesson-values.json: no ${group} entry for ${missing.join(", ")}`,
    );
  }
}

/** The look of a status, or `undefined` for a value the table does not know. */
export function statusLook(status: string): StatusLook | undefined {
  return values.status[status];
}

/** The look of a kind, or `undefined` for a value the table does not know. */
export function kindLook(kind: string): DotLook | undefined {
  return values.kind[kind];
}

/** The look of a polarity, or `undefined` for a value the table does not know. */
export function polarityLook(polarity: string): DotLook | undefined {
  return values.polarity[polarity];
}
