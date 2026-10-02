// In-memory stand-in for the Rust side, used when the screens run in a plain
// browser (`npm run dev` without Tauri). It keeps everything in memory and
// starts with one sample project, so layouts can be checked quickly.
// Each file next to this one holds the part of the backend for one area.

import type { Backend } from '../types';
import { browserMethods } from './browser';
import { cardMethods } from './cards';
import { deviceMethods, otherDevice } from './devices';
import { docMethods } from './docs';
import { driveMethods } from './drives';
import { exchangeMethods } from './exchange';
import { formatMethods } from './formats';
import { ioMethods } from './io';
import { noteMethods } from './notes';
import { projectMethods } from './projects';
import { searchMethods } from './search';
import { seed } from './state';

/**
 * Seeds the sample project and hands out the stand-in. Nothing runs at import
 * time, so a production build can drop this whole folder when it is not
 * called. The parts are joined in here too: spreading them at module level
 * would count as a side effect and keep them in the bundle.
 */
export function startMockBackend(): Backend {
  seed();
  if (typeof window !== 'undefined') (window as unknown as { __otherDevice: typeof otherDevice }).__otherDevice = otherDevice;
  return {
    isDesktop: false,
    ...projectMethods,
    ...docMethods,
    ...ioMethods,
    ...exchangeMethods,
    ...searchMethods,
    ...cardMethods,
    ...noteMethods,
    ...formatMethods,
    ...deviceMethods,
    ...driveMethods,
    ...browserMethods,
  };
}
