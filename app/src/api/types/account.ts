import type { ViewSettings } from '../../lib/view';
import type { BookPreferences } from '../../lib/book';

export interface AccountIdentity { id: string; email: string; name: string }
export interface AccountStatus {
  url: string | null;
  account: AccountIdentity | null;
}
export type AccountLogin =
  | { kind: 'oauth'; provider: 'google' | 'kakao' }
  | { kind: 'sso'; domain: string }
  | { kind: 'password' | 'signup'; email: string; password: string };
export interface AccountLoginResult { account: AccountIdentity | null; confirmationRequired: boolean }
export interface RibbonPreferences { tab: '편집' | '서식' | '보기' | '쪽' | '검토'; collapsed: boolean }
/** Only portable preferences travel: voices and reader device mode stay local. */
export interface PersonalSettings {
  view?: Partial<Omit<ViewSettings, 'readVoice'>>;
  ribbon?: Partial<RibbonPreferences>;
  book?: Partial<Omit<BookPreferences, 'device'>>;
}
export interface SettingsSnapshot { revision: number; settings: PersonalSettings; updatedAt: string | null }
export interface SettingsWrite { conflict: boolean; snapshot: SettingsSnapshot }
