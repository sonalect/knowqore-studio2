# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
While the major version is 0, a breaking change bumps the minor version.

## [Unreleased]

### Added

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
  groups and icons, a header with breadcrumbs, summary tiles, the table of
  Lessons in a card, and a details card for the clicked row, all with
  placeholder content.
- A title bar that looks native on each system: the system's own on macOS;
  a Windows 11 caption bar on the Mica backdrop on Windows, with Snap
  Layouts on its maximize button; on Linux a bar of Studio's own with
  GNOME-style round buttons, placed as the desktop's button layout says.
- The theme follows the system's light or dark mode and its accent color,
  on Windows with the Mica backdrop as well. On Linux the interface uses the
  desktop's font. Buttons and other accented controls take the system's
  exact accent; a change in the system settings shows when the window
  regains focus.
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
