// The in-app browser and the window, which only the desktop app has.

import type { Backend } from '../types';

export const browserMethods = {
  async browserOpen() {},
  async browserBounds() {},
  async browserNavigate(_label, url) {
    return url;
  },
  async browserStep() {},
  async browserClose() {},
  async browserClip() {
    throw '앱 안 브라우저는 데스크톱 앱에서만 됨';
  },
  onBrowserPage() {
    return () => {};
  },
  onCloseRequested() {},
  setWindowTitle(title) {
    document.title = title;
  },
  async appVersion() {
    return '0.1.0';
  },
  // The browser stand-in has no releases to look at.
  async updateCheck() {
    return null;
  },
  async updateInstall() {},
  onUpdateProgress() {
    return () => {};
  },
} satisfies Partial<Backend>;
