import { Badge } from "@astryxdesign/core/Badge";
import { Button } from "@astryxdesign/core/Button";
import { Card } from "@astryxdesign/core/Card";
import { Dialog, DialogHeader } from "@astryxdesign/core/Dialog";
import { HStack, VStack } from "@astryxdesign/core/Stack";
import { Switch } from "@astryxdesign/core/Switch";
import { Text } from "@astryxdesign/core/Text";
import { type ReactNode, useEffect, useState } from "react";
// Swizzled: wraps a long line under its own indent (see the `Studio:` notes).
import { CodeBlock } from "../components/astryx/CodeBlock";
import { eventLook, originLook, toneBadge } from "../historyLooks";
import {
  formatDay,
  formatTime,
  formatUpdatedFull,
  type LessonHistory as History,
  type HistoryChange,
  type HistoryEvent,
  type HistoryRun,
  lessonWriteDiff,
} from "../lessons";

/** What is read of the history of one Lesson: the history, or why it failed. */
export type HistoryReading =
  | { id: string; history: History }
  | { id: string; error: string };

/** How a chip under an event is colored: added green, taken away red, a plain fact neutral. */
function Chips({ event }: { event: HistoryEvent }) {
  if (event.chips.length === 0) return null;
  return (
    <HStack gap={1.5} wrap="wrap">
      {event.chips.map((chip) => (
        <Badge
          key={`${chip.tone}:${chip.text}`}
          label={chip.text}
          variant={toneBadge(chip.tone)}
        />
      ))}
    </HStack>
  );
}

/** The statement before a revision and after it: the old one in red, the new one in green. */
function Statements({
  statements,
}: {
  statements: { old: string; new: string };
}) {
  return (
    <VStack gap={1}>
      <Card variant="red" padding={2}>
        <Text display="block" wordBreak="break-word">
          {`− ${statements.old}`}
        </Text>
      </Card>
      <Card variant="green" padding={2}>
        <Text display="block" wordBreak="break-word">
          {`+ ${statements.new}`}
        </Text>
      </Card>
    </VStack>
  );
}

/** One event: its kind as a colored badge, what happened, and what it carries. */
function Event({ event }: { event: HistoryEvent }) {
  const look = eventLook(event.kind);
  const quiet = event.kind === "quiet";
  return (
    <VStack gap={1}>
      <HStack gap={2} vAlign="center" wrap="wrap">
        <Badge label={look.label} variant={look.badge} />
        <Text
          weight={quiet ? "normal" : "medium"}
          color={quiet ? "secondary" : "primary"}
          wordBreak="break-word"
        >
          {event.title}
        </Text>
      </HStack>
      {event.sub === null ? null : (
        <Text type="supporting" display="block" wordBreak="break-word">
          {event.sub}
        </Text>
      )}
      <Chips event={event} />
      {event.reason === null ? null : (
        <Text type="supporting" display="block" wordBreak="break-word">
          {event.reason}
        </Text>
      )}
      {event.other === null ? null : (
        <HStack gap={1.5} vAlign="center" wrap="wrap">
          <Text type="supporting">{event.other.label}</Text>
          <Text type="code" wordBreak="break-all">
            {event.other.id}
          </Text>
        </HStack>
      )}
      {event.statements === null ? null : (
        <Statements statements={event.statements} />
      )}
    </VStack>
  );
}

/** One write: its time and id (a click opens its diff) above its events. */
function Write({
  change,
  onOpen,
}: {
  change: HistoryChange;
  onOpen: (change: HistoryChange) => void;
}) {
  return (
    <VStack gap={2}>
      <HStack gap={2} vAlign="center" hAlign="between">
        <Text type="supporting">{formatTime(change.at)}</Text>
        <Button
          label={change.shortId}
          size="sm"
          variant="ghost"
          tooltip="Show the change in this write"
          onClick={() => onOpen(change)}
        />
      </HStack>
      {change.events.map((event) => (
        // A write has at most one event of each kind.
        <Event key={event.kind} event={event} />
      ))}
    </VStack>
  );
}

/** A group: the day, the origin of the run, the run itself, and its writes. */
function Run({
  run,
  changes,
  onOpen,
}: {
  run: HistoryRun;
  changes: HistoryChange[];
  onOpen: (change: HistoryChange) => void;
}) {
  const origin = originLook(run.writer.origin);
  return (
    <Card padding={3}>
      <VStack gap={3}>
        <HStack gap={2} vAlign="center" wrap="wrap">
          <Text weight="semibold">{formatDay(run.day)}</Text>
          <Badge label={origin.label} variant={origin.badge} />
          <Text type="supporting" wordBreak="break-word">
            {run.title}
          </Text>
          {run.writer.run === null ? null : (
            <Text type="code" color="secondary">
              {`pid ${run.writer.run}`}
            </Text>
          )}
        </HStack>
        {run.changes.map((ix) => {
          const change = changes[ix];
          return change === undefined ? null : (
            <Write key={change.writeId} change={change} onOpen={onOpen} />
          );
        })}
      </VStack>
    </Card>
  );
}

/** The diff of a write, read when the dialog opens. */
type DiffReading =
  | { writeId: string; diff: string }
  | { writeId: string; error: string };

function WriteDialog({
  lessonId,
  change,
  onClose,
}: {
  lessonId: string;
  change: HistoryChange;
  onClose: () => void;
}) {
  const [reading, setReading] = useState<DiffReading | null>(null);
  const writeId = change.writeId;

  useEffect(() => {
    let current = true;
    setReading(null);
    lessonWriteDiff(lessonId, writeId).then(
      (diff) => {
        if (current) setReading({ writeId, diff });
      },
      (reason: unknown) => {
        if (current) setReading({ writeId, error: String(reason) });
      },
    );
    return () => {
      current = false;
    };
  }, [lessonId, writeId]);

  let body: ReactNode;
  if (reading === null) {
    body = <Text color="secondary">Reading the change…</Text>;
  } else if ("error" in reading) {
    body = <Text color="secondary">{reading.error}</Text>;
  } else {
    body = (
      <CodeBlock
        code={reading.diff}
        language="yaml"
        isDiff
        isWrapped
        maxHeight="60dvh"
      />
    );
  }
  return (
    <Dialog
      isOpen
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      width={720}
      padding={4}
    >
      {/* The note the write carries (the contract's `LessonVersion.note`, as
          the writer gave it) heads the dialog; a write without one is named
          by its id. */}
      <DialogHeader
        title={change.writer.note || `Change in write ${change.shortId}`}
        subtitle={
          change.writer.note
            ? `Write ${change.shortId} · ${formatUpdatedFull(change.at)}`
            : formatUpdatedFull(change.at)
        }
        onOpenChange={(open) => {
          if (!open) onClose();
        }}
      />
      {body}
    </Dialog>
  );
}

/**
 * The history of a Lesson as the backend computed it: the writes grouped by
 * the run that made them on one day, newest first, each with its events. The
 * writes that changed nothing are hidden unless asked for.
 */
export function LessonHistoryView({
  lessonId,
  reading,
}: {
  lessonId: string;
  reading: HistoryReading | null;
}) {
  const [showQuiet, setShowQuiet] = useState(false);
  const [opened, setOpened] = useState<HistoryChange | null>(null);

  if (reading === null || reading.id !== lessonId) {
    return <Text color="secondary">Reading the history…</Text>;
  }
  if ("error" in reading) {
    return <Text color="secondary">{reading.error}</Text>;
  }
  const { history } = reading;
  if (history.changes.length === 0) {
    return <Text color="secondary">No history.</Text>;
  }
  const runs = showQuiet ? history.runsWithQuiet : history.runs;
  const runsCount = history.runsWithQuiet.length;
  const summary = [
    `${history.meaningful} ${history.meaningful === 1 ? "change" : "changes"}`,
    `${runsCount} ${runsCount === 1 ? "run" : "runs"}`,
    history.since === null ? null : `since ${formatDay(history.since)}`,
  ]
    .filter((part) => part !== null)
    .join(" · ");
  return (
    <VStack gap={3}>
      <HStack gap={2} vAlign="center" hAlign="between" wrap="wrap">
        <Text type="supporting">{summary}</Text>
        {history.quiet > 0 ? (
          <Switch
            size="sm"
            label={`Show ${history.quiet} unchanged`}
            value={showQuiet}
            onChange={setShowQuiet}
          />
        ) : null}
      </HStack>
      {runs.length === 0 ? (
        <Text color="secondary">No change worth showing.</Text>
      ) : (
        runs.map((run) => (
          <Run
            key={`${run.day}:${run.changes[0]}`}
            run={run}
            changes={history.changes}
            onOpen={setOpened}
          />
        ))
      )}
      {opened === null ? null : (
        <WriteDialog
          lessonId={lessonId}
          change={opened}
          onClose={() => setOpened(null)}
        />
      )}
    </VStack>
  );
}
