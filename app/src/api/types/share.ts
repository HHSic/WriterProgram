import type { DriveProvider } from './devices';

export interface ShareLink {
  id: string;
  url: string;
  title: string;
  chapterCount: number;
  createdAt: string;
  provider: DriveProvider;
  account: string;
}
