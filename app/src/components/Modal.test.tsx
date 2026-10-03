// @vitest-environment jsdom
// Dialogs and Korean input (docs/safety-design.md K2): Esc while composing,
// clicking outside, asking before dropping edits, and where focus goes.

import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Modal } from './Modal';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement;
let root: Root;

beforeEach(() => {
  host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
});

afterEach(() => {
  act(() => root.unmount());
  document.body.replaceChildren();
});

function show(props: Partial<Parameters<typeof Modal>[0]> = {}, onClose = vi.fn()) {
  act(() =>
    root.render(
      <Modal title="시험 창" onClose={onClose} footer={<button type="button">확인</button>} {...props}>
        <input aria-label="이름" />
        <button type="button">가운데</button>
      </Modal>,
    ),
  );
  return onClose;
}

function press(key: string, init: KeyboardEventInit & { keyCode?: number } = {}, target: EventTarget = document.activeElement ?? document.body) {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init });
  if (init.keyCode !== undefined) Object.defineProperty(event, 'keyCode', { value: init.keyCode });
  act(() => {
    target.dispatchEvent(event);
  });
  return event;
}

function button(label: string): HTMLButtonElement {
  const found = [...document.querySelectorAll('button')].find((b) => b.textContent === label || b.getAttribute('aria-label') === label);
  if (!found) throw new Error(`no button ${label}`);
  return found;
}

describe('Esc', () => {
  it('closes the dialog', () => {
    const onClose = show();
    press('Escape');
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('is ignored while a Korean syllable is being composed', () => {
    const onClose = show();
    press('Escape', { isComposing: true });
    press('Escape', { keyCode: 229 });
    expect(onClose).not.toHaveBeenCalled();
  });
});

describe('clicking outside', () => {
  function clickBackdrop() {
    const overlay = document.querySelector('.overlay')!;
    act(() => {
      overlay.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    });
  }

  it('does nothing by default', () => {
    const onClose = show();
    clickBackdrop();
    expect(onClose).not.toHaveBeenCalled();
  });

  it('closes small dialogs that allow it', () => {
    const onClose = show({ dismissOnBackdrop: true });
    clickBackdrop();
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('does not close when pressing inside the dialog', () => {
    const onClose = show({ dismissOnBackdrop: true });
    act(() => {
      document.querySelector('.modal-body')!.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    });
    expect(onClose).not.toHaveBeenCalled();
  });
});

describe('edits not applied yet', () => {
  it('asks before closing with Esc and keeps the dialog when the writer goes on', () => {
    const onClose = show({ dirty: true });
    press('Escape');
    expect(onClose).not.toHaveBeenCalled();
    expect(document.body.textContent).toContain('고친 내용을 버릴까요?');
    act(() => button('계속 고치기').click());
    expect(document.body.textContent).not.toContain('고친 내용을 버릴까요?');
    expect(onClose).not.toHaveBeenCalled();
  });

  it('closes when the writer drops the edits from the close button', () => {
    const onClose = show({ dirty: true });
    act(() => button('닫기').click());
    expect(onClose).not.toHaveBeenCalled();
    act(() => button('버리고 닫기').click());
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('takes a second Esc as "go on editing"', () => {
    const onClose = show({ dirty: true });
    press('Escape');
    press('Escape');
    expect(document.body.textContent).not.toContain('고친 내용을 버릴까요?');
    expect(onClose).not.toHaveBeenCalled();
  });
});

describe('focus', () => {
  it('starts in the dialog and goes back where it was when the dialog closes', () => {
    const opener = document.createElement('button');
    opener.textContent = '열기';
    document.body.append(opener);
    opener.focus();
    show();
    expect(document.activeElement?.getAttribute('aria-label')).toBe('이름');
    act(() => root.render(<></>));
    expect(document.activeElement).toBe(opener);
  });

  it('goes round inside the dialog with Tab and Shift+Tab', () => {
    show();
    const first = button('닫기');
    const last = button('확인');
    last.focus();
    const forward = press('Tab');
    expect(forward.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(first);
    const back = press('Tab', { shiftKey: true });
    expect(back.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(last);
  });

  it('leaves Tab alone between the first and the last', () => {
    show();
    button('가운데').focus();
    expect(press('Tab').defaultPrevented).toBe(false);
  });
});
