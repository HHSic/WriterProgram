// New versions of the app (app/src-tauri/src/commands/update.rs).

export interface UpdateInfo {
  version: string;
  current: string;
  /** What changed, as written in the release. */
  notes: string | null;
  /** When the release was made (RFC 3339). */
  date: string | null;
}

/** How far the download of a new version has come. */
export interface UpdateProgress {
  received: number;
  total: number | null;
}
