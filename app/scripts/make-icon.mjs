// Draws the app icon (app-icon.png, 1024×1024): a sheet of 원고지 on the
// accent red. Run `node scripts/make-icon.mjs && npx tauri icon` after editing.

import { writeFileSync } from 'node:fs';
import { deflateSync } from 'node:zlib';

const SIZE = 1024;
const RED = [0xb8, 0x43, 0x2f];
const PAPER = [0xff, 0xfd, 0xf7];
const LINE = [0xd9, 0x8a, 0x7b];

/** Coverage of a rounded rectangle at (x, y), with 4×4 supersampling. */
function roundedRect(x, y, left, top, right, bottom, radius) {
  let hits = 0;
  for (let sy = 0; sy < 4; sy++) {
    for (let sx = 0; sx < 4; sx++) {
      const px = x + (sx + 0.5) / 4;
      const py = y + (sy + 0.5) / 4;
      if (px < left || px > right || py < top || py > bottom) continue;
      const cx = Math.min(Math.max(px, left + radius), right - radius);
      const cy = Math.min(Math.max(py, top + radius), bottom - radius);
      if ((px - cx) ** 2 + (py - cy) ** 2 <= radius ** 2) hits++;
    }
  }
  return hits / 16;
}

const mix = (a, b, t) => a.map((v, i) => Math.round(v + (b[i] - v) * t));

// The sheet: 4 columns × 5 rows of cells, like a corner of 원고지.
const sheet = { left: 232, top: 212, right: 792, bottom: 812 };
const cols = 4;
const rows = 5;
const gap = 26;
const cellW = (sheet.right - sheet.left - gap * (cols + 1)) / cols;
const cellH = (sheet.bottom - sheet.top - gap * (rows + 1)) / rows;

function cellLine(x, y) {
  const lx = x - sheet.left - gap;
  const ly = y - sheet.top - gap;
  if (lx < 0 || ly < 0) return 0;
  const inX = lx % (cellW + gap);
  const inY = ly % (cellH + gap);
  const col = Math.floor(lx / (cellW + gap));
  const row = Math.floor(ly / (cellH + gap));
  if (col >= cols || row >= rows || inX > cellW || inY > cellH) return 0;
  const edge = Math.min(inX, inY, cellW - inX, cellH - inY);
  return edge < 7 ? 1 : 0;
}

const raw = Buffer.alloc((SIZE * 4 + 1) * SIZE);
for (let y = 0; y < SIZE; y++) {
  const row = y * (SIZE * 4 + 1);
  raw[row] = 0; // no filter
  for (let x = 0; x < SIZE; x++) {
    const bg = roundedRect(x, y, 32, 32, SIZE - 32, SIZE - 32, 220);
    const paper = roundedRect(x, y, sheet.left, sheet.top, sheet.right, sheet.bottom, 36);
    let color = mix(RED, PAPER, paper);
    if (paper > 0.99 && cellLine(x, y)) color = LINE;
    const i = row + 1 + x * 4;
    raw[i] = color[0];
    raw[i + 1] = color[1];
    raw[i + 2] = color[2];
    raw[i + 3] = Math.round(bg * 255);
  }
}

function crc32(buf) {
  let c;
  let crc = 0xffffffff;
  for (const byte of buf) {
    c = (crc ^ byte) & 0xff;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    crc = (crc >>> 8) ^ c;
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

const header = Buffer.alloc(13);
header.writeUInt32BE(SIZE, 0);
header.writeUInt32BE(SIZE, 4);
header[8] = 8; // bit depth
header[9] = 6; // RGBA
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk('IHDR', header),
  chunk('IDAT', deflateSync(raw, { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
]);
writeFileSync(new URL('../app-icon.png', import.meta.url), png);
console.log('app-icon.png written');
