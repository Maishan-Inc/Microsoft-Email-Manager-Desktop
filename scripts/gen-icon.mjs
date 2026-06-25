// 生成 512x512 PNG 作为应用图标源，供 `pnpm tauri icon` 使用。
// 图形：墨黑圆角方底 + 白色「层叠文档」字形（与 src/assets/logo-*.svg 一致的观感）。
// 无第三方依赖，使用 Node 内置 zlib 手写最小 PNG。
import { writeFileSync } from "node:fs";
import { deflateSync } from "node:zlib";

const W = 512;
const H = 512;

// 颜色（DESIGN.md ink / white）
const INK = [0x17, 0x17, 0x17];
const WHITE = [0xff, 0xff, 0xff];

// 圆角矩形命中测试
function inRR(px, py, x0, y0, x1, y1, r) {
  if (px < x0 || px >= x1 || py < y0 || py >= y1) return false;
  const cx = px < x0 + r ? x0 + r : px > x1 - r ? x1 - r : px;
  const cy = py < y0 + r ? y0 + r : py > y1 - r ? y1 - r : py;
  const dx = px - cx;
  const dy = py - cy;
  return dx * dx + dy * dy <= r * r;
}

// 三张层叠的「纸」：从右上（后）到左下（前），各带墨色描边形成分隔。
const SR = 30; // 纸圆角
const HALO = 9; // 纸之间的墨色间隔
// 中间纸居中
const B = [151, 151, 361, 361];
const A = [B[0] + 22, B[1] - 22, B[2] + 22, B[3] - 22]; // 后
const C = [B[0] - 22, B[1] + 22, B[2] - 22, B[3] + 22]; // 前

function colorAt(x, y) {
  let c = null; // null = 透明
  // 圆角方底（墨黑）
  if (inRR(x, y, 40, 40, 472, 472, 96)) c = INK;
  if (c === null) return null;
  // 后纸
  if (inRR(x, y, A[0], A[1], A[2], A[3], SR)) c = WHITE;
  // 中纸（先描边后填白）
  if (inRR(x, y, B[0] - HALO, B[1] - HALO, B[2] + HALO, B[3] + HALO, SR + HALO)) c = INK;
  if (inRR(x, y, B[0], B[1], B[2], B[3], SR)) c = WHITE;
  // 前纸
  if (inRR(x, y, C[0] - HALO, C[1] - HALO, C[2] + HALO, C[3] + HALO, SR + HALO)) c = INK;
  if (inRR(x, y, C[0], C[1], C[2], C[3], SR)) c = WHITE;
  return c;
}

const stride = W * 4;
const raw = Buffer.alloc((stride + 1) * H);
for (let y = 0; y < H; y++) {
  const rowStart = y * (stride + 1);
  raw[rowStart] = 0; // filter: none
  for (let x = 0; x < W; x++) {
    const p = rowStart + 1 + x * 4;
    const c = colorAt(x, y);
    if (c === null) {
      raw[p] = raw[p + 1] = raw[p + 2] = 0;
      raw[p + 3] = 0; // 透明
    } else {
      raw[p] = c[0];
      raw[p + 1] = c[1];
      raw[p + 2] = c[2];
      raw[p + 3] = 255;
    }
  }
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])) >>> 0, 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

function crc32(buf) {
  let c = ~0;
  for (let i = 0; i < buf.length; i++) {
    c ^= buf[i];
    for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1));
  }
  return ~c;
}

const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(W, 0);
ihdr.writeUInt32BE(H, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type RGBA
const idat = deflateSync(raw);
const png = Buffer.concat([
  sig,
  chunk("IHDR", ihdr),
  chunk("IDAT", idat),
  chunk("IEND", Buffer.alloc(0)),
]);

writeFileSync(new URL("./appicon.png", import.meta.url), png);
console.log("已生成 appicon.png（层叠文档图标）");
