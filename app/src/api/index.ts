import { mockBackend } from './mock';
import { tauriBackend } from './tauri';
import type { Backend } from './types';

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** The Rust side in the desktop app; an in-memory stand-in in a plain browser. */
export const api: Backend = inTauri ? tauriBackend : mockBackend;
