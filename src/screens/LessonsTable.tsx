import { Badge } from "@astryxdesign/core/Badge";
import { Icon } from "@astryxdesign/core/Icon";
import { HStack, VStack } from "@astryxdesign/core/Stack";
import { StatusDot } from "@astryxdesign/core/StatusDot";
import {
  pixel,
  proportional,
  Table,
  type TableColumn,
  type TablePlugin,
  useTableColumnResize,
  useTableSortable,
} from "@astryxdesign/core/Table";
import { Text } from "@astryxdesign/core/Text";
import { Tooltip } from "@astryxdesign/core/Tooltip";
import { colorVars, spacingVars } from "@astryxdesign/core/theme/tokens.stylex";
import * as stylex from "@stylexjs/stylex";
import { ChevronDown, ChevronRight } from "lucide-react";
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
  type GroupHeader,
  type LessonFilters,
  type LessonRow,
  type LessonSortKey,
  lessonsPage,
  type PageItem,
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

/**
 * A row of the table: a Lesson, a group header, a blank placeholder, or a
 * spacer. `rowKey` is the row's position, since a Lesson seen in two groups
 * appears twice with one id.
 */
type Item = LessonRow & {
  rowKey: string;
  marker?: "placeholder" | "spacer";
  group?: GroupHeader;
};

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

// Widths of the columns after the Lesson's, and the floors of the Lesson
// and Updated columns, in pixels.
const KIND_WIDTH = 130;
const STATUS_WIDTH = 110;
const POLARITY_WIDTH = 100;
const TRANSFER_WIDTH = 100;
const UPDATED_MIN_WIDTH = 150;
const LESSON_MIN_WIDTH = 320;

// Every column but the last has a pixel width, so dragging its border
// resizes that column itself and the table grows sideways; Astryx resizes a
// proportional column's neighbour instead. The last, Updated, is proportional
// and takes the room left over. The Lesson column's width until the person
// sets one is the room the others leave (see `fittedWidths`).
const COLUMNS: TableColumn<Item>[] = [
  {
    key: "statement",
    header: "Lesson",
    width: pixel(LESSON_MIN_WIDTH),
    sortable: true,
    renderCell: (item) => (
      <VStack gap={0.5}>
        {/* A retired Lesson is struck through: it is kept, but no longer said. */}
        <Text
          maxLines={1}
          hasTruncateTooltip={false}
          hasStrikethrough={item.status === "retired"}
        >
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
    width: pixel(KIND_WIDTH),
    sortable: true,
    renderCell: (item) => (
      <DotValue value={item.kind} look={kindLook(item.kind)} />
    ),
  },
  {
    key: "status",
    header: "Status",
    width: pixel(STATUS_WIDTH),
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
    width: pixel(POLARITY_WIDTH),
    renderCell: (item) => (
      <DotValue value={item.polarity} look={polarityLook(item.polarity)} />
    ),
  },
  {
    key: "transfer",
    header: "Transfer",
    width: pixel(TRANSFER_WIDTH),
    sortable: true,
  },
  {
    key: "updatedAt",
    header: "Updated",
    width: proportional(1, { minWidth: UPDATED_MIN_WIDTH }),
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
  // The arrow, as everywhere in Studio: Studio's own StyleX is unlayered, so a
  // `pointer` here would win over the page rule that keeps it (src/index.css).
  groupRow: {
    cursor: "default",
    // Set apart from the Lesson rows in both themes: lighter than the card
    // in the dark one, a shade darker in the light one. (Muted is the card's
    // own color in the dark theme.)
    backgroundColor: "var(--color-background-gray)",
  },
  // The plugin's own chevron pattern: a bare button, since IconButton passes
  // no aria-expanded through.
  chevron: {
    display: "inline-flex",
    alignItems: "center",
    padding: spacingVars["--spacing-1"],
    border: "none",
    background: "none",
    color: colorVars["--color-text-secondary"],
    cursor: "default",
  },
  groupCell: {
    paddingBlock: 0,
    paddingInline: spacingVars["--spacing-2"],
  },
  selectedCell: {
    backgroundColor: colorVars["--color-accent-muted"],
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

function blank(id: string, marker?: "placeholder" | "spacer"): Item {
  return {
    id,
    rowKey: id,
    statement: "",
    kind: "",
    status: "",
    transfer: "",
    polarity: "",
    updatedAt: "",
    marker,
  };
}

/**
 * The column widths to draw: the ones the person set, and for the Lesson
 * column, until they set it, the room the other columns leave in a table
 * `width` wide (never below its floor), so it follows the window. Nothing
 * here is kept: only a drag is.
 */
function fittedWidths(
  widths: Record<string, number>,
  width: number,
): Record<string, number> {
  if (widths.statement !== undefined || width <= 0) return widths;
  const others =
    (widths.kind ?? KIND_WIDTH) +
    (widths.status ?? STATUS_WIDTH) +
    (widths.polarity ?? POLARITY_WIDTH) +
    (widths.transfer ?? TRANSFER_WIDTH) +
    UPDATED_MIN_WIDTH;
  return {
    ...widths,
    statement: Math.max(LESSON_MIN_WIDTH, Math.floor(width - others)),
  };
}

/** What the column resizing reads of the columns: their keys and declared widths. */
const COLUMN_SIZES: TableColumn<Record<string, unknown>>[] = COLUMNS.map(
  ({ key, header, width }) => ({
    key,
    header,
    width,
  }),
);

/** Where the widths the person gave the columns are kept, in the webview's storage. */
const COLUMN_WIDTHS_KEY = "lessons-table-widths";

/**
 * The column widths kept from the last time, by column key. Storage is a
 * convenience: when it cannot be read, or holds something else, the columns
 * take their declared widths.
 */
function readColumnWidths(): Record<string, number> {
  try {
    const kept: unknown = JSON.parse(
      localStorage.getItem(COLUMN_WIDTHS_KEY) ?? "{}",
    );
    if (typeof kept !== "object" || kept === null) return {};
    return Object.fromEntries(
      Object.entries(kept).filter(
        ([key, width]) =>
          key !== "updatedAt" &&
          COLUMNS.some((column) => column.key === key) &&
          typeof width === "number" &&
          Number.isFinite(width) &&
          width > 0,
      ),
    );
  } catch {
    return {};
  }
}

/** Keeps the column widths for the next start; a failure only loses them. */
function keepColumnWidths(widths: Record<string, number>) {
  try {
    localStorage.setItem(COLUMN_WIDTHS_KEY, JSON.stringify(widths));
  } catch {
    // Storage is blocked or full: the widths hold for this session only.
  }
}

/** The Lessons of the Vault in a table that holds only the rows near the view. */
export function LessonsTable({
  node,
  filters,
  selectedId,
  collapsed,
  onToggleGroup,
  onSelect,
  onRefreshed,
  onTotal,
  version,
}: {
  node: string | null;
  filters: LessonFilters;
  /** The keys of the groups folded shut. */
  collapsed: string[];
  onToggleGroup: (key: string) => void;
  selectedId: string | null;
  onSelect: (row: LessonRow) => void;
  /** The selected Lesson as the latest answer holds it. */
  onRefreshed: (row: LessonRow) => void;
  onTotal: (total: number) => void;
  /**
   * Moves when the data was read again: the blocks in view are fetched
   * again and replace the old ones as they arrive, with no placeholders and
   * no scrolling in between.
   */
  version: number;
}) {
  const [sort, setSort] = useState<Sort>(DEFAULT_SORT);
  const [total, setTotal] = useState<number | null>(null);
  const [blocks, setBlocks] = useState<ReadonlyMap<number, PageItem[]>>(
    () => new Map(),
  );
  const [error, setError] = useState<string | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewport, setViewport] = useState(ESTIMATED_VIEWPORT);
  // The width of the scroll container; 0 until it is measured.
  const [width, setWidth] = useState(0);
  const [rowHeight, setRowHeight] = useState(ESTIMATED_ROW_HEIGHT);

  // Bumped by every sort or node change: an answer of an older one is ignored.
  const generation = useRef(0);
  // The blocks of `loaded` already fetched for this generation; after a
  // refresh the rest are the old rows, shown until their answer comes.
  const fresh = useRef(new Set<number>());
  const selectedIdRef = useRef(selectedId);
  selectedIdRef.current = selectedId;
  // `generation:block` of every call in flight.
  const inFlight = useRef(new Set<string>());
  // The blocks held, readable without being an effect dependency.
  const loaded = useRef<ReadonlyMap<number, PageItem[]>>(blocks);
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

  // The folded groups by value, likewise.
  const collapsedKey = JSON.stringify(collapsed);
  const stableCollapsed = useMemo<string[]>(
    () => JSON.parse(collapsedKey) as string[],
    [collapsedKey],
  );

  // A node, filter, or folded-group change starts the list over, as a sort
  // change does; the sort stays. Only a folded group keeps the scroll
  // position: the person is looking at the group they just toggled.
  const shown = useRef({ node, filtersKey, collapsedKey });
  useEffect(() => {
    const was = shown.current;
    if (
      was.node === node &&
      was.filtersKey === filtersKey &&
      was.collapsedKey === collapsedKey
    )
      return;
    shown.current = { node, filtersKey, collapsedKey };
    generation.current += 1;
    inFlight.current.clear();
    fresh.current = new Set();
    loaded.current = new Map();
    setBlocks(loaded.current);
    setError(null);
    if (was.node !== node || was.filtersKey !== filtersKey) {
      scrollEl.current?.scrollTo({ top: 0 });
      setScrollTop(0);
    }
  }, [node, filtersKey, collapsedKey]);

  // New data: a new generation that keeps the rows on screen. The effect
  // below fetches the blocks in view again.
  const shownVersion = useRef(version);
  useEffect(() => {
    if (shownVersion.current === version) return;
    shownVersion.current = version;
    generation.current += 1;
    inFlight.current.clear();
    fresh.current = new Set();
  }, [version]);

  useEffect(() => {
    void version;
    const pruned = new Map(
      [...loaded.current].filter(([block]) => block >= lo && block <= hi),
    );
    for (const block of fresh.current) {
      if (block < lo || block > hi) fresh.current.delete(block);
    }
    if (pruned.size !== loaded.current.size) {
      loaded.current = pruned;
      setBlocks(pruned);
    }
    const gen = generation.current;
    for (let block = lo; block <= hi; block += 1) {
      const key = `${gen}:${block}`;
      if (fresh.current.has(block) || inFlight.current.has(key)) continue;
      inFlight.current.add(key);
      lessonsPage({
        sortKey,
        direction,
        offset: block * BLOCK,
        limit: BLOCK,
        node,
        filters: stableFilters,
        collapsed: stableCollapsed,
      })
        .then((page) => {
          inFlight.current.delete(key);
          if (gen !== generation.current) return;
          setError(null);
          setTotal(page.total);
          // The readout counts Lessons, not the rows and headers scrolled.
          onTotal(page.lessons);
          const chosen = selectedIdRef.current;
          if (chosen !== null) {
            for (const item of page.items) {
              if (item.type === "lesson" && item.id === chosen) {
                onRefreshed({
                  id: item.id,
                  statement: item.statement,
                  kind: item.kind,
                  status: item.status,
                  transfer: item.transfer,
                  polarity: item.polarity,
                  updatedAt: item.updatedAt,
                });
              }
            }
          }
          const { lo: nlo, hi: nhi } = needed.current;
          if (block < nlo || block > nhi) return;
          fresh.current.add(block);
          const next = new Map(loaded.current);
          next.set(block, page.items);
          loaded.current = next;
          setBlocks(next);
        })
        .catch((reason: unknown) => {
          inFlight.current.delete(key);
          if (gen !== generation.current) return;
          setError(String(reason));
        });
    }
  }, [
    lo,
    hi,
    sortKey,
    direction,
    node,
    stableFilters,
    stableCollapsed,
    onTotal,
    onRefreshed,
    version,
  ]);

  const attachScroll = useCallback((el: HTMLDivElement | null) => {
    observer.current?.disconnect();
    observer.current = null;
    scrollEl.current = el;
    if (!el) return;
    setViewport(el.clientHeight);
    setWidth(el.clientWidth);
    setScrollTop(el.scrollTop);
    const watcher = new ResizeObserver(() => {
      setViewport(el.clientHeight);
      setWidth(el.clientWidth);
    });
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
    fresh.current = new Set();
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

  // The person drags the column borders; the widths are kept for the next
  // start.
  const [columnWidths, setColumnWidths] =
    useState<Record<string, number>>(readColumnWidths);
  const columnResize = useTableColumnResize<Item>({
    columns: COLUMN_SIZES,
    columnWidths: fittedWidths(columnWidths, width),
    onColumnResizeEnd: (updates) => {
      // Updated, the last column, always takes the room left over: its width
      // is never kept.
      const { updatedAt: _remainder, ...kept } = updates;
      setColumnWidths((widths) => {
        const next = { ...widths, ...kept };
        keepColumnWidths(next);
        return next;
      });
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
      const entry = blocks.get(Math.floor(index / BLOCK))?.[index % BLOCK];
      if (!entry) {
        out.push(blank(`placeholder-${index}`, "placeholder"));
      } else if (entry.type === "group") {
        out.push({
          ...blank(`group:${entry.key}`),
          rowKey: `row-${index}`,
          group: entry,
        });
      } else {
        out.push({ ...entry, rowKey: `row-${index}` });
      }
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
        item.marker || item.group
          ? { ...props, isContentSuppressed: true }
          : item.id === selectedId
            ? // The wash goes on the cells: WebKit does not paint a row's own
              // background or inset shadow under cells that have theirs.
              { ...props, xstyle: [...props.xstyle, styles.selectedCell] }
            : props,
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
        const group = item.group;
        if (group) {
          const label = `${group.collapsed ? "Expand" : "Collapse"} ${group.label}`;
          return {
            ...props,
            xstyle: [
              ...props.xstyle,
              styles.groupRow,
              styles.height(rowHeight),
            ],
            htmlProps: {
              ...props.htmlProps,
              onClick: () => onToggleGroup(group.key),
            },
            // One full-width cell, as the grouped-rows plugin draws its
            // headers; a colSpan past the column count is clamped.
            children: (
              <td colSpan={999} {...stylex.props(styles.groupCell)}>
                <HStack gap={2} vAlign="center">
                  <button
                    type="button"
                    aria-label={label}
                    aria-expanded={!group.collapsed}
                    {...stylex.props(styles.chevron)}
                    onClick={(event) => {
                      event.stopPropagation();
                      onToggleGroup(group.key);
                    }}
                  >
                    <Icon
                      icon={group.collapsed ? ChevronRight : ChevronDown}
                      size="sm"
                    />
                  </button>
                  <Text weight="semibold" maxLines={1}>
                    {group.label}
                  </Text>
                  <Text color="secondary">({group.lessons})</Text>
                </HStack>
              </td>
            ),
          };
        }
        const isSelected = item.id === selectedId;
        return {
          ...props,
          ref: props.ref ? composeRefs(props.ref, measureRow) : measureRow,
          xstyle: [...props.xstyle, styles.row],
          htmlProps: {
            ...props.htmlProps,
            "aria-selected": isSelected ? true : undefined,
            onClick: () => onSelect(item),
          },
        };
      },
    }),
    [
      attachScroll,
      measureRow,
      onSelect,
      onToggleGroup,
      rowHeight,
      selectedId,
      spacerHeights,
    ],
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
      idKey="rowKey"
      density="compact"
      hasHover
      textOverflow="truncate"
      plugins={{ sortable, columnResize, windowing }}
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
