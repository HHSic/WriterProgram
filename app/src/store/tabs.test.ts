// @vitest-environment jsdom
import { beforeEach, expect, it, vi } from 'vitest';
import { makePane, navigate } from '../lib/tabs';
import { useApp } from './state';
import { resizeTab, transferTab } from './tabs';
const mocks = vi.hoisted(() => ({ save: vi.fn() }));
vi.mock('./ui', () => ({ saveEverything: mocks.save, findDoc: () => null, allManuscript: () => [] }));
beforeEach(() => { localStorage.clear(); mocks.save.mockReset().mockResolvedValue(true); useApp.setState({ overview: null, focus: 0, editor: null }); });
it('flushes before moving a tab with its history and width into another pane', async () => {
  const left = makePane([{ kind: 'doc', id: 'a' }, { kind: 'doc', id: 'b' }]);
  left.tabs[0] = { ...navigate(left.tabs[0], { kind: 'doc', id: 'c' }), width: 250 };
  const right = makePane([{ kind: 'doc', id: 'c' }], true);
  useApp.setState({ panes: [left, right] });
  await transferTab(1, left.tabs[0].key, right.active);
  const state = useApp.getState();
  expect(mocks.save).toHaveBeenCalledOnce();
  expect(state.panes[0].tabs.map((t) => t.target.id)).toEqual(['b']);
  expect(state.panes[1].tabs[0]).toEqual(left.tabs[0]);
  expect(state.panes[1].locked).toBe(true);
  expect(state.focus).toBe(1);
  expect(state.panes[1].active).toBe(left.tabs[0].key);
});
it('keeps the last-tab source as an empty drop destination and blocks transfer on save failure', async () => {
  const left = makePane([{ kind: 'doc', id: 'a' }]); const right = makePane();
  useApp.setState({ panes: [left, right] }); mocks.save.mockResolvedValueOnce(false);
  await transferTab(1, left.active!, null);
  expect(useApp.getState().panes[0].tabs).toHaveLength(1);
  await transferTab(1, left.active!, null);
  expect(useApp.getState().panes).toHaveLength(2);
  expect(useApp.getState().panes[0].tabs).toHaveLength(0);
});
it('clamps resized widths and resets them without changing tab order', () => {
  const pane = makePane([{ kind: 'doc', id: 'a' }]); useApp.setState({ panes: [pane] });
  resizeTab(pane.active!, 500); expect(useApp.getState().panes[0].tabs[0].width).toBe(420);
  resizeTab(pane.active!, undefined); expect(useApp.getState().panes[0].tabs[0].width).toBeUndefined();
});
