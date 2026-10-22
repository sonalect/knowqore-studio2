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
import { HStack, StackItem, VStack } from "@astryxdesign/core/Stack";
import { Text } from "@astryxdesign/core/Text";
import { Toolbar } from "@astryxdesign/core/Toolbar";
import { TreeList } from "@astryxdesign/core/TreeList";
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
import { useState } from "react";
import { formatUpdated, type LessonRow } from "../lessons";
import type { OpenWorkspace } from "../workspace";
import { LessonsTable } from "./LessonsTable";

// Placeholder branches of the tree on the left.
const SOURCES = [
  { label: "Source A", topics: ["Topic 1", "Topic 2"] },
  { label: "Source B", topics: ["Topic 3"] },
];

const NUMBER = new Intl.NumberFormat();

// Summary tiles above the table; only the Lessons count is real yet.
function summary(total: number | null) {
  return [
    {
      label: "Lessons",
      value: total === null ? "—" : NUMBER.format(total),
      icon: BookOpen,
    },
    { label: "Sources", value: "—", icon: FolderGit2 },
    { label: "Last update", value: "—", icon: Clock },
  ];
}

const styles = stylex.create({
  // AppShell fills the viewport by default; here it sits under Studio's
  // own title bar, so it fills its parent instead.
  shell: {
    height: "100%",
  },
});

function SummaryTile({
  label,
  value,
  icon,
}: ReturnType<typeof summary>[number]) {
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

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <VStack gap={0.5}>
      <Text color="secondary">{label}</Text>
      <Text>{value}</Text>
    </VStack>
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
          <Heading level={3}>Lesson</Heading>
          <IconButton
            label="Close details"
            size="sm"
            variant="ghost"
            icon={<Icon icon={X} size="sm" />}
            onClick={onClose}
          />
        </HStack>
        <Text>{lesson.statement}</Text>
        <Divider />
        <Fact label="Kind" value={lesson.kind} />
        <Fact label="Status" value={lesson.status} />
        <Fact label="Transfer" value={lesson.transfer} />
        <Fact label="Polarity" value={lesson.polarity} />
        <Fact label="Updated" value={formatUpdated(lesson.updatedAt)} />
        <Fact label="Id" value={lesson.id} />
      </VStack>
    </Card>
  );
}

/** The Lessons of the open workspace: a tree, a table, and the details of the selected Lesson. */
export function LessonsScreen({ workspace }: { workspace: OpenWorkspace }) {
  const [selected, setSelected] = useState<LessonRow | null>(null);
  const [total, setTotal] = useState<number | null>(null);
  const [selectedTopic, setSelectedTopic] = useState<string | null>(null);

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
          <LayoutContent padding={4} isScrollable={false}>
            <VStack gap={4} height="100%">
              <Grid columns={3} gap={4}>
                {summary(total).map((tile) => (
                  <SummaryTile key={tile.label} {...tile} />
                ))}
              </Grid>
              <StackItem size="fill">
                <Card padding={0} height="100%">
                  <LessonsTable
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
            <LayoutPanel
              width={360}
              padding={4}
              role="complementary"
              label="Lesson details"
            >
              <LessonDetails
                lesson={selected}
                onClose={() => setSelected(null)}
              />
            </LayoutPanel>
          ) : undefined
        }
      />
    </AppShell>
  );
}
