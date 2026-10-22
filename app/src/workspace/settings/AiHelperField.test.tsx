// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { AiHelperField, cliExample, mcpCommand } from './AiHelperField';

const mock = vi.hoisted(() => ({
  api: { mcpServer: vi.fn(), cliCommand: vi.fn(), cliPathSet: vi.fn() },
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
  mock.api.cliCommand.mockResolvedValue(null);
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

it('stays hidden without writer-mcp and writer', async () => {
  mock.api.mcpServer.mockResolvedValue(null);
  await show();
  expect(document.body.textContent).toBe('');
});

const button = (text: string) => [...document.querySelectorAll('button')].find((b) => b.textContent === text);

it('offers the writer command and puts it on PATH', async () => {
  mock.api.mcpServer.mockResolvedValue(null);
  const cli = { path: 'C:\\Users\\작가\\AppData\\Local\\WriterProgram\\writer.exe', on: false, canChange: true };
  mock.api.cliCommand.mockResolvedValue(cli);
  mock.api.cliPathSet.mockResolvedValue({ ...cli, on: true });
  await show();
  expect(document.querySelector<HTMLInputElement>('[aria-label="터미널 명령 예"]')!.value).toBe(cliExample('G:\\내 드라이브\\소설\\사흘째 맑음'));
  await act(async () => button('PATH에 등록')!.click());
  expect(mock.api.cliPathSet).toHaveBeenCalledWith(true);
  expect(button('PATH에서 빼기')).toBeTruthy();
  expect(mock.toast.mock.calls.at(-1)![0].text).toContain('새로 연 터미널부터');
});

it('tells how to add it to PATH where the app cannot', async () => {
  mock.api.mcpServer.mockResolvedValue(null);
  mock.api.cliCommand.mockResolvedValue({ path: '/Applications/WriterProgram.app/Contents/MacOS/writer', on: false, canChange: false });
  await show();
  expect(document.body.textContent).toContain('export PATH="/Applications/WriterProgram.app/Contents/MacOS:$PATH"');
  expect(button('PATH에 등록')).toBeUndefined();
});
