// Writing on several devices: copies left by sync programs, sync folders, drives and outside changes.

import type { Section } from './docs';

/** A copy of a file left by a sync program (crates/core/src/copies.rs). */
export interface CopyInfo {
  /** File name in its folder, e.g. `k7q2m9x4t1ab-DESKTOP-1AB2C3D.md`. */
  file: string;
  section: Section;
  /** Id of the document, card or note it is a copy of. */
  of: string;
  title: string;
  modified: string | null;
  chars: number;
  /** The device it came from, when the sync program named it. */
  device: string | null;
}

/** take: the copy replaces the original; discard: it goes to the trash; keepBoth: it becomes its own. */
export type CopyAction = 'take' | 'discard' | 'keepBoth';

/** Where projects are kept (crates/core/src/places.rs). */
export type Service = 'onedrive' | 'googledrive' | 'dropbox' | 'icloud' | 'local';

export interface Place {
  service: Service;
  label: string;
  /** The folder the program keeps in step. */
  root: string;
  /** Where new projects go in this place. */
  suggested: string;
}

/** Drives the app can keep projects in step with directly (crates/sync). */
export type DriveProvider = 'google' | 'onedrive' | 'dropbox';

export interface DriveAccount {
  name: string;
  email: string;
}

export interface DriveInfo {
  provider: DriveProvider;
  label: string;
  /** The app is registered with this drive, so it can sign in. */
  registered: boolean;
  account: DriveAccount | null;
  connectedAt: string | null;
}

export interface DriveStatus {
  drives: DriveInfo[];
  /** Where the app's registrations with the drives are read from. */
  appsFile: string;
}

/** A project kept in step with a folder on a drive. */
export interface DriveLink {
  provider: DriveProvider;
  folder: string;
  linkedAt: string;
  syncedAt: string | null;
  /** Why the last pass stopped; null when it went through. */
  error: string | null;
}

export interface SyncReport {
  uploaded: string[];
  downloaded: string[];
  removedHere: string[];
  removedThere: string[];
  copies: string[];
  merged: boolean;
  later: string[];
  /** Room left on the drive in bytes when it is running low; null when there is room or the drive does not say. */
  spaceLeft: number | null;
}

export interface SyncOutcome {
  link: DriveLink;
  report: SyncReport | null;
}

export interface DriveProject {
  folder: string;
  title: string;
  id: string;
}

/** A change in the project folder the app did not make (crates/core/src/changes.rs). */
export type Change =
  | { kind: 'project' }
  | { kind: 'projectCopy' }
  | { kind: 'doc'; id: string; rev: string | null }
  | { kind: 'card'; id: string }
  | { kind: 'note'; id: string }
  | { kind: 'records'; docId: string }
  | { kind: 'trash' };
