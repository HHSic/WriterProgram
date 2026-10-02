// Export to files, the file and folder pickers, and importing chapters from files.

import { blocksFromJSON } from '../../editor/counts';
import type { Backend, ImportChapter, ImportPreview, PageSetup } from '../types';
import { body, doc, newDoc, project } from './state';

type SkipTotalsLike = { tables: number; images: number; footnotes: number };

/** What the sample 한글 file says about its pages. */
const MOCK_PAGE: PageSetup = {
  paper: { kind: 'a4', widthMm: 210, heightMm: 297 },
  margins: { top: 20, bottom: 15, inside: 30, outside: 30, header: 15, footer: 15 },
  header: { content: 'custom', text: '달빛 서점', align: 'center', skipChapterFirst: false },
  pageNumbers: 'center',
  footer: { text: '투고용 · 무단 배포 금지', align: 'right' },
  summary: '용지 A4, 여백 위 20 · 아래 15 · 양옆 30mm. 머리말은 “달빛 서점”(가운데), 쪽 번호는 아래 가운데, 꼬리말은 “투고용 · 무단 배포 금지”(오른쪽).',
};

const exportText: Backend['exportText'] = async (root, items, opts) => {
  const p = project(root);
  const sep = opts.blankLineBetween ? '\n\n' : '\n';
  return items
    .map((item) => {
      const blocks = blocksFromJSON(doc(p, item.docId).body);
      const text = blocks
        .map((b) => (b.scene ? (opts.blankLineBetween ? opts.sceneBreak : `\n${opts.sceneBreak}\n`) : b.lines.join('\n')))
        .join(sep);
      return opts.includeTitles && item.heading.trim() ? `${item.heading.trim()}\n\n${text}` : text;
    })
    .join('\n\n\n');
};

const importPreview: Backend['importPreview'] = async (paths, opts) => {
  const none = { tables: 0, images: 0, footnotes: 0 };
  const sample: [string, number, SkipTotalsLike][] = [
    ['문 닫는 시간', 4210, { tables: 0, images: 0, footnotes: 0 }],
    ['빗소리가 들리는 밤', 5032, { tables: 1, images: 2, footnotes: 0 }],
    ['비에 젖은 손님', 4876, { tables: 0, images: 0, footnotes: 1 }],
  ];
  const files: ImportPreview['files'] = paths.map((path) => {
    const name = path.split('\\').pop() ?? path;
    const kind = name.split('.').pop() ?? '';
    const bad = kind === 'hwp';
    return {
      name,
      kind,
      error: bad ? '옛 한글 형식(.hwp)은 읽을 수 없음 · 한글에서 hwpx로 저장해 주세요' : null,
      encoding: kind === 'txt' ? 'euc-kr' : null,
      rule: bad ? '' : opts.rule === 'file' ? '파일 하나가 회차 하나' : kind === 'docx' || kind === 'hwpx' ? '제목 서식' : '제N화',
      chapters: 0,
      page: kind === 'hwpx' ? MOCK_PAGE : null,
    };
  });
  const chapters: ImportChapter[] = [];
  files.forEach((f, file) => {
    if (f.error) return;
    const list: [string, number, SkipTotalsLike][] =
      opts.rule === 'file' ? [[f.name.replace(/\.[^.]+$/, ''), 9800, none]] : f.kind === 'docx' || f.kind === 'hwpx' ? sample : [['외전 · 그날의 서하', 3120, none]];
    for (const [title, chars, skipped] of list) {
      chapters.push({ index: chapters.length, file, title, chars, paragraphs: Math.round(chars / 60), snippet: '서하는 매일 밤 열한 시에 서점 문을 닫았다. 할머니가 그랬고, 할머니의 어머니도 그랬다고 했다.', skipped });
    }
    f.chapters = list.length;
  });
  const skipped = { ...none };
  for (const c of chapters) {
    skipped.tables += c.skipped.tables;
    skipped.images += c.skipped.images;
    skipped.footnotes += c.skipped.footnotes;
  }
  return { files, chapters, skipped };
};

export const ioMethods = {
  exportText,
  async exportTxt(root, items, opts, dest, perDoc) {
    const text = await exportText(root, items, opts);
    console.info('[mock] export', { dest, perDoc, text });
    return perDoc ? items.map((i) => `${dest}\\${i.fileName}.txt`) : [dest];
  },
  async reveal(path) {
    console.info('[mock] reveal', path);
  },
  async pickFolder() {
    return 'C:\\Users\\작가\\Documents\\WriterProgram';
  },
  async exportFile(_root, items, opts, format, kind, dest, perDoc) {
    console.info('[mock] export file', { kind, dest, perDoc, format, opts, items });
    return perDoc ? items.map((i) => `${dest}\\${i.fileName}.${kind}`) : [dest];
  },
  async pickFiles(_title, only) {
    if (only) return [`C:\\Users\\작가\\Downloads\\교정본_김편집.${only.extensions[0]}`];
    return ['C:\\원고\\연재본.hwpx', 'C:\\원고\\외전.txt', 'C:\\원고\\옛 원고.hwp'];
  },
  importPreview,
  async importCommit(root, paths, opts, spec) {
    const p = project(root);
    const preview = await importPreview(paths, opts);
    const ids: string[] = [];
    for (const pick of [...spec.picks].sort((a, b) => a.index - b.index)) {
      const chapter = preview.chapters[pick.index];
      const d = newDoc(p, 'manuscript', pick.title, body(chapter?.snippet ?? ''));
      ids.push(d.meta.id);
    }
    const withAfter = spec.after ? p.parts.find((x) => x.docs.includes(spec.after!)) : undefined;
    if (withAfter) withAfter.docs.splice(withAfter.docs.indexOf(spec.after!) + 1, 0, ...ids);
    else (p.parts.find((x) => x.id === spec.partId) ?? p.parts[p.parts.length - 1]).docs.push(...ids);
    const page = spec.pageSetup ? preview.files.find((f) => f.page)?.page : undefined;
    if (page) {
      const f = p.info.manuscriptFormat;
      p.info.manuscriptFormat = {
        ...f,
        paper: page.paper,
        margins: page.margins,
        header: page.header,
        pageNumbers: page.pageNumbers !== null,
        pageNumberAlign: page.pageNumbers ?? f.pageNumberAlign,
        footer: page.footer,
      };
    }
    return { docs: ids, notes: 0, format: !!page };
  },
  async pickSaveFile(_title, defaultName) {
    return `C:\\Users\\작가\\Documents\\${defaultName}`;
  },
} satisfies Partial<Backend>;
