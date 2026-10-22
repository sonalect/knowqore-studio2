# Styling: Astryx first, StyleX for our own

Studio styles its screens in this order (owner, 22 October 2026):

1. Astryx components and their props.
2. Our own styles with StyleX: `stylex.create` next to the component, with
   the typed tokens of `@astryxdesign/core/theme/tokens.stylex`
   (`colorVars["--color-text-primary"]`, `spacingVars["--spacing-3"]`). A
   token missing from the typed exports is written as a string,
   `"var(--text-supporting-size)"`.
3. Plain CSS (`src/index.css`) only for what StyleX cannot express: the
   page itself (`html`, `body`, `#root`) and rules that need descendant or
   global selectors.

The compiler is `@stylexjs/unplugin` (`stylex.vite()` in `vite.config.ts`).
It covers dev and build, so StyleX page templates and swizzled Astryx
components work as they are. Astryx itself stays prebuilt: keep importing
its stylesheets, never compile its sources.

This overrides the line "No StyleX/Tailwind compiler here" in the generated
`astryx.md`, which `astryx init` and `astryx upgrade` write back.

```tsx
// BAD — a new class in index.css for one component
<header className="status-bar">

// GOOD
const styles = stylex.create({
  bar: { color: colorVars["--color-text-primary"] },
});
<header {...stylex.props(styles.bar)}>
```
