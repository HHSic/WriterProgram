// Saving a card or a note a moment after the last change, one save after another.

import { useEffect, useRef, type DependencyList } from 'react';
import { registerFlusher } from './flush';

export interface DebouncedSave<T> {
  /** Writes one value. Runs after the previous write has finished. */
  save: (value: T) => Promise<void>;
  /** The write failed; the value is kept to send again with the next change or flush. */
  onError: (e: unknown) => void;
  /**
   * Turns the value into what is written, when the flush starts (not when its
   * turn in the chain comes), e.g. to fill in a name left empty.
   */
  prepare?: (value: T) => T;
}

/**
 * The latest unsaved value waits in `pending` until `schedule`'s delay runs
 * out, the project flushes (switching documents, closing the window), or the
 * screen goes away; saves run one after another, like the manuscript's
 * (editor/session.ts). The options are taken as they were when `deps` last
 * changed, so leaving a project still saves into that project.
 */
export function useDebouncedSave<T>(options: DebouncedSave<T>, deps: DependencyList) {
  const pending = useRef<T | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const flushRef = useRef<() => Promise<void>>(async () => {});

  useEffect(() => {
    const { save, onError, prepare } = options;
    let chain: Promise<void> = Promise.resolve();
    const flush = () => {
      clearTimeout(timer.current);
      const next = pending.current;
      if (!next) return chain;
      pending.current = null;
      const toSave = prepare ? prepare(next) : next;
      chain = chain.then(async () => {
        try {
          await save(toSave);
        } catch (e) {
          // Keep the value marked unsaved so the next change or retry sends it again.
          pending.current ??= next;
          onError(e);
        }
      });
      return chain;
    };
    flushRef.current = flush;
    const unregister = registerFlusher(flush);
    return () => {
      unregister();
      void flush();
    };
  }, deps); // eslint-disable-line react-hooks/exhaustive-deps

  /** Holds `value` as the one to save and saves it after `delay` ms without another change. */
  const schedule = (value: T, delay = 600) => {
    pending.current = value;
    clearTimeout(timer.current);
    timer.current = setTimeout(() => void flushRef.current(), delay);
  };

  return { pending, schedule };
}
