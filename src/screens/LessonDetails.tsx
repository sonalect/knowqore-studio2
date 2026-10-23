import { Badge } from "@astryxdesign/core/Badge";
import { Card } from "@astryxdesign/core/Card";
import { Heading } from "@astryxdesign/core/Heading";
import { Icon } from "@astryxdesign/core/Icon";
import { IconButton } from "@astryxdesign/core/IconButton";
import {
  MetadataList,
  MetadataListItem,
} from "@astryxdesign/core/MetadataList";
import { HStack, VStack } from "@astryxdesign/core/Stack";
import { StatusDot } from "@astryxdesign/core/StatusDot";
import { Tab, TabList } from "@astryxdesign/core/TabList";
import { Text } from "@astryxdesign/core/Text";
import { X } from "lucide-react";
import { type ReactNode, useEffect, useRef, useState } from "react";
// Swizzled: wraps a long line under its own indent (see the `Studio:` notes).
import { CodeBlock } from "../components/astryx/CodeBlock";
import {
  type FoundLesson,
  findLesson,
  formatUpdatedFull,
  type Lesson,
  type LessonRow,
  lessonHistory,
} from "../lessons";
import {
  type DotLook,
  kindLook,
  polarityLook,
  statusLook,
} from "../lessonValues";
import { type HistoryReading, LessonHistoryView } from "./LessonHistory";

/** A value that may be long and has no spaces (ids, digests, paths): it breaks anywhere. */
function Mono({ children }: { children: ReactNode }) {
  return (
    <Text type="code" display="block" wordBreak="break-all">
      {children}
    </Text>
  );
}

/** Running text, in full. */
function Prose({ children }: { children: ReactNode }) {
  return (
    <Text display="block" wordBreak="break-word">
      {children}
    </Text>
  );
}

function None({ children = "None" }: { children?: string }) {
  return <Text color="secondary">{children}</Text>;
}

function when(value: string | undefined): ReactNode {
  return value === undefined ? undefined : (
    <Prose>{formatUpdatedFull(value)}</Prose>
  );
}

function text(value: string | undefined): ReactNode {
  return value === undefined || value === "" ? undefined : <Mono>{value}</Mono>;
}

/** One label and its value; `undefined` leaves the line out. */
interface Entry {
  label: string;
  value: ReactNode;
}

function row(label: string, value: ReactNode): Entry {
  return { label, value };
}

/** The items of `entries` that have a value, as a metadata list. */
function Facts({ entries }: { entries: Entry[] }) {
  const shown = entries.filter((entry) => entry.value !== undefined);
  if (shown.length === 0) return <None />;
  return (
    <MetadataList label={{ position: "top" }}>
      {shown.map(({ label, value }) => (
        <MetadataListItem key={label} label={label}>
          {value}
        </MetadataListItem>
      ))}
    </MetadataList>
  );
}

function Dot({ look }: { look: DotLook | undefined }) {
  if (look === undefined) return undefined;
  return (
    <HStack gap={1.5} vAlign="center">
      {look.dot ? <StatusDot variant={look.dot} label={look.label} /> : null}
      <Text>{look.label}</Text>
    </HStack>
  );
}

/**
 * The statement in full and the fields the table shows; the YAML below the
 * card holds the rest of the record.
 */
function Record({ lesson }: { lesson: Lesson }) {
  const status = lesson.status ?? "";
  const statusView = statusLook(status);
  const kind = lesson.kind ?? "";
  const polarity = lesson.polarity ?? "";
  return (
    <VStack gap={4}>
      {/* `spans` colors the text in the first paint. The default picks the CSS
          Highlight API where the webview has it (WebView2 on Windows), which
          paints plain text first and colors it a moment later. */}
      <CodeBlock
        code={lesson.statement ?? ""}
        language="markdown"
        highlightMode="spans"
        isWrapped
      />

      <Facts
        entries={[
          row("Kind", Dot({ look: kindLook(kind) }) ?? text(kind)),
          row(
            "Status",
            status === "" ? undefined : (
              <HStack>
                <Badge
                  label={statusView?.label ?? status}
                  variant={statusView?.badge}
                />
              </HStack>
            ),
          ),
          row(
            "Polarity",
            Dot({ look: polarityLook(polarity) }) ?? text(polarity),
          ),
          row("Transfer", text(lesson.transfer)),
          row("Updated", when(lesson.updatedAt)),
          row("Id", text(lesson.id)),
        ]}
      />
    </VStack>
  );
}

/** The views of the details: the fields, the history, or the whole record as YAML. */
type DetailsTab = "overview" | "history" | "yaml";

/** What is read of the selected Lesson: the record, its absence, or why it failed. */
type Reading =
  | { id: string; record: FoundLesson | null }
  | { id: string; error: string };

/** The record and the history of one Lesson, read together and shown together. */
interface Shown {
  id: string;
  reading: Reading;
  history: HistoryReading;
}

/** How long a read may take before the card says it is reading. */
const SLOW_READ_MS = 300;

/**
 * The details of a Lesson: its statement and the fields of the table, its
 * history, and its whole record as YAML, read from the store when the row is
 * selected.
 *
 * The record and the history arrive together and replace the card in one
 * step: until then it keeps showing the Lesson before, so it does not flash
 * through an interim state. Only a read slower than `SLOW_READ_MS` says so.
 */
export function LessonDetails({
  row,
  version,
  onClose,
}: {
  row: LessonRow;
  /** Moves when the data was read again: the same Lesson is read again and keeps its tab. */
  version: number;
  onClose: () => void;
}) {
  const [shown, setShown] = useState<Shown | null>(null);
  const [isSlow, setIsSlow] = useState(false);
  // Every Lesson opens on Overview; the tab chosen holds while it is shown.
  const [tab, setTab] = useState<DetailsTab>("overview");
  // The Lesson the tab belongs to: a read of the same Lesson again keeps it.
  const tabOf = useRef<string | null>(null);
  const chooseTab = (value: string) =>
    setTab(
      value === "yaml" ? "yaml" : value === "history" ? "history" : "overview",
    );

  useEffect(() => {
    const id = row.id;
    void version;
    let current = true;
    setIsSlow(false);
    const slow = setTimeout(() => {
      if (current) setIsSlow(true);
    }, SLOW_READ_MS);
    const reading = findLesson(id).then(
      (record): Reading => ({ id, record }),
      (reason: unknown): Reading => ({ id, error: String(reason) }),
    );
    const history = lessonHistory(id).then(
      (value): HistoryReading => ({ id, history: value }),
      (reason: unknown): HistoryReading => ({ id, error: String(reason) }),
    );
    void Promise.all([reading, history]).then(([record, past]) => {
      if (!current) return;
      clearTimeout(slow);
      setIsSlow(false);
      // With the new Lesson, not before: the Lesson still shown keeps its
      // tab, and so does a read of the same Lesson again.
      if (tabOf.current !== id) {
        tabOf.current = id;
        setTab("overview");
      }
      setShown({ id, reading: record, history: past });
    });
    return () => {
      current = false;
      clearTimeout(slow);
    };
  }, [row.id, version]);

  const isCurrent = shown !== null && shown.id === row.id;
  let body: ReactNode;
  if (shown === null || (!isCurrent && isSlow)) {
    body = isSlow ? (
      <Text color="secondary">Reading the Lesson…</Text>
    ) : undefined;
  } else if ("error" in shown.reading) {
    body = <Text color="secondary">{shown.reading.error}</Text>;
  } else if (shown.reading.record === null) {
    body = <Text color="secondary">This Lesson is no longer stored.</Text>;
  } else {
    const found = shown.reading.record;
    const history = shown.history;
    body = (
      <VStack gap={4}>
        <TabList value={tab} onChange={chooseTab} size="sm" hasDivider>
          <Tab value="overview" label="Overview" />
          <Tab
            value="history"
            label="History"
            endContent={
              "history" in history ? (
                <Badge label={String(history.history.meaningful)} />
              ) : undefined
            }
          />
          <Tab value="yaml" label="YAML" />
        </TabList>
        {tab === "overview" ? (
          <Record lesson={found.lesson} />
        ) : tab === "history" ? (
          <LessonHistoryView lessonId={shown.id} reading={history} />
        ) : (
          // The whole record as the contract gives it.
          <CodeBlock
            code={found.yaml}
            language="yaml"
            highlightMode="spans"
            isWrapped
          />
        )}
      </VStack>
    );
  }

  return (
    <Card padding={4}>
      <VStack gap={4}>
        <HStack gap={2} vAlign="center" hAlign="between">
          <Heading level={3}>Lesson</Heading>
          <IconButton
            label="Close details"
            size="sm"
            variant="ghost"
            icon={<Icon icon={X} size="sm" />}
            onClick={onClose}
          />
        </HStack>
        {body}
      </VStack>
    </Card>
  );
}
