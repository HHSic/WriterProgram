// 단축키: every keyboard shortcut, grouped, read from the one table the key
// handlers use (lib/shortcuts.ts).

import { Fragment } from 'react';
import { Modal } from '../components/Modal';
import { comboText, shortcutsByGroup } from '../lib/shortcuts';
import { closeDialog } from '../store';

export function ShortcutsDialog() {
  const mac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);
  return (
    <Modal title="단축키" onClose={closeDialog} width={560} dismissOnBackdrop>
      <div className="shortcut-list">
        {shortcutsByGroup().map(({ group, items }) => (
          <section key={group} aria-label={group}>
            <h3>{group}</h3>
            <dl>
              {items.map((s) => (
                <Fragment key={s.id}>
                  <dt>
                    {s.label}
                    {s.note && <small>{s.note}</small>}
                  </dt>
                  <dd>
                    {s.keys.map((k, i) => (
                      <Fragment key={k}>
                        {i > 0 && <span className="shortcut-or">또는</span>}
                        <kbd>{comboText(k, mac)}</kbd>
                      </Fragment>
                    ))}
                  </dd>
                </Fragment>
              ))}
            </dl>
          </section>
        ))}
      </div>
    </Modal>
  );
}
