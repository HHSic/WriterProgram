#!/usr/bin/env node
// kfiction MCP 서버 (stdio, JSON-RPC 2.0, 줄 단위 JSON). 외부 의존성 없음.
// 도구: kfiction_check, kfiction_levels
import { readFileSync, existsSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { lint } from '../../../lib/lint.js';
import { renderReport, renderLevels } from '../../../lib/report.js';

const SERVER_INFO = { name: 'kfiction', version: '0.1.0' };
const SUPPORTED = ['2025-06-18', '2025-03-26', '2024-11-05'];
const MAX_BYTES = 2 * 1024 * 1024;
const SOURCE = {
  text: { type: 'string', description: '점검할 원고 본문' },
  path: { type: 'string', description: '점검할 원고 파일 경로 (text가 없을 때)' },
};

const TOOLS = [
  {
    name: 'kfiction_check',
    description: '한국어 소설 원고를 점검한다. 겹친 피동, 의 연쇄, 같은 끝맺음, 연결 어미 뒤 쉼표, 원고에 섞인 설명과 마크다운 등을 짚는다. 막는 기준이 아니라 참고용이다. 파일은 고치지 않는다.',
    inputSchema: { type: 'object', properties: { ...SOURCE, all: { type: 'boolean', description: '참고 항목까지 보이기' } } },
  },
  {
    name: 'kfiction_levels',
    description: '원고의 대사마다 말 단계(하십시오체, 해요체, 해체 등)를 어림해 보여 준다. 화자는 정하지 못한다.',
    inputSchema: { type: 'object', properties: SOURCE },
  },
];

function source(args) {
  if (typeof args.text === 'string') return args.text;
  if (typeof args.path !== 'string') throw new Error('text 또는 path 가 필요하다');
  const abs = path.resolve(args.path);
  if (!existsSync(abs) || !statSync(abs).isFile()) throw new Error(`파일을 찾을 수 없다: ${args.path}`);
  if (statSync(abs).size > MAX_BYTES) throw new Error(`파일이 너무 크다: ${args.path}`);
  return readFileSync(abs, 'utf8');
}

export function callTool(name, args = {}) {
  if (name === 'kfiction_check') {
    const result = lint(source(args));
    return { content: [{ type: 'text', text: renderReport(result, { file: args.path ? path.basename(args.path) : undefined, all: !!args.all }) }, { type: 'text', text: JSON.stringify(result) }] };
  }
  if (name === 'kfiction_levels') return { content: [{ type: 'text', text: renderLevels(source(args)) }] };
  throw Object.assign(new Error(`알 수 없는 도구: ${name}`), { code: -32602 });
}

/** 메시지 하나를 처리한다. 응답 객체 또는 null(알림). */
export function handleMessage(msg) {
  const { id, method, params } = msg;
  const isNotification = id === undefined || id === null;
  const ok = (result) => ({ jsonrpc: '2.0', id, result });
  const fail = (code, message) => ({ jsonrpc: '2.0', id: id ?? null, error: { code, message } });
  switch (method) {
    case 'initialize': {
      const want = params?.protocolVersion;
      return ok({ protocolVersion: SUPPORTED.includes(want) ? want : SUPPORTED[0], capabilities: { tools: {} }, serverInfo: SERVER_INFO });
    }
    case 'ping':
      return isNotification ? null : ok({});
    case 'tools/list':
      return ok({ tools: TOOLS });
    case 'tools/call':
      try {
        return ok(callTool(params?.name, params?.arguments ?? {}));
      } catch (e) {
        if (e.code === -32602) return fail(-32602, e.message);
        return ok({ isError: true, content: [{ type: 'text', text: e.message }] });
      }
    default:
      return isNotification ? null : fail(-32601, `Method not found: ${method}`);
  }
}

function main() {
  let buf = '';
  process.stdin.setEncoding('utf8');
  process.stdin.on('data', (chunk) => {
    buf += chunk;
    let i;
    while ((i = buf.indexOf('\n')) >= 0) {
      const line = buf.slice(0, i).trim();
      buf = buf.slice(i + 1);
      if (!line) continue;
      let res;
      try {
        res = handleMessage(JSON.parse(line));
      } catch {
        res = { jsonrpc: '2.0', id: null, error: { code: -32700, message: 'Parse error' } };
      }
      if (res) process.stdout.write(JSON.stringify(res) + '\n');
    }
  });
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
