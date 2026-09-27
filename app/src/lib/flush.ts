// Everything that holds unsaved text registers a flusher here, so switching
// documents, leaving a project or closing the window can wait for the last
// save.

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
