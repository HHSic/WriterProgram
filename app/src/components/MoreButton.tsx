// ⋯ on a list row: the same menu as a right click or a long press. Shown on
// hover with a mouse, always on touch screens (styles.css .more-btn).

import { Icon } from './Icon';
import { openMenu, type MenuItem, type MenuOptions } from './Menu';

export function MoreButton({
  items,
  opts,
  label,
}: {
  items: () => MenuItem[];
  opts?: () => MenuOptions;
  label: string;
}) {
  return (
    <button
      type="button"
      className="more-btn"
      aria-label={label}
      title={label}
      onClick={(e) => openMenu(e, items(), opts?.())}
      onContextMenu={(e) => openMenu(e, items(), opts?.())}
    >
      <Icon name="more" size={14} />
    </button>
  );
}
