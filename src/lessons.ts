import { invoke } from "@tauri-apps/api/core";
import { dateLocale } from "./appearance";
import type {
  HistoryEventKind,
  HistoryOrigin,
  HistoryTone,
} from "./historyLooks";

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

/** A calendar day (`YYYY-MM-DD`, as the store's UTC time reads) in the system's regional format. */
export function formatDay(day: string): string {
  return formatDate(`${day}T00:00:00Z`, () => ({
    dateStyle: "medium",
    timeZone: "UTC",
  }));
}

/** The time of day of a timestamp in the system's regional format and local time zone. */
export function formatTime(at: string): string {
  return formatDate(at, () => ({ timeStyle: "short" }));
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

/** A source of the Vault: its kind and id. */
export interface LessonSource {
  kind: string;
  id: string;
}

/** Where an asset lives. */
export interface LessonAddress {
  source?: LessonSource;
  version?: string;
  locator?: string;
}

/** The asset block of a Lesson of kind `asset`. */
export interface LessonAsset {
  name?: string;
  address?: LessonAddress;
  usage?: string;
}

/** A place in a source that backs the Lesson, with the text found there. */
export interface LessonAnchor {
  locator?: string;
  text?: string;
}

/** The observation that amended the Lesson. */
export interface LessonAmendment {
  observation?: string;
  container?: string;
  at?: string;
}

/** Where the Lesson was seen. Dates are RFC 3339 UTC. */
export interface LessonSupport {
  firstContainer?: string;
  sourceContainers?: number;
  recordContainers?: number;
  containers?: number;
  presentContainers?: number;
  absentContainers?: number;
  origins?: string[];
  sourceAt?: string;
  contentAt?: string;
  latestAt?: string;
  choices?: number;
  amendment?: LessonAmendment;
}

/** The verdict of the curation of a Lesson. */
export interface LessonCuration {
  verdict?: string;
  other?: string;
  reason?: string;
  answer?: string;
}

/** What produced the record. */
export interface LessonProducer {
  name?: string;
  rulesVersion?: number;
}

/**
 * The whole record of a Lesson, as the contract's JSON spells it. The store
 * leaves out a field that is empty or unset, so every field but the ones the
 * contract requires may be absent.
 */
export interface Lesson {
  version?: number;
  id?: string;
  revision?: number;
  kind?: string;
  statement?: string;
  polarity?: string;
  conditions?: string[];
  transfer?: string;
  tasks?: string[];
  asset?: LessonAsset;
  anchors?: LessonAnchor[];
  support?: LessonSupport;
  status?: string;
  statusReasons?: string[];
  statusRule?: number;
  curation?: LessonCuration;
  cluster?: string;
  membersCount?: number;
  reducedContainers?: number;
  membersDigest?: string;
  producer?: LessonProducer;
  updatedAt?: string;
  quotes?: string[];
  searchable?: boolean;
}

/**
 * Reads the whole record of one Lesson and its YAML; `null` when the store
 * does not hold it. A failure rejects with the store's code and message as a string.
 */
export function findLesson(id: string): Promise<FoundLesson | null> {
  return invoke<FoundLesson | null>("find_lesson", { id });
}

/** The whole record of one Lesson, and the same record written as YAML. */
export interface FoundLesson {
  lesson: Lesson;
  yaml: string;
}

/** What a chip under an event says and how it reads. */
export interface HistoryChip {
  text: string;
  tone: HistoryTone;
}

/** One thing a write changed. */
export interface HistoryEvent {
  kind: HistoryEventKind;
  title: string;
  sub: string | null;
  chips: HistoryChip[];
  reason: string | null;
  /** The other Lesson a curator's verdict names. */
  other: { label: string; id: string } | null;
  /** The statement before a revision and after it. */
  statements: { old: string; new: string } | null;
}

/** Who wrote a version: the origin, the run's process when the note names one, and the note. */
export interface HistoryWriter {
  origin: HistoryOrigin;
  run: string | null;
  note: string;
}

/** One version of the record with what it changed. */
export interface HistoryChange {
  writeId: string;
  shortId: string;
  /** RFC 3339 UTC. */
  at: string;
  /** `YYYY-MM-DD`, UTC. */
  day: string;
  writer: HistoryWriter;
  quiet: boolean;
  events: HistoryEvent[];
}

/** The changes one run wrote on one day, as indices into `changes`. */
export interface HistoryRun {
  writer: HistoryWriter;
  title: string;
  day: string;
  changes: number[];
}

/** The history of a Lesson, newest first, computed by the backend. */
export interface LessonHistory {
  changes: HistoryChange[];
  /** The groups without the quiet changes. */
  runs: HistoryRun[];
  /** The groups with them. */
  runsWithQuiet: HistoryRun[];
  meaningful: number;
  quiet: number;
  since: string | null;
}

/**
 * Reads the history of one Lesson; empty for a Lesson the store never held.
 * A failure rejects with the store's code and message as a string.
 */
export function lessonHistory(id: string): Promise<LessonHistory> {
  return invoke<LessonHistory>("lesson_history", { id });
}

/** The unified YAML diff of a Lesson between a write and the readable version before it. */
export function lessonWriteDiff(id: string, writeId: string): Promise<string> {
  return invoke<string>("lesson_write_diff", { id, writeId });
}
