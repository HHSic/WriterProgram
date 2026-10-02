import { startMockBackend } from './mock';
import { tauriBackend } from './tauri';
import type { Backend } from './types';

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/**
 * The Rust side in the desktop app; an in-memory stand-in in a plain browser.
 * The stand-in only exists in `npm run dev`: a production build folds
 * `import.meta.env.DEV` to false and leaves the whole mock out of the bundle.
 */
export const api: Backend = inTauri ? tauriBackend : import.meta.env.DEV ? startMockBackend() : tauriBackend;
