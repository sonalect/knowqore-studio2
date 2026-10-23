import { Icon } from "@astryxdesign/core/Icon";
import { IconButton } from "@astryxdesign/core/IconButton";
import { Selector } from "@astryxdesign/core/Selector";
import { HStack } from "@astryxdesign/core/Stack";
import { spacingVars } from "@astryxdesign/core/theme/tokens.stylex";
import * as stylex from "@stylexjs/stylex";
import { RefreshCw } from "lucide-react";
import { REFRESH_CHOICES, type Refresh, type RefreshMinutes } from "../refresh";

const spin = stylex.keyframes({
  from: { transform: "rotate(0deg)" },
  to: { transform: "rotate(360deg)" },
});

const styles = stylex.create({
  group: {
    paddingInline: spacingVars["--spacing-2"],
  },
  // A spinning icon is motion; with reduced motion it stands still and the
  // button's busy state says the rest.
  spinning: {
    animationName: {
      default: spin,
      "@media (prefers-reduced-motion: reduce)": "none",
    },
    animationDuration: "1s",
    animationTimingFunction: "linear",
    animationIterationCount: "infinite",
  },
});

const OPTIONS = REFRESH_CHOICES.map(({ minutes, label }) => ({
  value: String(minutes),
  label,
}));

function isMinutes(value: string): RefreshMinutes | null {
  return (
    REFRESH_CHOICES.find((choice) => String(choice.minutes) === value)
      ?.minutes ?? null
  );
}

/**
 * How often the data is read again, and a button that does it now. The icon
 * spins while a refresh runs; a failed one turns the icon red and the
 * button's tooltip into the store's words.
 */
export function RefreshControls({ refresh }: { refresh: Refresh }) {
  const { isRefreshing, error, minutes, setMinutes } = refresh;
  return (
    <HStack gap={1} vAlign="center" xstyle={styles.group}>
      <IconButton
        label="Refresh"
        size="sm"
        variant="ghost"
        tooltip={error ?? "Read the data again"}
        icon={
          <Icon
            icon={RefreshCw}
            size="sm"
            color={error === null ? "inherit" : "error"}
            xstyle={isRefreshing ? styles.spinning : undefined}
          />
        }
        onClick={refresh.refresh}
      />
      <Selector
        label="Refresh interval"
        isLabelHidden
        size="sm"
        variant="ghost"
        options={OPTIONS}
        value={String(minutes)}
        onChange={(value) => {
          const next = isMinutes(value);
          if (next !== null) setMinutes(next);
        }}
      />
    </HStack>
  );
}
