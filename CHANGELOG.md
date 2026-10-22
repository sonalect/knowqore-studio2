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
- The Astryx design system (0.7.3, Neutral theme) draws the window: a
  welcome screen in place of the template page.
- A native window and title bar on each system: the system's own on macOS
  and Linux, a Windows 11 caption bar on the Mica backdrop on Windows, with
  Snap Layouts on its maximize button.
- The theme follows the system's light or dark mode, and on Windows its
  accent color and Mica backdrop, while Studio runs.
- The window reopens where it was left, and is pulled back onto a visible
  display if a monitor was unplugged. A window closed maximized reopens
  maximized, and Restore returns it to the size it had before.
- mimalloc as the Rust global allocator.
- The Astryx CLI (`bun run astryx`) and its agent guide in
  `.claude/rules/astryx.md`.
