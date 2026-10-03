// Autosave for the open document: saves after a short pause in typing, and at
// least every few seconds while typing continues. Saves for one document run
// one after another, never at the same time.
//
// Each save carries the fingerprint of the text the editor started from
// (`base`). When another device changed the file in the meantime, the save is
// refused and the editor's text kept as a record; the session then stops
// saving until the writer picks a version (store.ts: keepMine / takeTheirs).
//
// How saving goes is reported as the target `doc:<id>` (store/saves.ts), which
// tries a failed save again with growing waits and keeps a rescue copy when
// saving keeps failing.

import type { JSONContent } from '@tiptap/core';
import { api } from '../api';
import type { SaveOutcome } from '../api/types';
import { registerFlusher } from '../lib/flush';
import { markConflict } from '../store';
import { markFailed, markSaved, markSaving, registerSave } from '../store/saves';

const IDLE_MS = 800;
const MAX_WAIT_MS = 5000;

export class SaveSession {
  private idleTimer: ReturnType<typeof setTimeout> | undefined;
  private maxTimer: ReturnType<typeof setTimeout> | undefined;
  private dirty = false;
  private chain: Promise<void> = Promise.resolve();
  private readonly unregister: () => void;
  private readonly unregisterSave: () => void;
  private readonly key: string;
  /** Fingerprint of the text on disk that the editor's text is based on. */
  base: string;
  /** Another device changed the text; saving waits for the writer's choice. */
  conflict = false;

  constructor(
    private readonly root: string,
    private readonly docId: string,
    private readonly getBody: () => JSONContent,
    private readonly onSaved: (outcome: SaveOutcome) => void,
    base: string,
  ) {
    this.base = base;
    this.key = `doc:${docId}`;
    this.unregister = registerFlusher(() => this.flush());
    this.unregisterSave = registerSave(this.key, {
      // With nothing left to send (the text was reloaded from disk meanwhile)
      // the failure is over.
      retry: () =>
        this.dirty && !this.conflict
          ? this.flush()
          : this.chain.then(() => {
              if (!this.dirty || this.conflict) markSaved(this.key);
            }),
      pending: () => this.dirty && !this.conflict,
      rescue: () => ({ item: this.docId, content: { body: this.getBody() } }),
    });
  }

  changed() {
    this.dirty = true;
    if (this.conflict) return;
    markSaving(this.key);
    clearTimeout(this.idleTimer);
    this.idleTimer = setTimeout(() => void this.flush(), IDLE_MS);
    this.maxTimer ??= setTimeout(() => void this.flush(), MAX_WAIT_MS);
  }

  /** True while there are edits not sent to disk yet. */
  get pending(): boolean {
    return this.dirty;
  }

  /** Waits for the save under way, if any. */
  settle(): Promise<void> {
    return this.chain;
  }

  flush(): Promise<void> {
    clearTimeout(this.idleTimer);
    clearTimeout(this.maxTimer);
    this.idleTimer = undefined;
    this.maxTimer = undefined;
    if (!this.dirty || this.conflict) return this.chain;
    this.dirty = false;
    const body = this.getBody();
    this.chain = this.chain.then(async () => {
      try {
        const outcome = await api.docSave(this.root, this.docId, body, this.base);
        if (outcome.conflict) {
          // The editor's text is kept as a record; nothing is lost if the
          // window closes now.
          this.conflict = true;
          this.dirty = false;
          markSaved(this.key);
          markConflict(this.docId, outcome.snapshot);
          return;
        }
        this.base = outcome.rev;
        if (!this.dirty) markSaved(this.key);
        this.onSaved(outcome);
      } catch (e) {
        // Keep the text marked unsaved; store/saves.ts sends it again later.
        this.dirty = true;
        markFailed(this.key, e);
      }
    });
    return this.chain;
  }

  /** The editor now shows the text on disk with this fingerprint. */
  loaded(rev: string) {
    clearTimeout(this.idleTimer);
    clearTimeout(this.maxTimer);
    this.idleTimer = undefined;
    this.maxTimer = undefined;
    this.base = rev;
    this.dirty = false;
    this.conflict = false;
    markSaved(this.key);
  }

  /** Saves the editor's text over another device's (kept as a record). */
  async saveOver(): Promise<SaveOutcome> {
    await this.chain;
    const outcome = await api.docSave(this.root, this.docId, this.getBody(), this.base, true);
    this.loaded(outcome.rev);
    markSaved(this.key);
    this.onSaved(outcome);
    return outcome;
  }

  async dispose() {
    await this.flush();
    this.unregister();
    // A text that could not be saved stays with store/saves.ts until it is.
    this.unregisterSave();
  }
}
