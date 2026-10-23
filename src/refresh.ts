import { useCallback, useEffect, useRef, useState } from "react";
import { reloadLessons } from "./lessons";

/** How often the data is read again, in minutes; 0 is never, only by the button. */
export type RefreshMinutes = 0 | 5 | 15 | 30;

/** The choices of the interval, in the order the select lists them. */
export const REFRESH_CHOICES: { minutes: RefreshMinutes; label: string }[] = [
  { minutes: 5, label: "Every 5 minutes" },
  { minutes: 15, label: "Every 15 minutes" },
  { minutes: 30, label: "Every 30 minutes" },
  { minutes: 0, label: "Manual" },
];

const DEFAULT_MINUTES: RefreshMinutes = 15;

/** Where the chosen interval is kept, in the webview's storage. */
const INTERVAL_KEY = "refresh-minutes";

/** A refresh spins its icon for at least this long, so a quick one is still seen. */
const MIN_SPIN_MS = 700;

const MS_PER_MINUTE = 60_000;

/** The interval kept from the last time, or the default when storage cannot be read. */
export function readRefreshMinutes(): RefreshMinutes {
  try {
    const kept = localStorage.getItem(INTERVAL_KEY);
    const found = REFRESH_CHOICES.find(
      (choice) => String(choice.minutes) === kept,
    );
    return found?.minutes ?? DEFAULT_MINUTES;
  } catch {
    return DEFAULT_MINUTES;
  }
}

/** Keeps the interval for the next start; a failure only loses it. */
function keepRefreshMinutes(minutes: RefreshMinutes) {
  try {
    localStorage.setItem(INTERVAL_KEY, String(minutes));
  } catch {
    // Storage is blocked or full: the choice holds for this session only.
  }
}

/** What the screens and the controls share about reading the data again. */
export interface Refresh {
  /** Bumped after every successful refresh: screens read their data again when it changes. */
  version: number;
  isRefreshing: boolean;
  /** The store's words when the last refresh failed; `null` otherwise. */
  error: string | null;
  minutes: RefreshMinutes;
  setMinutes: (minutes: RefreshMinutes) => void;
  /** Reads the data again now; does nothing while a refresh runs. */
  refresh: () => void;
}

/**
 * Reads the data again every `minutes` while `enabled`, and on demand. The
 * store has no change notification yet (`.claude/notes/store-calls-wanted.md`).
 * A tick that falls in a running refresh is skipped.
 */
export function useRefresh(enabled: boolean): Refresh {
  const [minutes, setMinutesState] = useState(readRefreshMinutes);
  const [version, setVersion] = useState(0);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const running = useRef(false);

  const refresh = useCallback(() => {
    if (running.current) return;
    running.current = true;
    setIsRefreshing(true);
    const started = Date.now();
    reloadLessons()
      .then(
        () => {
          setError(null);
          setVersion((was) => was + 1);
        },
        (reason: unknown) => setError(String(reason)),
      )
      .finally(() => {
        const left = Math.max(MIN_SPIN_MS - (Date.now() - started), 0);
        setTimeout(() => {
          running.current = false;
          setIsRefreshing(false);
        }, left);
      });
  }, []);

  useEffect(() => {
    if (!enabled || minutes === 0) return;
    const timer = setInterval(refresh, minutes * MS_PER_MINUTE);
    return () => clearInterval(timer);
  }, [enabled, minutes, refresh]);

  const setMinutes = useCallback((next: RefreshMinutes) => {
    keepRefreshMinutes(next);
    setMinutesState(next);
  }, []);

  return { version, isRefreshing, error, minutes, setMinutes, refresh };
}
