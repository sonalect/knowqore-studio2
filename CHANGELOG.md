# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
While the major version is 0, a breaking change bumps the minor version.

## [Unreleased]

### Added

- Choosing a source in the tree groups the Lessons table by the next level
  below it (hosts under a kind, owners under a host, repositories under an
  owner). Each group has a header with its label and the number of Lessons
  that pass the filters, and folds shut with its chevron or a click; the folds
  are kept while the source stays chosen. A Lesson seen in two groups is
  listed in both; the result count above the table counts each Lesson once.
  Choosing another source closes the details card.
- The details card reads the selected Lesson from the store and has three
  tabs. Overview shows its statement in full, highlighted as Markdown, with
  the fields of the table: kind, status, polarity, transfer, updated, and id.
  YAML shows the whole record as the contract gives it, highlighted, with
  long lines wrapped under their own indent. Every Lesson opens on Overview,
  and the card changes in one step, without flashing an interim state.
- The History tab of the details lists what happened to the Lesson, newest
  first, read from the store with the Lesson, so the tab label shows the
  count before the tab is opened. The writes are grouped by the run that
  made them on one day, with the day, a colored `learn` or
  `ingest` chip, and the run; each write shows colored events (created,
  revision with the old and new statement, status, curator, containers,
  evidence, your choice, amended by an agent, and more) and the tab label
  counts them. Writes that changed nothing are hidden until a switch shows
  them. A click on a write's id opens a dialog, headed by the write's note,
  with the YAML diff of the Lesson between that write and the version
  before it.
- A divider between the table and the details card drags the card wider or
  narrower. The card starts at 40% of the two, and its width is kept for the
  next start, as is the width of the side navigation.
- A filter bar above the Lessons table: Kind, Status, Transfer, and Polarity
  each take several values (a Lesson matches any value of one filter and all
  filters). Every menu shows how many Lessons under the chosen source have
  each value, the bar shows the number of results, and "Clear all" empties
  the filters. Controls that do not fit fold behind a "+N" button.
- Dates follow the system's regional format and local time zone, as in
  native apps, not the interface language.
- A retired Lesson's statement is struck through in the table.
- The columns of the Lessons table are resized by dragging their borders,
  and their widths are kept for the next start; a table wider than its card
  scrolls sideways. Until the person sets it, the Lesson column takes the
  room the other columns leave.
- The Lessons table shows the status as a badge and the kind with a colored
  dot, has a Polarity column, and puts the Lesson id under its statement.
- The application skeleton on Tauri 2, React, and TypeScript (Vite, bun):
  `bun run tauri dev` opens the template window.
- Every dependency pinned to an exact version, TypeScript 7 included.
- `bun run check`, the whole gate in one command: Biome and markdownlint,
  the TypeScript check and the Vite build, then `cargo test`,
  `cargo clippy`, and `cargo fmt --check`. `bun run format` fixes what the
  formatters can.
- `bun install` keeps the two-day quarantine on npm packages
  (`minimumReleaseAge` in `bunfig.toml`).
- The Astryx design system (0.7.3, Neutral theme) draws the window.
- At startup Studio opens the workspace at its default location
  (`~/.knowqore` on Linux and macOS, `%LOCALAPPDATA%\Knowqore` on Windows)
  through the store, read-only. When it is missing or the store refuses it,
  an open screen says why and takes the folder to open instead.
- A first skeleton of the Lessons screen: a side navigation with labeled
  groups and icons, a header with breadcrumbs, the table of
  Lessons in a card, and a details card for the clicked row.
- The Lessons table shows the Vault's Lessons, newest first, sortable by
  every column. It reads only the rows in view as the person scrolls, so a
  Vault of any size stays responsive, and the details card shows the
  clicked Lesson.
- The side navigation shows the Vault's sources as a tree (kind, host,
  owner, repository), each with the number of Lessons seen under it.
  Choosing one lists only its Lessons, and the "Sources" tile counts the
  Vault's containers. "All Lessons" shows every Lesson again.
- A title bar that looks native on each system: the system's own on macOS;
  a Windows 11 caption bar on the Mica backdrop on Windows, with Snap
  Layouts on its maximize button; on Linux a bar of Studio's own with
  GNOME-style round buttons, placed as the desktop's button layout says.
- The theme follows the system's light or dark mode and its accent color,
  on Windows with the Mica backdrop as well. On Linux the interface uses the
  desktop's font. Buttons and other accented controls take the system's
  exact accent; a change in the system settings shows when the window
  regains focus.
- The mouse pointer stays an arrow over buttons, links, and other clickable
  items, as in a desktop app, instead of the web's pointing hand.
- No white flash at startup or on resize: the window opens in the theme's
  background color even before the page has drawn.
- The window reopens where it was left, and is pulled back onto a visible
  display if a monitor was unplugged. A window closed maximized reopens
  maximized, and Restore returns it to the size it had before.
- mimalloc as the Rust global allocator.
- The Astryx CLI (`bun run astryx`) and its agent guide in
  `.claude/rules/astryx.md`.
- Studio builds with Knowqore's store: the contract `knowqore-stores` and
  its implementation `knowqore-store-scheda`, from one Knowqore commit.
  `tokio` follows Knowqore's pin to 1.54.0.
- The StyleX compiler for Studio's own styles, so screens can start from
  Astryx templates that use StyleX; the title bar is styled with it.
