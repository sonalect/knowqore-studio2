import { Button } from "@astryxdesign/core/Button";
import { Link } from "@astryxdesign/core/Link";
import { MultiSelector } from "@astryxdesign/core/MultiSelector";
import type { OverflowItem } from "@astryxdesign/core/OverflowList";
import { OverflowList } from "@astryxdesign/core/OverflowList";
import { Popover } from "@astryxdesign/core/Popover";
import { HStack, StackItem, VStack } from "@astryxdesign/core/Stack";
import { Text } from "@astryxdesign/core/Text";
import {
  borderVars,
  colorVars,
  radiusVars,
} from "@astryxdesign/core/theme/tokens.stylex";
import * as stylex from "@stylexjs/stylex";
import { useEffect, useMemo, useState } from "react";
import {
  type LessonFacets,
  type LessonFilters,
  lessonFacets,
  NO_FILTERS,
} from "../lessons";
import { type FilterField, knownValues, valueLabel } from "../lessonValues";

const NUMBER = new Intl.NumberFormat();

// The width held back for the readout ("N results • Clear all"), so the
// controls fold before it is pushed off the row (as in the Astryx
// table-filter template).
const READOUT_RESERVE = 184;

// The fields in the order of the bar, with the name each shows.
const FIELDS: { field: FilterField; label: string }[] = [
  { field: "kind", label: "Kind" },
  { field: "status", label: "Status" },
  { field: "transfer", label: "Transfer" },
  { field: "polarity", label: "Polarity" },
];

const styles = stylex.create({
  row: { rowGap: 6 },
  // The slot is sized from the row, never from its controls, so folding one
  // does not shrink the room the list measures.
  slot: {
    display: "flex",
    alignItems: "center",
    flexGrow: 1,
    paddingInlineEnd: READOUT_RESERVE,
    flexShrink: 100,
    minWidth: 44 + READOUT_RESERVE,
  },
  // The readout draws over the reserve, against the last control.
  meta: {
    flexShrink: 0,
    marginInlineStart: 8,
    marginInlineEnd: -READOUT_RESERVE,
  },
  overflowItem: { display: "flex", flexDirection: "column" },
  chrome: {
    borderWidth: borderVars["--border-width"],
    borderStyle: "solid",
    borderColor: colorVars["--color-border-emphasized"],
    borderRadius: radiusVars["--radius-element"],
  },
  surface: { backgroundColor: colorVars["--color-background-surface"] },
  // The fill of a control that holds a value.
  filled: { backgroundColor: colorVars["--color-overlay-pressed"] },
  option: { flexGrow: 1 },
});

/** The folded controls, behind a "+N" button. */
function Overflow({ items }: { items: OverflowItem[] }) {
  const count = items.length;
  const label = `${count} more ${count === 1 ? "filter" : "filters"}`;
  return (
    <Popover
      placement="below"
      alignment="end"
      width={280}
      label={label}
      content={
        <VStack gap={2}>
          {items.map(({ child, index }) => (
            <StackItem key={index} xstyle={styles.overflowItem}>
              {child}
            </StackItem>
          ))}
        </VStack>
      }
    >
      <Button
        label={`+${count}`}
        tooltip={label}
        aria-label={label}
        variant="ghost"
        size="sm"
        xstyle={[styles.chrome, styles.surface]}
      />
    </Popover>
  );
}

/**
 * The filters of the table: a multi-select per field, with the number of
 * Lessons under the chosen node that have each value (not narrowed by the
 * other filters), and the count of results with a way to clear them all.
 */
export function LessonsFilterBar({
  node,
  filters,
  total,
  version,
  onChange,
}: {
  node: string | null;
  filters: LessonFilters;
  total: number | null;
  /** Moves when the data was read again: the counts are read again, the old ones stay until then. */
  version: number;
  onChange: (filters: LessonFilters) => void;
}) {
  // The facets with the node they were read for, so an answer for another
  // node (or a failed read) shows the values without counts.
  const [read, setRead] = useState<{
    node: string | null;
    facets: LessonFacets;
  } | null>(null);

  useEffect(() => {
    let current = true;
    void version;
    lessonFacets(node).then(
      (facets) => {
        if (current) setRead({ node, facets });
      },
      () => {
        // A failed re-read for the same node leaves its counts.
        if (current) setRead((was) => (was?.node === node ? was : null));
      },
    );
    return () => {
      current = false;
    };
  }, [node, version]);

  const counts = useMemo(() => {
    const byField = new Map<FilterField, Map<string, number>>();
    if (read !== null && read.node === node) {
      for (const { field } of FIELDS) {
        byField.set(
          field,
          new Map(read.facets[field].map((f) => [f.value, f.lessons])),
        );
      }
    }
    return byField;
  }, [read, node]);

  const active = FIELDS.some(({ field }) => filters[field].length > 0);

  const controls = FIELDS.map(({ field, label }) => {
    const fieldCounts = counts.get(field);
    const options = knownValues(field).map((value) => ({
      value,
      label: valueLabel(field, value),
    }));
    return (
      <MultiSelector
        key={field}
        label={`${label} filter`}
        isLabelHidden
        placeholder={label}
        size="sm"
        hasClear
        triggerDisplay="labels"
        // One name and a count keeps every control on the row.
        formatValue={(items) =>
          items.length > 1
            ? `${items[0]?.label}, +${items.length - 1}`
            : (items[0]?.label ?? "")
        }
        options={options}
        // The count sits at the end of the row; there is no slot for it
        // beside the label, so the row is drawn here.
        renderOption={(option) => (
          <HStack gap={2} hAlign="between" vAlign="center" width="100%">
            <Text xstyle={styles.option}>{option.label ?? option.value}</Text>
            {fieldCounts !== undefined && (
              <Text color="secondary" hasTabularNumbers>
                {NUMBER.format(fieldCounts.get(option.value) ?? 0)}
              </Text>
            )}
          </HStack>
        )}
        value={filters[field]}
        controlXstyle={filters[field].length > 0 ? styles.filled : undefined}
        onChange={(next) => onChange({ ...filters, [field]: next })}
      />
    );
  });

  return (
    <HStack
      gap={2}
      vAlign="center"
      wrap="nowrap"
      minHeight={32}
      xstyle={styles.row}
    >
      <StackItem xstyle={styles.slot}>
        <OverflowList
          behavior="observeParent"
          gap={2}
          minVisibleItems={1}
          overflowRenderer={(items) => <Overflow items={items} />}
        >
          {controls}
        </OverflowList>
        <HStack gap={2} vAlign="center" xstyle={styles.meta}>
          <Text type="supporting" color="secondary" hasTabularNumbers>
            {total === null
              ? "Reading…"
              : `${NUMBER.format(total)} ${total === 1 ? "result" : "results"}`}
          </Text>
          {active && (
            <>
              <Text type="supporting" color="secondary">
                •
              </Text>
              {/* Href-less, so Link renders a button: this resets the filters. */}
              <Link type="supporting" onClick={() => onChange(NO_FILTERS)}>
                Clear all
              </Link>
            </>
          )}
        </HStack>
      </StackItem>
    </HStack>
  );
}
