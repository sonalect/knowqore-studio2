import { invoke } from "@tauri-apps/api/core";
import { dateLocale } from "./appearance";

/** The columns the backend sorts the Lessons by, as `lessons_page` names them. */
export type LessonSortKey =
  | "statement"
  | "kind"
  | "status"
  | "transfer"
  | "updatedAt";

export type SortDirection = "ascending" | "descending";

/** The values a Lesson may have, per field; an empty field does not narrow. */
export interface LessonFilters {
  kind: string[];
  status: string[];
  transfer: string[];
  polarity: string[];
}

/** Filters that narrow nothing. */
export const NO_FILTERS: LessonFilters = {
  kind: [],
  status: [],
  transfer: [],
  polarity: [],
};

/** One window of the sorted Lessons: `limit` is capped at 500 by the backend. */
export interface LessonsQuery {
  sortKey: LessonSortKey;
  direction: SortDirection;
  offset: number;
  limit: number;
  /** The key of a tree node: only the Lessons under it. `null` is every Lesson. */
  node: string | null;
  /** Values of one field are joined by "or", the fields by "and". */
  filters: LessonFilters;
}

/**
 * One Lesson as the table shows it. `statement` is at most 240 characters and
 * ends in `…` when cut; `updatedAt` is RFC 3339 UTC.
 */
export interface LessonRow extends Record<string, unknown> {
  id: string;
  statement: string;
  kind: string;
  status: string;
  transfer: string;
  polarity: string;
  updatedAt: string;
}

/** `rows` starts at `offset`; `total` counts every Lesson of the Vault. */
export interface LessonsPage {
  total: number;
  offset: number;
  rows: LessonRow[];
}

/**
 * Reads one window of the Lessons, sorted by the backend. The first call
 * after a workspace opens reads every Lesson once and may take a moment.
 * A failure rejects with the store's code and message as a string.
 */
export function lessonsPage(query: LessonsQuery): Promise<LessonsPage> {
  return invoke<LessonsPage>("lessons_page", { query });
}

/**
 * A node of the tree of the Vault's containers: a kind of source, or a
 * prefix of the container ids (host, owner, repository). `lessons` counts
 * the distinct Lessons seen under it; `key` is what `LessonsQuery.node` takes.
 */
export interface LessonTreeNode {
  key: string;
  label: string;
  lessons: number;
  children: LessonTreeNode[];
}

/** `containers` is the number of distinct containers the Lessons name. */
export interface LessonTree {
  containers: number;
  nodes: LessonTreeNode[];
}

/** Reads the tree of containers; a failure rejects with the store's error as a string. */
export function lessonTree(): Promise<LessonTree> {
  return invoke<LessonTree>("lesson_tree");
}

/** A date in the system's regional format and local time zone. */
function formatDate(
  updatedAt: string,
  options: (date: Date) => Intl.DateTimeFormatOptions,
): string {
  const date = new Date(updatedAt);
  if (Number.isNaN(date.getTime())) {
    return updatedAt;
  }
  return new Intl.DateTimeFormat(dateLocale(), options(date)).format(date);
}

/**
 * The time of a Lesson as "Nov 3, 10:08 PM" in the system's regional format,
 * with the year only when it is not the current one; the raw text when it is
 * not a date.
 */
export function formatUpdated(updatedAt: string): string {
  return formatDate(updatedAt, (date) => ({
    year:
      date.getFullYear() === new Date().getFullYear() ? undefined : "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }));
}

/** The full date and time with seconds and the time zone, for a tooltip. */
export function formatUpdatedFull(updatedAt: string): string {
  return formatDate(updatedAt, () => ({
    dateStyle: "long",
    timeStyle: "long",
  }));
}

/** How many Lessons have one value of a field. */
export interface FacetCount {
  value: string;
  lessons: number;
}

/**
 * The values of each field the Lessons under a node have, with how many
 * Lessons have each, in no particular order. A value that does not occur is
 * not listed.
 */
export type LessonFacets = Record<keyof LessonFilters, FacetCount[]>;

/** Counts the values under a node (every Lesson for `null`); a failure rejects with the store's error as a string. */
export function lessonFacets(node: string | null): Promise<LessonFacets> {
  return invoke<LessonFacets>("lesson_facets", { node });
}
