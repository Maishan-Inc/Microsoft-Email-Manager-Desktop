// 生成一个纯色 512x512 PNG 作为应用图标源，供 `tauri icon` 使用。
// 无第三方依赖，使用 Node 内置 zlib 手写最小 PNG。
import { writeFileSync } from "node:fs";
import { deflateSync } from "node:zlib";

const W = 512;
const H = 512;
// 主题蓝 #3b82f6
const [R, G, B] = [0x3b, 0x82, 0xf6];

// 构造 RGBA 原始数据，每行前置 filter 字节 0
const stride = W * 4;
const raw = Buffer.alloc((stride + 1) * H);
for (let y = 0; y < H; y++) {
  const rowStart = y * (stride + 1);
  raw[rowStart] = 0; // filter: none
  for (let x = 0; x < W; x++) {
    const p = rowStart + 1 + x * 4;
    // 画一个简单的圆角观感：边缘留白透明
    const dx = x - W / 2;
    const dy = y - H / 2;
    const inside = Math.abs(dx) < 220 && Math.abs(dy) < 220;
    raw[p] = R;
    raw[p + 1] = G;
    raw[p + 2] = B;
    raw[p + 3] = inside ? 255 : 0;
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
console.log("已生成 appicon.png");
