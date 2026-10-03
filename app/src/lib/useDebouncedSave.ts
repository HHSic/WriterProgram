// Saving a card, a note or a chapter's title a moment after the last change,
// one save after another.

import { useEffect, useRef, type DependencyList } from 'react';
import type { RescueContent } from '../api/types';
import { markFailed, markSaved, markSaving, registerSave } from '../store/saves';
import { registerFlusher } from './flush';

export interface DebouncedSave<T> {
  /** The save target's key in store/saves.ts, e.g. `card:<id>`. */
  key: string;
  /** Writes one value. Runs after the previous write has finished. */
  save: (value: T) => Promise<void>;
  /** The write failed; the value is kept to send again (store/saves.ts tries again later). */
  onError?: (e: unknown) => void;
  /** A write succeeded. */
  onSaved?: () => void;
  /**
   * Turns the value into what is written, when the flush starts (not when its
   * turn in the chain comes), e.g. to fill in a name left empty.
   */
  prepare?: (value: T) => T;
  /**
   * A value that failed to save, joined with one changed since (`newer`, if
   * any). By default the newer value replaces it: cards and notes are saved
   * whole. Partial patches (title, synopsis) merge.
   */
  merge?: (failed: T, newer: T | null) => T;
  /** What to keep in the rescue folder when saving keeps failing. */
  rescue?: (value: T) => { item: string; content: RescueContent };
}

/** Holds the waiting value and sends it; split from the hook so it can be tested without React. */
export function debouncedSaver<T>(options: DebouncedSave<T>) {
  const { key, save, onError, onSaved, prepare, merge, rescue } = options;
  let pending: T | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let chain: Promise<void> = Promise.resolve();
  /** Sends under way. */
  let sending = 0;

  const flush = (): Promise<void> => {
    clearTimeout(timer);
    timer = undefined;
    const next = pending;
    if (next === null) {
      // Nothing waits (e.g. the failed value was sent by another flush meanwhile).
      return chain;
    }
    pending = null;
    sending += 1;
    const toSave = prepare ? prepare(next) : next;
    chain = chain.then(async () => {
      try {
        await save(toSave);
        sending -= 1;
        // Cleared only now: a value changed while this one was on its way waits its own turn.
        if (pending === null && sending === 0) markSaved(key);
        onSaved?.();
      } catch (e) {
        sending -= 1;
        // Never drop an edit: put the failed value back under anything newer.
        pending = merge ? merge(next, pending) : (pending ?? next);
        markFailed(key, e);
        onError?.(e);
      }
    });
    return chain;
  };

  const target = {
    retry: () => (pending === null && sending === 0 ? chain.then(() => markSaved(key)) : flush()),
    pending: () => pending !== null || sending > 0,
    rescue: () => {
      if (!rescue || pending === null) return null;
      return rescue(pending);
    },
  };

  return {
    flush,
    target,
    /** The value waiting to be saved, if any. */
    get pending(): T | null {
      return pending;
    },
    /** Holds `value` as the one to save and saves it after `delay` ms without another change. */
    schedule(value: T, delay = 600) {
      pending = value;
      markSaving(key);
      clearTimeout(timer);
      timer = setTimeout(() => void flush(), delay);
    },
  };
}

/**
 * The latest unsaved value waits in `pending` until `schedule`'s delay runs
 * out, the project flushes (switching documents, closing the window), or the
 * screen goes away; saves run one after another, like the manuscript's
 * (editor/session.ts). The options are taken as they were when `deps` last
 * changed, so leaving a project still saves into that project. A value that
 * could not be saved stays with store/saves.ts, which keeps trying, after the
 * screen has gone away.
 */
export function useDebouncedSave<T>(options: DebouncedSave<T>, deps: DependencyList) {
  const saver = useRef<ReturnType<typeof debouncedSaver<T>> | null>(null);
  const optionsRef = useRef(options);
  optionsRef.current = options;

  useEffect(() => {
    const s = debouncedSaver<T>({
      ...optionsRef.current,
      // Read the latest callbacks: they may close over the screen's state.
      onError: (e) => optionsRef.current.onError?.(e),
      onSaved: () => optionsRef.current.onSaved?.(),
    });
    saver.current = s;
    const unregisterFlush = registerFlusher(s.flush);
    const unregisterSave = registerSave(optionsRef.current.key, s.target);
    return () => {
      unregisterFlush();
      void s.flush();
      unregisterSave();
    };
  }, deps); // eslint-disable-line react-hooks/exhaustive-deps

  // `pending.current` mirrors the saver's waiting value for the screen's checks.
  const view = {
    get current(): T | null {
      return saver.current?.pending ?? null;
    },
  };

  const schedule = (value: T, delay = 600) => saver.current?.schedule(value, delay);

  return { pending: view, schedule };
}
