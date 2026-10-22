import { invoke } from "@tauri-apps/api/core";

/** The columns the backend sorts the Lessons by, as `lessons_page` names them. */
export type LessonSortKey =
  | "statement"
  | "kind"
  | "status"
  | "transfer"
  | "updatedAt";

export type SortDirection = "ascending" | "descending";

/** One window of the sorted Lessons: `limit` is capped at 500 by the backend. */
export interface LessonsQuery {
  sortKey: LessonSortKey;
  direction: SortDirection;
  offset: number;
  limit: number;
  /** The key of a tree node: only the Lessons under it. `null` is every Lesson. */
  node: string | null;
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

const UPDATED_FORMAT = new Intl.DateTimeFormat(undefined, {
  dateStyle: "short",
  timeStyle: "short",
});

/** The time of a Lesson in the viewer's locale; the raw text when it is not a date. */
export function formatUpdated(updatedAt: string): string {
  const date = new Date(updatedAt);
  return Number.isNaN(date.getTime()) ? updatedAt : UPDATED_FORMAT.format(date);
}
