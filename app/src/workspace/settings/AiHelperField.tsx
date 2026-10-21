// AI 도우미 연결 in 작품 설정: the command that connects Claude Code to this
// project through writer-mcp, which the installer puts next to the app
// (docs/mcp.md). Hidden when writer-mcp is not there, as in the browser preview.

import { useEffect, useState } from 'react';
import { api } from '../../api';
import { showToast, toastError } from '../../store';

/** The Claude Code command that adds writer-mcp for the project at `root`. */
export function mcpCommand(server: string, root: string): string {
  return `claude mcp add --scope user writerprogram -- "${server}" serve --project "${root}"`;
}

export function AiHelperField({ root }: { root: string }) {
  const [server, setServer] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    api.mcpServer().then(
      (s) => live && setServer(s),
      () => {},
    );
    return () => {
      live = false;
    };
  }, []);

  if (!server) return null;
  const command = mcpCommand(server, root);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(command);
      showToast({ text: '연결 명령을 복사했습니다. Claude Code를 쓰는 터미널에 붙여 넣으세요.' });
    } catch (e) {
      toastError('명령을 복사하지 못함', e);
    }
  };

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
