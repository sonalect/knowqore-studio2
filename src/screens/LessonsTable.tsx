import { Badge } from "@astryxdesign/core/Badge";
import { HStack, VStack } from "@astryxdesign/core/Stack";
import { StatusDot } from "@astryxdesign/core/StatusDot";
import {
  pixel,
  proportional,
  Table,
  type TableColumn,
  type TablePlugin,
  useTableSortable,
} from "@astryxdesign/core/Table";
import { Text } from "@astryxdesign/core/Text";
import { Tooltip } from "@astryxdesign/core/Tooltip";
import { colorVars } from "@astryxdesign/core/theme/tokens.stylex";
import * as stylex from "@stylexjs/stylex";
import {
  type Ref,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  formatUpdated,
  formatUpdatedFull,
  type LessonFilters,
  type LessonRow,
  type LessonSortKey,
  lessonsPage,
  type SortDirection,
} from "../lessons";
import { kindLook, polarityLook, statusLook } from "../lessonValues";

/** Rows are fetched in blocks of this many. */
const BLOCK = 100;
/** Rows rendered beyond the visible ones, each way. */
const OVERSCAN = 10;
/** Until a real row is measured. */
const ESTIMATED_ROW_HEIGHT = 52;
/** Until the scroll container is measured. */
const ESTIMATED_VIEWPORT = 600;

interface Sort {
  sortKey: LessonSortKey;
  direction: SortDirection;
}

const DEFAULT_SORT: Sort = { sortKey: "updatedAt", direction: "descending" };

/** A row of the table: a Lesson, a blank placeholder, or a spacer. */
type Item = LessonRow & { marker?: "placeholder" | "spacer" };

/** A value with an optional colored dot before it; plain text for an unknown value. */
function DotValue({
  value,
  look,
}: {
  value: string;
  look: ReturnType<typeof kindLook>;
}) {
  if (!look) {
    return <Text maxLines={1}>{value}</Text>;
  }
  return (
    <HStack gap={2} vAlign="center">
      {look.dot && <StatusDot variant={look.dot} label={look.label} />}
      <Text maxLines={1}>{look.label}</Text>
    </HStack>
  );
}

const COLUMNS: TableColumn<Item>[] = [
  {
    key: "statement",
    header: "Lesson",
    width: proportional(4),
    sortable: true,
    renderCell: (item) => (
      <VStack gap={0.5}>
        <Text maxLines={1} hasTruncateTooltip={false}>
          {item.statement}
        </Text>
        <Text type="supporting" size="xsm" maxLines={1}>
          {item.id}
        </Text>
      </VStack>
    ),
  },
  {
    key: "kind",
    header: "Kind",
    width: proportional(1, { minWidth: 130 }),
    sortable: true,
    renderCell: (item) => (
      <DotValue value={item.kind} look={kindLook(item.kind)} />
    ),
  },
  {
    key: "status",
    header: "Status",
    width: pixel(110),
    sortable: true,
    renderCell: (item) => {
      const look = statusLook(item.status);
      return look ? (
        <Badge variant={look.badge} label={look.label} />
      ) : (
        <Text maxLines={1}>{item.status}</Text>
      );
    },
  },
  {
    key: "polarity",
    header: "Polarity",
    width: pixel(100),
    renderCell: (item) => (
      <DotValue value={item.polarity} look={polarityLook(item.polarity)} />
    ),
  },
  {
    key: "transfer",
    header: "Transfer",
    width: proportional(1),
    sortable: true,
  },
  {
    key: "updatedAt",
    header: "Updated",
    width: proportional(1, { minWidth: 150 }),
    sortable: true,
    renderCell: (item) => (
      <Tooltip content={formatUpdatedFull(item.updatedAt)}>
        <Text maxLines={1}>{formatUpdated(item.updatedAt)}</Text>
      </Tooltip>
    ),
  },
];

const styles = stylex.create({
  scroll: {
    height: "100%",
    overflowY: "auto",
  },
  header: {
    position: "sticky",
    top: 0,
    zIndex: 1,
    backgroundColor: colorVars["--color-background-card"],
  },
  row: {
    cursor: "default",
  },
  selectedRow: {
    boxShadow: `inset 0 0 0 100vmax ${colorVars["--color-accent-muted"]}`,
  },
  height: (height: number) => ({ height }),
  error: {
    color: colorVars["--color-error"],
  },
});

// The columns the backend sorts by; the polarity column is not one of them.
function isSortKey(key: string): key is LessonSortKey {
  return COLUMNS.some((column) => column.key === key && column.sortable);
}

function blank(id: string, marker: "placeholder" | "spacer"): Item {
  return {
    id,
    statement: "",
    kind: "",
    status: "",
    transfer: "",
    polarity: "",
    updatedAt: "",
    marker,
  };
}

/** The Lessons of the Vault in a table that holds only the rows near the view. */
export function LessonsTable({
  node,
  filters,
  selectedId,
  onSelect,
  onTotal,
}: {
  node: string | null;
  filters: LessonFilters;
  selectedId: string | null;
  onSelect: (row: LessonRow) => void;
  onTotal: (total: number) => void;
}) {
  const [sort, setSort] = useState<Sort>(DEFAULT_SORT);
  const [total, setTotal] = useState<number | null>(null);
  const [blocks, setBlocks] = useState<ReadonlyMap<number, LessonRow[]>>(
    () => new Map(),
  );
  const [error, setError] = useState<string | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewport, setViewport] = useState(ESTIMATED_VIEWPORT);
  const [rowHeight, setRowHeight] = useState(ESTIMATED_ROW_HEIGHT);

  // Bumped by every sort or node change: an answer of an older one is ignored.
  const generation = useRef(0);
  // `generation:block` of every call in flight.
  const inFlight = useRef(new Set<string>());
  // The blocks held, readable without being an effect dependency.
  const loaded = useRef<ReadonlyMap<number, LessonRow[]>>(blocks);
  // The blocks the view needs now, for answers that arrive late.
  const needed = useRef({ lo: 0, hi: 0 });
  const scrollEl = useRef<HTMLDivElement | null>(null);
  const observer = useRef<ResizeObserver | null>(null);

  const lastRow = total === null ? 0 : Math.max(total - 1, 0);
  const visFirst = Math.min(Math.floor(scrollTop / rowHeight), lastRow);
  const visLast = Math.min(
    Math.ceil((scrollTop + viewport) / rowHeight),
    lastRow,
  );
  const lo = total === null ? 0 : Math.max(Math.floor(visFirst / BLOCK) - 1, 0);
  const hi =
    total === null
      ? 0
      : Math.min(Math.floor(visLast / BLOCK) + 1, Math.floor(lastRow / BLOCK));
  needed.current = { lo, hi };

  const sortKey = sort.sortKey;
  const direction = sort.direction;

  // The filters by value: a re-render with equal filters changes nothing.
  const filtersKey = JSON.stringify(filters);
  const stableFilters = useMemo<LessonFilters>(
    () => JSON.parse(filtersKey) as LessonFilters,
    [filtersKey],
  );

  // A node or filter change starts the list over, as a sort change does; the
  // sort stays.
  const shown = useRef({ node, filtersKey });
  useEffect(() => {
    if (shown.current.node === node && shown.current.filtersKey === filtersKey)
      return;
    shown.current = { node, filtersKey };
    generation.current += 1;
    inFlight.current.clear();
    loaded.current = new Map();
    setBlocks(loaded.current);
    setError(null);
    scrollEl.current?.scrollTo({ top: 0 });
    setScrollTop(0);
  }, [node, filtersKey]);

  useEffect(() => {
    const pruned = new Map(
      [...loaded.current].filter(([block]) => block >= lo && block <= hi),
    );
    if (pruned.size !== loaded.current.size) {
      loaded.current = pruned;
      setBlocks(pruned);
    }
    const gen = generation.current;
    for (let block = lo; block <= hi; block += 1) {
      const key = `${gen}:${block}`;
      if (loaded.current.has(block) || inFlight.current.has(key)) continue;
      inFlight.current.add(key);
      lessonsPage({
        sortKey,
        direction,
        offset: block * BLOCK,
        limit: BLOCK,
        node,
        filters: stableFilters,
      })
        .then((page) => {
          inFlight.current.delete(key);
          if (gen !== generation.current) return;
          setError(null);
          setTotal(page.total);
          onTotal(page.total);
          const { lo: nlo, hi: nhi } = needed.current;
          if (block < nlo || block > nhi) return;
          const next = new Map(loaded.current);
          next.set(block, page.rows);
          loaded.current = next;
          setBlocks(next);
        })
        .catch((reason: unknown) => {
          inFlight.current.delete(key);
          if (gen !== generation.current) return;
          setError(String(reason));
        });
    }
  }, [lo, hi, sortKey, direction, node, stableFilters, onTotal]);

  const attachScroll = useCallback((el: HTMLDivElement | null) => {
    observer.current?.disconnect();
    observer.current = null;
    scrollEl.current = el;
    if (!el) return;
    setViewport(el.clientHeight);
    setScrollTop(el.scrollTop);
    const watcher = new ResizeObserver(() => setViewport(el.clientHeight));
    watcher.observe(el);
    observer.current = watcher;
  }, []);

  const measureRow = useCallback((el: HTMLTableRowElement | null) => {
    if (!el) return;
    const height = el.getBoundingClientRect().height;
    if (height > 0) setRowHeight((prev) => (prev === height ? prev : height));
  }, []);

  const changeSort = useCallback((next: Sort) => {
    generation.current += 1;
    inFlight.current.clear();
    loaded.current = new Map();
    setBlocks(loaded.current);
    setError(null);
    scrollEl.current?.scrollTo({ top: 0 });
    setScrollTop(0);
    setSort(next);
  }, []);

  const sortable = useTableSortable<Item>({
    sort: [sort],
    onSortChange: (next) => {
      const first = next[0];
      if (first && isSortKey(first.sortKey)) {
        changeSort({ sortKey: first.sortKey, direction: first.direction });
      }
    },
  });

  const rFirst = Math.max(visFirst - OVERSCAN, 0);
  const rLast = Math.min(visLast + OVERSCAN, lastRow);
  const count = total ?? 0;

  const items = useMemo(() => {
    const out: Item[] = [];
    if (count === 0) return out;
    if (rFirst > 0) {
      out.push(blank("spacer-top", "spacer"));
    }
    for (let index = rFirst; index <= rLast; index += 1) {
      const row = blocks.get(Math.floor(index / BLOCK))?.[index % BLOCK];
      out.push(row ?? blank(`placeholder-${index}`, "placeholder"));
    }
    if (rLast < count - 1) {
      out.push(blank("spacer-bottom", "spacer"));
    }
    return out;
  }, [blocks, count, rFirst, rLast]);

  const spacerHeights = useMemo(
    () => ({
      "spacer-top": rFirst * rowHeight,
      "spacer-bottom": (count - rLast - 1) * rowHeight,
    }),
    [count, rFirst, rLast, rowHeight],
  );

  const windowing = useMemo<TablePlugin<Item>>(
    () => ({
      transformScrollWrapper: (props) => ({
        ...props,
        htmlProps: {
          ...props.htmlProps,
          // A new callback on every render would attach the observer again on
          // every scroll; another plugin's ref is composed only when there is one.
          ref: props.htmlProps.ref
            ? composeRefs(props.htmlProps.ref, attachScroll)
            : attachScroll,
          onScroll: (event) => {
            props.htmlProps.onScroll?.(event);
            setScrollTop(event.currentTarget.scrollTop);
          },
        },
        xstyle: [...props.xstyle, styles.scroll],
      }),
      transformHeaderCell: (props) => ({
        ...props,
        xstyle: [...props.xstyle, styles.header],
      }),
      transformBodyCell: (props, _column, item) =>
        item.marker ? { ...props, isContentSuppressed: true } : props,
      transformBodyRow: (props, item) => {
        if (item.marker === "spacer") {
          const height =
            item.id === "spacer-top"
              ? spacerHeights["spacer-top"]
              : spacerHeights["spacer-bottom"];
          return {
            ...props,
            xstyle: [...props.xstyle, styles.height(height)],
            htmlProps: { ...props.htmlProps, "aria-hidden": true },
          };
        }
        if (item.marker === "placeholder") {
          return {
            ...props,
            xstyle: [...props.xstyle, styles.height(rowHeight)],
            htmlProps: { ...props.htmlProps, "aria-hidden": true },
          };
        }
        const isSelected = item.id === selectedId;
        return {
          ...props,
          ref: props.ref ? composeRefs(props.ref, measureRow) : measureRow,
          xstyle: [
            ...props.xstyle,
            styles.row,
            isSelected && styles.selectedRow,
          ],
          htmlProps: {
            ...props.htmlProps,
            "aria-selected": isSelected ? true : undefined,
            onClick: () => onSelect(item),
          },
        };
      },
    }),
    [attachScroll, measureRow, onSelect, rowHeight, selectedId, spacerHeights],
  );

  if (error !== null) {
    return <Text xstyle={styles.error}>{error}</Text>;
  }
  if (total === null) {
    return <Text color="secondary">Reading the Lessons…</Text>;
  }
  if (total === 0) {
    return (
      <Text color="secondary">
        {Object.values(filters).some((values) => values.length > 0)
          ? "No Lessons match the filters."
          : node === null
            ? "There are no Lessons yet."
            : "No Lessons are seen under this source."}
      </Text>
    );
  }

  return (
    <Table
      data={items}
      columns={COLUMNS}
      idKey="id"
      density="compact"
      hasHover
      textOverflow="truncate"
      plugins={{ sortable, windowing }}
    />
  );
}

/** One ref callback or object that feeds several. */
function composeRefs<E>(
  ...refs: (Ref<E> | undefined)[]
): (el: E | null) => void {
  return (el) => {
    for (const ref of refs) {
      if (typeof ref === "function") ref(el);
      else if (ref) ref.current = el;
    }
  };
}
