import type { ReactNode } from 'react';

const PATHS = {
  chevronDown: <path d="M6 9l6 6 6-6" />,
  chevronRight: <path d="M9 6l6 6-6 6" />,
  plus: <path d="M12 5v14M5 12h14" />,
  trash: <path d="M5 7h14M10 7V4h4v3M7 7l1 13h8l1-13" />,
  doc: (
    <>
      <path d="M6 3h9l4 4v14H6z" />
      <path d="M9 12h7M9 16h5" />
    </>
  ),
  pencil: <path d="M4 20h4L19 9l-4-4L4 16z" />,
  folder: <path d="M3 7h6l2 2h10v10H3z" />,
  download: <path d="M12 4v11M7 10l5 5 5-5M5 20h14" />,
  close: <path d="M6 6l12 12M18 6L6 18" />,
  more: (
    <>
      <circle cx="5" cy="12" r="1.2" />
      <circle cx="12" cy="12" r="1.2" />
      <circle cx="19" cy="12" r="1.2" />
    </>
  ),
  swap: <path d="M8 9l4-4 4 4M8 15l4 4 4-4" />,
  panel: (
    <>
      <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
      <path d="M15 4.5v15" />
    </>
  ),
  clock: (
    <>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 8v4l3 2" />
    </>
  ),
  type: <path d="M5 19l5-14h1l5 14M7 14h7M17 19l2.5-7 2.5 7M18 17h3" />,
  diamond: <path d="M12 4l6 8-6 8-6-8z" />,
  back: <path d="M15 6l-6 6 6 6" />,
  restore: (
    <>
      <path d="M4 12a8 8 0 1 0 2.4-5.7" />
      <path d="M4 4v4h4" />
    </>
  ),
  copy: (
    <>
      <rect x="8" y="8" width="12" height="12" rx="2" />
      <path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3" />
    </>
  ),
} satisfies Record<string, ReactNode>;

export type IconName = keyof typeof PATHS;

export function Icon({ name, size = 16 }: { name: IconName; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {PATHS[name]}
    </svg>
  );
}
