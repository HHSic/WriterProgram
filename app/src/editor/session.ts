// Autosave for the open document: saves after a short pause in typing, and at
// least every few seconds while typing continues. Saves for one document run
// one after another, never at the same time.

import type { JSONContent } from '@tiptap/core';
import { api } from '../api';
import type { SaveOutcome } from '../api/types';
import { errorText } from '../lib/format';
import { registerFlusher } from '../lib/flush';
import { useApp } from '../store';

const IDLE_MS = 800;
const MAX_WAIT_MS = 5000;

export class SaveSession {
  private idleTimer: ReturnType<typeof setTimeout> | undefined;
  private maxTimer: ReturnType<typeof setTimeout> | undefined;
  private dirty = false;
  private chain: Promise<void> = Promise.resolve();
  private readonly unregister: () => void;

  constructor(
    private readonly root: string,
    private readonly docId: string,
    private readonly getBody: () => JSONContent,
    private readonly onSaved: (outcome: SaveOutcome) => void,
  ) {
    this.unregister = registerFlusher(() => this.flush());
  }

  changed() {
    this.dirty = true;
    if (useApp.getState().save.state === 'saved') useApp.setState({ save: { state: 'saving' } });
    clearTimeout(this.idleTimer);
    this.idleTimer = setTimeout(() => void this.flush(), IDLE_MS);
    this.maxTimer ??= setTimeout(() => void this.flush(), MAX_WAIT_MS);
  }

  flush(): Promise<void> {
    clearTimeout(this.idleTimer);
    clearTimeout(this.maxTimer);
    this.idleTimer = undefined;
    this.maxTimer = undefined;
    if (!this.dirty) return this.chain;
    this.dirty = false;
    const body = this.getBody();
    this.chain = this.chain.then(async () => {
      try {
        const outcome = await api.docSave(this.root, this.docId, body);
        if (!this.dirty) useApp.setState({ save: { state: 'saved' } });
        this.onSaved(outcome);
      } catch (e) {
        // Keep the text marked unsaved so the next change or retry sends it again.
        this.dirty = true;
        useApp.setState({ save: { state: 'error', error: errorText(e) } });
      }
    });
    return this.chain;
  }

  async dispose() {
    await this.flush();
    this.unregister();
  }
}
