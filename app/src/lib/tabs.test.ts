import { describe, expect, it } from 'vitest';
import {
  activeTab,
  dropTargets,
  fromSaved,
  makePane,
  moveTab,
  navigate,
  openTab,
  removeTab,
  step,
  toSaved,
  type Target,
} from './tabs';

const doc = (id: string): Target => ({ kind: 'doc', id });
const ids = (targets: Target[]) => targets.map((t) => t.id);

describe('tabs', () => {
  it('goes back and forward, skipping what is gone', () => {
    let tab = makePane([doc('a')]).tabs[0];
    tab = navigate(tab, doc('b'));
    tab = navigate(tab, doc('c'));
    expect(ids(tab.back)).toEqual(['a', 'b']);

    const back = step(tab, -1, () => true)!;
    expect(back.target.id).toBe('b');
    expect(ids(back.forward)).toEqual(['c']);

    const skipping = step(tab, -1, (t) => t.id !== 'b')!;
    expect(skipping.target.id).toBe('a');
    expect(ids(skipping.back)).toEqual([]);

    const forward = step(back, 1, () => true)!;
    expect(forward.target.id).toBe('c');
    expect(ids(forward.back)).toEqual(['a', 'b']);
    expect(step(forward, 1, () => true)).toBeNull();
  });

  it('opens after the open tab and closes to a neighbour', () => {
    let pane = makePane([doc('a'), doc('b')]);
    pane = openTab(pane, doc('c'));
    expect(pane.tabs.map((t) => t.target.id)).toEqual(['a', 'c', 'b']);
    expect(activeTab(pane)?.target.id).toBe('c');

    pane = removeTab(pane, pane.active!);
    expect(activeTab(pane)?.target.id).toBe('b');
    pane = removeTab(pane, pane.active!);
    expect(activeTab(pane)?.target.id).toBe('a');
  });

  it('moves a tab before another', () => {
    const pane = makePane([doc('a'), doc('b'), doc('c')]);
    const [a, , c] = pane.tabs;
    expect(moveTab(pane, c.key, a.key).tabs.map((t) => t.target.id)).toEqual(['c', 'a', 'b']);
    expect(moveTab(pane, a.key, null).tabs.map((t) => t.target.id)).toEqual(['b', 'c', 'a']);
  });

  it('drops tabs of trashed things and fills an empty pane', () => {
    let pane = makePane([doc('a')]);
    pane = { ...pane, tabs: [navigate(pane.tabs[0], doc('x'))] };
    const gone = (t: Target) => t.id === 'x';
    const next = dropTargets(pane, gone, doc('b'));
    expect(next.tabs.map((t) => t.target.id)).toEqual(['b']);
    expect(activeTab(next)?.target.id).toBe('b');

    const kept = dropTargets(makePane([doc('a'), doc('x')]), gone, doc('b'));
    expect(kept.tabs.map((t) => t.target.id)).toEqual(['a']);
  });

  it('saves and restores the layout without missing things', () => {
    const one = makePane([doc('a'), doc('b')]);
    const two = { ...makePane([doc('gone'), { kind: 'card', id: 'k' }]), locked: true };
    const saved = JSON.parse(JSON.stringify(toSaved([one, { ...two, active: two.tabs[1].key }], 1, 'column')));
    const restored = fromSaved(saved, (t) => t.id !== 'gone')!;
    expect(restored.split).toBe('column');
    expect(restored.focus).toBe(1);
    expect(restored.panes.map((p) => p.tabs.map((t) => t.target.id))).toEqual([['a', 'b'], ['k']]);
    expect(restored.panes[1].locked).toBe(true);
    expect(activeTab(restored.panes[1])?.target.id).toBe('k');

    const onlyFirst = fromSaved(saved, (t) => t.kind === 'doc' && t.id !== 'gone')!;
    expect(onlyFirst.panes).toHaveLength(1);
    expect(onlyFirst.focus).toBe(0);
    expect(fromSaved({ panes: 'nope' }, () => true)).toBeNull();
    expect(fromSaved(null, () => true)).toBeNull();
  });
});
