// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { AiHelperField, mcpCommand } from './AiHelperField';

const mock = vi.hoisted(() => ({
  api: { mcpServer: vi.fn() },
  toast: vi.fn(),
  error: vi.fn(),
  copy: vi.fn(),
}));
vi.mock('../../api', () => ({ api: mock.api }));
vi.mock('../../store', () => ({ showToast: mock.toast, toastError: mock.error }));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root;
beforeEach(() => {
  vi.clearAllMocks();
  mock.copy.mockResolvedValue(undefined);
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: mock.copy } });
  const host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
});
afterEach(() => {
  act(() => root.unmount());
  document.body.replaceChildren();
});

async function show() {
  await act(async () => {
    root.render(<AiHelperField root="G:\내 드라이브\소설\사흘째 맑음" />);
  });
}

it('builds the claude mcp add command for the project', () => {
  expect(mcpCommand('C:\\Apps\\writer-mcp.exe', 'D:\\소설')).toBe(
    'claude mcp add --scope user writerprogram -- "C:\\Apps\\writer-mcp.exe" serve --project "D:\\소설"',
  );
});

it('shows and copies the command when writer-mcp is installed', async () => {
  mock.api.mcpServer.mockResolvedValue('C:\\Users\\작가\\AppData\\Local\\WriterProgram\\writer-mcp.exe');
  await show();
  const input = document.querySelector('input')!;
  expect(input.value).toContain('writer-mcp.exe" serve --project "G:\\내 드라이브\\소설\\사흘째 맑음"');
  const button = [...document.querySelectorAll('button')].find((b) => b.textContent === '명령 복사')!;
  await act(async () => button.click());
  expect(mock.copy).toHaveBeenCalledWith(input.value);
  expect(mock.toast).toHaveBeenCalled();
});

it('stays hidden without writer-mcp', async () => {
  mock.api.mcpServer.mockResolvedValue(null);
  await show();
  expect(document.body.textContent).toBe('');
});
