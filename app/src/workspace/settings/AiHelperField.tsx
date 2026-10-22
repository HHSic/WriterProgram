// AI 도우미 연결 in 작품 설정: the command that connects Claude Code to this
// project through writer-mcp, and the `writer` command for terminals, both of
// which the installer puts next to the app (docs/mcp.md, docs/cli.md). Each
// is hidden when it is not there, as in the browser preview.

import { useEffect, useState } from 'react';
import { api } from '../../api';
import type { CliCommand } from '../../api/types';
import { showToast, toastError } from '../../store';

/** The Claude Code command that adds writer-mcp for the project at `root`. */
export function mcpCommand(server: string, root: string): string {
  return `claude mcp add --scope user writerprogram -- "${server}" serve --project "${root}"`;
}

/** A `writer` command for the project at `root`, to try in a terminal. */
export function cliExample(root: string): string {
  return `writer --project "${root}" outline`;
}

export function AiHelperField({ root }: { root: string }) {
  const [server, setServer] = useState<string | null>(null);
  const [cli, setCli] = useState<CliCommand | null>(null);

  useEffect(() => {
    let live = true;
    api.mcpServer().then(
      (s) => live && setServer(s),
      () => {},
    );
    api.cliCommand().then(
      (c) => live && setCli(c),
      () => {},
    );
    return () => {
      live = false;
    };
  }, []);

  return (
    <>
      {server && <McpCommand server={server} root={root} />}
      {cli && <TerminalCommand cli={cli} root={root} onChange={setCli} />}
    </>
  );
}

async function copyText(text: string, done: string) {
  try {
    await navigator.clipboard.writeText(text);
    showToast({ text: done });
  } catch (e) {
    toastError('명령을 복사하지 못함', e);
  }
}

function McpCommand({ server, root }: { server: string; root: string }) {
  const command = mcpCommand(server, root);

  const copy = () => copyText(command, '연결 명령을 복사했습니다. Claude Code를 쓰는 터미널에 붙여 넣으세요.');

  return (
    <div className="field">
      <span>AI 도우미 연결 (Claude Code)</span>
      <div className="row">
        <input readOnly value={command} className="grow" aria-label="연결 명령" onFocus={(e) => e.currentTarget.select()} />
        <button type="button" className="btn" onClick={() => void copy()}>
          명령 복사
        </button>
      </div>
      <small className="hint">
        터미널에 붙여 넣으면 Claude가 이 작품의 목차·회차·설정집·메모를 읽고, 시킬 때만 씁니다. 본문을 고치기 전에는 기록을 남겨 언제든 되돌릴 수 있습니다. 같은
        이름으로 이미 연결했다면 먼저 claude mcp remove writerprogram 을 실행합니다.
      </small>
    </div>
  );
}

/** The `writer` command: whether a terminal finds it, and turning that on or off. */
function TerminalCommand({ cli, root, onChange }: { cli: CliCommand; root: string; onChange: (c: CliCommand | null) => void }) {
  const [busy, setBusy] = useState(false);
  const example = cliExample(root);
  const folder = cli.path.replace(/[\\/][^\\/]+$/, '');

  const toggle = async () => {
    setBusy(true);
    try {
      const next = await api.cliPathSet(!cli.on);
      onChange(next);
      showToast({
        text: next?.on
          ? 'PATH에 등록했습니다. 새로 연 터미널부터 writer로 부를 수 있습니다.'
          : 'PATH에서 뺐습니다. 터미널에서는 전체 경로로 부릅니다.',
      });
    } catch (e) {
      toastError('PATH를 바꾸지 못함', e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="field">
      <span>터미널 명령 (writer)</span>
      <div className="row">
        <input readOnly value={example} className="grow" aria-label="터미널 명령 예" onFocus={(e) => e.currentTarget.select()} />
        <button type="button" className="btn" onClick={() => void copyText(example, '명령을 복사했습니다. 터미널에 붙여 넣으세요.')}>
          명령 복사
        </button>
      </div>
      <div className="row">
        <small className="hint grow">
          {cli.on
            ? '터미널에서 writer로 부를 수 있습니다(새로 연 터미널부터).'
            : cli.canChange
              ? 'PATH에 등록하면 터미널에서 writer로 부를 수 있습니다.'
              : `셸 설정 파일에 export PATH="${folder}:$PATH" 를 넣으면 writer로 부를 수 있습니다.`}
        </small>
        {cli.canChange && (
          <button type="button" className="btn small" disabled={busy} onClick={() => void toggle()}>
            {cli.on ? 'PATH에서 빼기' : 'PATH에 등록'}
          </button>
        )}
      </div>
      <small className="hint">
        MCP 연결과 같은 도구와 점검을 셸 명령으로 씁니다. writer help로 명령 목록을 봅니다. Claude Code 같은 도우미에게는 “writer help를 보고 이 작품을 다뤄 줘”라고
        하면 됩니다.
      </small>
    </div>
  );
}
