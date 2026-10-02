// Everything that holds unsaved text registers a flusher here, so switching
// documents, leaving a project or closing the window can wait for the last
// save. Closers run only when the project or the window closes (not on every
// Ctrl+S or tab switch): the creation journal ends its writing sessions there.

type Flusher = () => Promise<void>;

const flushers = new Set<Flusher>();

export function registerFlusher(fn: Flusher): () => void {
  flushers.add(fn);
  return () => {
    flushers.delete(fn);
  };
}

export async function flushAll(): Promise<void> {
  await Promise.all([...flushers].map((fn) => fn()));
}

const closers = new Set<Flusher>();

export function registerCloser(fn: Flusher): () => void {
  closers.add(fn);
  return () => {
    closers.delete(fn);
  };
}

/** Runs the closers, then waits for every pending save. */
export async function closeAll(): Promise<void> {
  await Promise.all([...closers].map((fn) => fn()));
  await flushAll();
}
