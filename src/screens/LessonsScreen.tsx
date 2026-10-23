import { AppShell } from "@astryxdesign/core/AppShell";
import { Badge } from "@astryxdesign/core/Badge";
import { BreadcrumbItem, Breadcrumbs } from "@astryxdesign/core/Breadcrumbs";
import { Card } from "@astryxdesign/core/Card";
import { Icon } from "@astryxdesign/core/Icon";
import { Layout, LayoutContent, LayoutPanel } from "@astryxdesign/core/Layout";
import { NavIcon } from "@astryxdesign/core/NavIcon";
import { ResizeHandle, useResizable } from "@astryxdesign/core/Resizable";
import { percent, pixel } from "@astryxdesign/core/Resizable/utils";
import {
  SideNav,
  SideNavHeading,
  SideNavItem,
  SideNavSection,
} from "@astryxdesign/core/SideNav";
import { StackItem, VStack } from "@astryxdesign/core/Stack";
import { Text } from "@astryxdesign/core/Text";
import { Toolbar } from "@astryxdesign/core/Toolbar";
import { TreeList, type TreeListItemData } from "@astryxdesign/core/TreeList";
import * as stylex from "@stylexjs/stylex";
import {
  BookOpen,
  Folder,
  FolderGit2,
  FolderTree,
  Layers,
  Library,
  Settings,
} from "lucide-react";
import {
  type ReactNode,
  type RefObject,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  type LessonFilters,
  type LessonRow,
  type LessonTree,
  type LessonTreeNode,
  lessonTree,
  NO_FILTERS,
} from "../lessons";
import type { OpenWorkspace } from "../workspace";
import { LessonDetails } from "./LessonDetails";
import { LessonsFilterBar } from "./LessonsFilterBar";
import { LessonsTable } from "./LessonsTable";

const NUMBER = new Intl.NumberFormat();

const styles = stylex.create({
  // AppShell fills the viewport by default; here it sits under Studio's
  // own title bar, so it fills its parent instead.
  shell: {
    height: "100%",
  },
});

/** The nodes from the kind down to the one with `key`, or none when no node has it. */
function pathTo(nodes: LessonTreeNode[], key: string): LessonTreeNode[] {
  for (const node of nodes) {
    if (node.key === key) return [node];
    const below = pathTo(node.children, key);
    if (below.length > 0) return [node, ...below];
  }
  return [];
}

/** The items of the tree list: kinds and hosts start expanded, deeper nodes collapsed. */
function treeItems(
  nodes: LessonTreeNode[],
  selected: string | null,
  choose: (key: string) => void,
  depth = 0,
): TreeListItemData[] {
  return nodes.map((node) => ({
    id: node.key,
    label: node.label,
    startContent: (
      <Icon
        icon={
          depth === 0 ? Layers : node.children.length > 0 ? Folder : FolderGit2
        }
        size="sm"
      />
    ),
    endContent: <Badge label={NUMBER.format(node.lessons)} />,
    isSelected: node.key === selected,
    isExpanded: depth < 2,
    onClick: () => choose(node.key),
    children:
      node.children.length > 0
        ? treeItems(node.children, selected, choose, depth + 1)
        : undefined,
  }));
}

/**
 * The details panel beside the table, with the divider that drags it wider.
 * It starts at 40% of the table and the panel together (`basis`) and goes up
 * to 70%; a dragged width is kept in the webview's storage for the next start.
 *
 * The width is set up here, when a Lesson is first chosen, not with the
 * screen: the screen mounts while the window still has its first size, before
 * it is restored, and a percentage taken then is a share of that.
 */
function DetailsPanel({
  basis,
  children,
}: {
  basis: RefObject<HTMLDivElement | null>;
  children: ReactNode;
}) {
  const width = useResizable({
    defaultSize: percent(40, { min: pixel(320) }),
    minSize: 320,
    maxSize: percent(70, { min: pixel(320) }),
    containerRef: basis,
    autoSaveId: "lesson-details-size",
  });
  return (
    <>
      <ResizeHandle
        resizable={width.props}
        isReversed
        hasDivider
        label="Resize the Lesson details"
      />
      <LayoutPanel
        resizable={width.props}
        padding={4}
        role="complementary"
        label="Lesson details"
      >
        {children}
      </LayoutPanel>
    </>
  );
}

/** The Lessons of the open workspace: a tree, a table, and the details of the selected Lesson. */
export function LessonsScreen({ workspace }: { workspace: OpenWorkspace }) {
  const [selected, setSelected] = useState<LessonRow | null>(null);
  const [total, setTotal] = useState<number | null>(null);
  const [node, setNodeState] = useState<string | null>(null);
  // Another source, or All Lessons, closes the details: the Lesson shown
  // belongs to the list left behind.
  const setNode = useCallback(
    (next: string | null) => {
      if (next !== node) setSelected(null);
      setNodeState(next);
    },
    [node],
  );
  // The folded groups of the chosen node: kept while it stays chosen, and
  // dropped when another is.
  const [folded, setFolded] = useState<{ node: string | null; keys: string[] }>(
    { node: null, keys: [] },
  );
  const collapsed = useMemo(
    () => (folded.node === node ? folded.keys : []),
    [folded, node],
  );
  const toggleGroup = useCallback(
    (key: string) =>
      setFolded((was) => {
        const keys = was.node === node ? was.keys : [];
        return {
          node,
          keys: keys.includes(key)
            ? keys.filter((k) => k !== key)
            : [...keys, key],
        };
      }),
    [node],
  );
  const [isNavCollapsed, setIsNavCollapsed] = useState(false);
  // The table and the details panel: the basis of the panel's percentages.
  const tableAndDetails = useRef<HTMLDivElement>(null);
  const [filters, setFilters] = useState<LessonFilters>(NO_FILTERS);
  // The tree, or the reason it could not be read; `null` while pending.
  const [tree, setTree] = useState<LessonTree | string | null>(null);

  useEffect(() => {
    lessonTree().then(setTree, (reason: unknown) => setTree(String(reason)));
  }, []);

  const items = useMemo(
    () =>
      typeof tree === "object" && tree !== null
        ? treeItems(tree.nodes, node, setNode)
        : [],
    [tree, node, setNode],
  );
  const path = useMemo(
    () =>
      node !== null && typeof tree === "object" && tree !== null
        ? pathTo(tree.nodes, node)
        : [],
    [tree, node],
  );

  return (
    <AppShell
      contentPadding={0}
      xstyle={styles.shell}
      sideNav={
        <SideNav
          collapsible={{
            isCollapsed: isNavCollapsed,
            onCollapsedChange: setIsNavCollapsed,
          }}
          resizable={{
            defaultWidth: 280,
            minWidth: 220,
            maxWidth: 420,
            // The dragged width is kept in the webview's storage.
            autoSaveId: "side-nav-width",
          }}
          header={
            <SideNavHeading
              heading="Knowqore"
              icon={<NavIcon icon={<Icon icon={Library} size="sm" />} />}
            />
          }
          footer={
            <SideNavSection title="Workspace" isHeaderHidden>
              <SideNavItem label="Settings" icon={Settings} isDisabled />
              <SideNavItem
                label={workspace.home}
                icon={FolderGit2}
                isDisabled
              />
            </SideNavSection>
          }
        >
          <SideNavSection title="Lessons">
            <SideNavItem
              label="All Lessons"
              icon={BookOpen}
              isSelected={node === null}
              onClick={() => setNode(null)}
            />
          </SideNavSection>
          <SideNavSection title="Sources">
            {isNavCollapsed ? (
              // The rail has no room for the tree: one icon stands for it,
              // marked while a node narrows the table, and opens the nav.
              <SideNavItem
                label="Sources"
                icon={FolderTree}
                isSelected={node !== null}
                onClick={() => setIsNavCollapsed(false)}
              />
            ) : tree === null ? (
              <Text color="secondary">Reading the sources…</Text>
            ) : typeof tree === "string" ? (
              <Text color="secondary">{tree}</Text>
            ) : tree.nodes.length === 0 ? (
              <Text color="secondary">No sources yet.</Text>
            ) : (
              // TreeList draws the guide lines of the nesting itself.
              <TreeList density="compact" variant="lineGuides" items={items} />
            )}
          </SideNavSection>
        </SideNav>
      }
    >
      <Layout
        ref={tableAndDetails}
        height="fill"
        header={
          <VStack gap={0}>
            <Toolbar
              label="Lessons"
              size="sm"
              dividers={["bottom"]}
              startContent={
                <Breadcrumbs label="Location">
                  <BreadcrumbItem onClick={() => setNode(null)}>
                    Lessons
                  </BreadcrumbItem>
                  {path.length === 0 ? (
                    <BreadcrumbItem isCurrent>All Lessons</BreadcrumbItem>
                  ) : (
                    path.map((crumb, index) =>
                      index === path.length - 1 ? (
                        <BreadcrumbItem key={crumb.key} isCurrent>
                          {crumb.label}
                        </BreadcrumbItem>
                      ) : (
                        <BreadcrumbItem
                          key={crumb.key}
                          onClick={() => setNode(crumb.key)}
                        >
                          {crumb.label}
                        </BreadcrumbItem>
                      ),
                    )
                  )}
                </Breadcrumbs>
              }
            />
            {/* In the header, so the table and the details card start at one height. */}
            <VStack paddingInline={4} paddingBlockStart={4}>
              <LessonsFilterBar
                node={node}
                filters={filters}
                total={total}
                onChange={setFilters}
              />
            </VStack>
          </VStack>
        }
        content={
          <LayoutContent padding={4} isScrollable={false}>
            <VStack height="100%">
              <StackItem size="fill">
                <Card padding={0} height="100%">
                  <LessonsTable
                    node={node}
                    filters={filters}
                    collapsed={collapsed}
                    onToggleGroup={toggleGroup}
                    selectedId={selected?.id ?? null}
                    onSelect={setSelected}
                    onTotal={setTotal}
                  />
                </Card>
              </StackItem>
            </VStack>
          </LayoutContent>
        }
        end={
          selected ? (
            <DetailsPanel basis={tableAndDetails}>
              <LessonDetails row={selected} onClose={() => setSelected(null)} />
            </DetailsPanel>
          ) : undefined
        }
      />
    </AppShell>
  );
}
