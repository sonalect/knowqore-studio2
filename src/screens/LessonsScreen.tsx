import { AppShell } from "@astryxdesign/core/AppShell";
import { BreadcrumbItem, Breadcrumbs } from "@astryxdesign/core/Breadcrumbs";
import { Card } from "@astryxdesign/core/Card";
import { Divider } from "@astryxdesign/core/Divider";
import { Grid } from "@astryxdesign/core/Grid";
import { Heading } from "@astryxdesign/core/Heading";
import { Icon } from "@astryxdesign/core/Icon";
import { IconButton } from "@astryxdesign/core/IconButton";
import { Layout, LayoutContent, LayoutPanel } from "@astryxdesign/core/Layout";
import { NavIcon } from "@astryxdesign/core/NavIcon";
import {
  SideNav,
  SideNavHeading,
  SideNavItem,
  SideNavSection,
} from "@astryxdesign/core/SideNav";
import { HStack, VStack } from "@astryxdesign/core/Stack";
import {
  proportional,
  Table,
  type TableColumn,
  type TablePlugin,
} from "@astryxdesign/core/Table";
import { Text } from "@astryxdesign/core/Text";
import { Toolbar } from "@astryxdesign/core/Toolbar";
import { TreeList } from "@astryxdesign/core/TreeList";
import { colorVars } from "@astryxdesign/core/theme/tokens.stylex";
import * as stylex from "@stylexjs/stylex";
import {
  BookOpen,
  Clock,
  FolderGit2,
  Layers,
  Library,
  Settings,
  X,
} from "lucide-react";
import { useMemo, useState } from "react";
import type { OpenWorkspace } from "../workspace";

// Placeholder rows: the skeleton shows the shape of the screen, not the
// Vault's Lessons yet.
interface LessonRow extends Record<string, unknown> {
  id: string;
  title: string;
  kind: string;
  source: string;
  updated: string;
}

const ROWS: LessonRow[] = Array.from({ length: 12 }, (_, index) => ({
  id: `lesson-${index + 1}`,
  title: `Lesson ${index + 1}`,
  kind: "—",
  source: "—",
  updated: "—",
}));

const COLUMNS: TableColumn<LessonRow>[] = [
  { key: "title", header: "Lesson", width: proportional(3) },
  { key: "kind", header: "Kind", width: proportional(1) },
  { key: "source", header: "Source", width: proportional(2) },
  { key: "updated", header: "Updated", width: proportional(1) },
];

// Placeholder branches of the tree on the left.
const SOURCES = [
  { label: "Source A", topics: ["Topic 1", "Topic 2"] },
  { label: "Source B", topics: ["Topic 3"] },
];

// Placeholder summary tiles above the table.
const SUMMARY = [
  { label: "Lessons", value: "—", icon: BookOpen },
  { label: "Sources", value: "—", icon: FolderGit2 },
  { label: "Last update", value: "—", icon: Clock },
];

const styles = stylex.create({
  // AppShell fills the viewport by default; here it sits under Studio's
  // own title bar, so it fills its parent instead.
  shell: {
    height: "100%",
  },
  row: {
    cursor: "default",
  },
  selectedRow: {
    boxShadow: `inset 0 0 0 100vmax ${colorVars["--color-accent-muted"]}`,
  },
});

/** A click on a row selects it; the selected row is washed with the accent. */
function useRowSelection(
  selectedId: string | null,
  onSelect: (id: string) => void,
): TablePlugin<LessonRow> {
  return useMemo(
    () => ({
      transformBodyRow: (props, item) => ({
        ...props,
        xstyle: [
          ...props.xstyle,
          styles.row,
          item.id === selectedId && styles.selectedRow,
        ],
        htmlProps: {
          ...props.htmlProps,
          "aria-selected": item.id === selectedId ? true : undefined,
          onClick: () => onSelect(item.id),
        },
      }),
    }),
    [selectedId, onSelect],
  );
}

function SummaryTile({ label, value, icon }: (typeof SUMMARY)[number]) {
  return (
    <Card padding={4}>
      <VStack gap={2}>
        <HStack gap={2} vAlign="center">
          <Icon icon={icon} size="sm" color="secondary" />
          <Text color="secondary">{label}</Text>
        </HStack>
        <Heading level={2}>{value}</Heading>
      </VStack>
    </Card>
  );
}

function LessonDetails({
  lesson,
  onClose,
}: {
  lesson: LessonRow;
  onClose: () => void;
}) {
  return (
    <Card padding={4}>
      <VStack gap={4}>
        <HStack gap={2} vAlign="center" hAlign="between">
          <Heading level={3}>{lesson.title}</Heading>
          <IconButton
            label="Close details"
            size="sm"
            variant="ghost"
            icon={<Icon icon={X} size="sm" />}
            onClick={onClose}
          />
        </HStack>
        <Text color="secondary">The details of the Lesson go here.</Text>
        <Divider />
        <Card variant="muted" padding={0} width="100%" height={96} />
        <Card variant="muted" padding={0} width="100%" height={160} />
        <Card variant="muted" padding={0} width="100%" height={64} />
      </VStack>
    </Card>
  );
}

/** The Lessons of the open workspace: a tree, a table, and the details of the selected Lesson. */
export function LessonsScreen({ workspace }: { workspace: OpenWorkspace }) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedTopic, setSelectedTopic] = useState<string | null>(null);
  const selection = useRowSelection(selectedId, setSelectedId);
  const selected = ROWS.find((row) => row.id === selectedId) ?? null;

  return (
    <AppShell
      contentPadding={0}
      xstyle={styles.shell}
      sideNav={
        <SideNav
          collapsible
          resizable={{ defaultWidth: 280, minWidth: 220, maxWidth: 420 }}
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
              isSelected
              onClick={() => {}}
            />
            <SideNavItem label="Recent" icon={Clock} onClick={() => {}} />
          </SideNavSection>
          <SideNavSection title="Sources">
            {/* TreeList draws the guide lines of the nesting itself. */}
            <TreeList
              density="compact"
              variant="lineGuides"
              items={SOURCES.map((source) => ({
                id: source.label,
                label: source.label,
                startContent: <Icon icon={FolderGit2} size="sm" />,
                isExpanded: true,
                children: source.topics.map((topic) => ({
                  id: `${source.label}/${topic}`,
                  label: topic,
                  startContent: <Icon icon={Layers} size="sm" />,
                  isSelected: topic === selectedTopic,
                  onClick: () => setSelectedTopic(topic),
                })),
              }))}
            />
          </SideNavSection>
        </SideNav>
      }
    >
      <Layout
        height="fill"
        header={
          <Toolbar
            label="Lessons"
            size="sm"
            dividers={["bottom"]}
            startContent={
              <Breadcrumbs label="Location">
                <BreadcrumbItem onClick={() => {}}>Lessons</BreadcrumbItem>
                <BreadcrumbItem isCurrent>All Lessons</BreadcrumbItem>
              </Breadcrumbs>
            }
          />
        }
        content={
          <LayoutContent padding={4}>
            <VStack gap={4}>
              <Grid columns={3} gap={4}>
                {SUMMARY.map((tile) => (
                  <SummaryTile key={tile.label} {...tile} />
                ))}
              </Grid>
              <Card padding={0}>
                <Table
                  data={ROWS}
                  columns={COLUMNS}
                  idKey="id"
                  density="compact"
                  hasHover
                  textOverflow="truncate"
                  plugins={{ selection }}
                />
              </Card>
            </VStack>
          </LayoutContent>
        }
        end={
          selected ? (
            <LayoutPanel
              width={360}
              padding={4}
              role="complementary"
              label="Lesson details"
            >
              <LessonDetails
                lesson={selected}
                onClose={() => setSelectedId(null)}
              />
            </LayoutPanel>
          ) : undefined
        }
      />
    </AppShell>
  );
}
