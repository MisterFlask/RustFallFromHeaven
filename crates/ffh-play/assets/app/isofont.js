// Herald, the isometric interface's lettering (docs/ISO.md, "The interface"): a pixel serif drawn
// for this game after the reference screenshot's panels (tools/m7/ref/reference_full.png), as glyph
// grids below, and made into a web font when the page loads (`installIsoFont`), so every label,
// button and paragraph of the iso skin takes it through CSS (`font-family: Herald`), wraps and
// selects like any text, and stays on the pixel grid.
//
// Grid: capitals and ascenders 7 px tall (rows 0-6, the baseline under row 6), x-height 5 (rows 2-6),
// descenders 2 (rows 7-8); 1 px strokes, 1 px between letters, a 3 px word space. Bold (weight 700)
// is the classic pixel bold: every glyph smeared 1 px to the right. Accented letters are composed
// from their base and a mark (rows 0-1 over a lower-case letter, one row above a capital).
//
// The font's em is 10 px of the grid (ascent 8, descent 3), so `font-size: 20px` draws 2 CSS px a
// grid pixel (the reference's scale, `--hu` in style.css) and `10px` 1. The outlines are the grid's
// squares, so at whole multiples the glyph edges fall on device pixels.

const UPM = 1000;
const PX = 100; // font units per grid pixel
const BASE_ROW = 6; // the last row above the baseline
const SPACE = 3;

// Each glyph: `= <char>` then its rows from row 0 (the cap line), 7 rows, or 9 with descenders;
// `#` is ink. The width is the rows' length. `= <char>!` is the bold drawn by hand, for the letters
// whose 1 px counters the smear would close (m n w v k y M N W).
const SOURCE = `
= A
...#...
..#.#..
..#.#..
.#...#.
.#####.
.#...#.
###.###
= B
######.
.#....#
.#....#
.#####.
.#....#
.#....#
######.
= C
..####.
.#....#
#......
#......
#......
.#....#
..####.
= D
#####..
.#...#.
.#....#
.#....#
.#....#
.#...#.
#####..
= E
######
.#...#
.#.#..
.###..
.#.#..
.#...#
######
= F
######
.#...#
.#.#..
.###..
.#.#..
.#....
###...
= G
..####.
.#....#
#......
#..####
#.....#
.#....#
..####.
= H
###.###
.#...#.
.#...#.
.#####.
.#...#.
.#...#.
###.###
= I
###
.#.
.#.
.#.
.#.
.#.
###
= J
..###
...#.
...#.
...#.
...#.
#..#.
.##..
= K
###.##
.#..#.
.#.#..
.##...
.#.#..
.#..#.
###.##
= L
###...
.#....
.#....
.#....
.#....
.#...#
######
= M
##...##
.##.##.
.#.#.#.
.#.#.#.
.#...#.
.#...#.
###.###
= N
##..###
.##..#.
.#.#.#.
.#.#.#.
.#..##.
.#...#.
###..#.
= O
..###..
.#...#.
#.....#
#.....#
#.....#
.#...#.
..###..
= P
######.
.#....#
.#....#
.#####.
.#.....
.#.....
###....
= Q
..###..
.#...#.
#.....#
#.....#
#.....#
.#...#.
..###..
....##.
.....##
= R
######.
.#....#
.#....#
.#####.
.#..#..
.#...#.
###..##
= S
.####.
#....#
#.....
.####.
.....#
#....#
.####.
= T
#######
#..#..#
...#...
...#...
...#...
...#...
..###..
= U
###.###
.#...#.
.#...#.
.#...#.
.#...#.
.#...#.
..###..
= V
###.###
.#...#.
.#...#.
..#.#..
..#.#..
...#...
...#...
= W
###...###
.#.....#.
.#..#..#.
.#..#..#.
.#.#.#.#.
..#...#..
..#...#..
= X
###.###
.#...#.
..#.#..
...#...
..#.#..
.#...#.
###.###
= Y
###.###
.#...#.
..#.#..
...#...
...#...
...#...
..###..
= Z
######
#...#.
...#..
..#...
.#....
#....#
######
= a
.....
.....
.###.
....#
.####
#...#
.####
= b
##...
.#...
.###.
.#..#
.#..#
.#..#
#.##.
= c
....
....
.###
#...
#...
#...
.###
= d
...##
....#
.####
#...#
#...#
#...#
.##.#
= e
.....
.....
.###.
#...#
#####
#....
.####
= f
..##
.#..
###.
.#..
.#..
.#..
###.
= g
.....
.....
.####
#...#
#...#
#...#
.####
....#
.###.
= h
##....
.#....
.###..
.#..#.
.#..#.
.#..#.
##..##
= i
.#.
...
##.
.#.
.#.
.#.
###
= j
..#
...
.##
..#
..#
..#
..#
#.#
.#.
= k
##...
.#...
.#.##
.#.#.
.##..
.#.#.
##.##
= l
##.
.#.
.#.
.#.
.#.
.#.
###
= m
.......
.......
##.#.#.
.##.#.#
.#..#.#
.#..#.#
##.##.#
= n
......
......
##.##.
.##..#
.#...#
.#...#
###.##
= o
.....
.....
.###.
#...#
#...#
#...#
.###.
= p
.....
.....
####.
.#..#
.#..#
.#..#
.###.
.#...
###..
= q
.....
.....
.####
#..#.
#..#.
#..#.
.###.
...#.
..###
= r
.....
.....
##.##
.##..
.#...
.#...
###..
= s
....
....
.###
#...
.##.
...#
###.
= t
....
.#..
###.
.#..
.#..
.#..
..##
= u
......
......
##..#.
.#..#.
.#..#.
.#..#.
..##.#
= v
.....
.....
##.##
.#.#.
.#.#.
..#..
..#..
= w
.......
.......
##...##
.#.#.#.
.#.#.#.
.#.#.#.
..#.#..
= x
.....
.....
##.##
.#.#.
..#..
.#.#.
##.##
= y
.....
.....
##.##
.#.#.
.#.#.
..#..
..#..
.#...
##...
= z
....
....
####
...#
..#.
.#..
####
= 0
.###.
#...#
#..##
#.#.#
##..#
#...#
.###.
= 1
..#..
.##..
..#..
..#..
..#..
..#..
.###.
= 2
.###.
#...#
....#
...#.
..#..
.#...
#####
= 3
.###.
#...#
....#
..##.
....#
#...#
.###.
= 4
...#.
..##.
.#.#.
#..#.
#####
...#.
..###
= 5
#####
#....
####.
....#
....#
#...#
.###.
= 6
..##.
.#...
#....
####.
#...#
#...#
.###.
= 7
#####
#...#
...#.
..#..
..#..
..#..
..#..
= 8
.###.
#...#
#...#
.###.
#...#
#...#
.###.
= 9
.###.
#...#
#...#
.####
....#
...#.
.##..
= .
.
.
.
.
.
.
#
= ,
..
..
..
..
..
.#
.#
#.
..
= :
.
.
.
#
.
.
#
= ;
..
..
..
.#
..
.#
.#
#.
..
= !
#
#
#
#
#
.
#
= ?
.###.
#...#
....#
...#.
..#..
.....
..#..
= '
#
#
.
.
.
.
.
= "
#.#
#.#
...
...
...
...
...
= -
...
...
...
...
###
...
...
= –
....
....
....
....
####
....
....
= —
.......
.......
.......
.......
#######
.......
.......
= +
.....
.....
..#..
..#..
#####
..#..
..#..
= =
....
....
....
####
....
####
....
= /
...#
...#
..#.
..#.
.#..
.#..
#...
= (
..#
.#.
#..
#..
#..
#..
#..
.#.
..#
= )
#..
.#.
..#
..#
..#
..#
..#
.#.
#..
= [
###
#..
#..
#..
#..
#..
#..
#..
###
= ]
###
..#
..#
..#
..#
..#
..#
..#
###
= %
.#...#
#.#.#.
.#..#.
...#..
..#..#
..#.#.#
.#...#.
= &
.##...
#..#..
#..#..
.##..#
#..##.
#...#.
.###.#
= #
.#.#.
#####
.#.#.
.#.#.
#####
.#.#.
.....
= *
.....
#.#.#
.###.
#.#.#
.....
.....
.....
= <
....
...#
..#.
.#..
..#.
...#
....
= >
....
#...
.#..
..#.
.#..
#...
....
= _
.....
.....
.....
.....
.....
.....
.....
.....
#####
= ×
.....
.....
#...#
.#.#.
..#..
.#.#.
#...#
= ·
.
.
.
#
.
.
.
= °
.#.
#.#
.#.
...
...
...
...
= M!
###....###
.###..###.
.##.##.##.
.##.##.##.
.##....##.
.##....##.
####..####
= N!
###..####
.###..##.
.####.##.
.##.####.
.##..###.
.##...##.
####..##.
= W!
###....###
.##....##.
.##.##.##.
.##.##.##.
.##.##.##.
..######..
..##..##..
= k!
###....
.##....
.##.###
.####..
.###...
.####..
###.###
= n!
........
........
######..
.##..##.
.##..##.
.##..##.
###..###
= m!
..........
..........
#########.
.##.##.##.
.##.##.##.
.##.##.##.
###.##.###
= v!
.......
.......
###.###
.##.##.
.##.##.
..###..
..###..
= w!
..........
..........
###....###
.##.##.##.
.##.##.##.
.########.
..##..##..
= y!
.......
.......
###.###
.##.##.
.##.##.
..###..
..##...
.##....
###....
`;

/** The marks of the composed letters, as rows over the base (centred on it). */
const MARKS = {
  acute: [".#", "#."],
  grave: ["#.", ".#"],
  circumflex: [".#.", "#.#"],
  diaeresis: ["#.#"],
  tilde: [".#.#", "#.#."],
  ring: [".#.", "#.#", ".#."],
  cedilla: null,
};
const COMPOSED = {
  "á": ["a", "acute"], "à": ["a", "grave"], "â": ["a", "circumflex"], "ä": ["a", "diaeresis"], "ã": ["a", "tilde"], "å": ["a", "ring"],
  "é": ["e", "acute"], "è": ["e", "grave"], "ê": ["e", "circumflex"], "ë": ["e", "diaeresis"],
  "í": ["ı", "acute"], "ì": ["ı", "grave"], "î": ["ı", "circumflex"], "ï": ["ı", "diaeresis"],
  "ó": ["o", "acute"], "ò": ["o", "grave"], "ô": ["o", "circumflex"], "ö": ["o", "diaeresis"], "õ": ["o", "tilde"],
  "ú": ["u", "acute"], "ù": ["u", "grave"], "û": ["u", "circumflex"], "ü": ["u", "diaeresis"],
  "ñ": ["n", "tilde"], "ý": ["y", "acute"], "ÿ": ["y", "diaeresis"],
  "Á": ["A", "acute"], "À": ["A", "grave"], "Â": ["A", "circumflex"], "Ä": ["A", "diaeresis"], "Å": ["A", "ring"],
  "É": ["E", "acute"], "È": ["E", "grave"], "Ê": ["E", "circumflex"], "Ë": ["E", "diaeresis"],
  "Í": ["I", "acute"], "Î": ["I", "circumflex"], "Ï": ["I", "diaeresis"],
  "Ó": ["O", "acute"], "Ô": ["O", "circumflex"], "Ö": ["O", "diaeresis"],
  "Ú": ["U", "acute"], "Û": ["U", "circumflex"], "Ü": ["U", "diaeresis"], "Ñ": ["N", "tilde"],
};
/** Characters drawn as another glyph (typographic quotes, spaces). */
const SAME = { "’": "'", "‘": "'", "ʼ": "'", "“": '"', "”": '"', "−": "-", " ": " " };

/** The glyph table: char -> { w, rows (9 strings, row 0 the cap line), top (rows above the cap
 *  line, for a capital's mark) }. */
export function glyphTable() {
  return tables().regular;
}

let parsed = null;
/** The regular glyph table and the hand-drawn bolds (char -> glyph). */
function tables() {
  if (parsed) return parsed;
  const out = new Map();
  const bold = new Map();
  let ch = null;
  let rows = [];
  const flush = () => {
    if (ch === null) return;
    const w = Math.max(...rows.map((r) => r.length));
    while (rows.length < 9) rows.push(".".repeat(w));
    const g = { w, rows: rows.map((r) => r.padEnd(w, ".")), top: [] };
    if (ch.length === 2 && ch.endsWith("!")) bold.set(ch[0], g);
    else out.set(ch, g);
  };
  for (const line of SOURCE.split("\n")) {
    if (line.startsWith("= ")) {
      flush();
      ch = line.slice(2);
      rows = [];
    } else if (ch !== null && /^[.#]+$/.test(line)) rows.push(line);
  }
  flush();
  // The dotless i, for the composed i's.
  const i = out.get("i");
  out.set("ı", { ...i, rows: i.rows.map((r, k) => (k === 0 ? ".".repeat(i.w) : r)) });
  for (const [c, [base, mark]] of Object.entries(COMPOSED)) {
    const b = out.get(base);
    const m = MARKS[mark];
    if (!b || !m) continue;
    const w = Math.max(b.w, m[0].length);
    const pad = (r, n) => ".".repeat(Math.floor((w - n) / 2)) + r + ".".repeat(Math.ceil((w - n) / 2));
    const rows = b.rows.map((r) => pad(r, b.w));
    const capital = base === base.toUpperCase();
    if (capital) {
      // One row above the cap line: the mark's last row.
      out.set(c, { w, rows, top: [pad(m[m.length - 1], m[0].length)] });
    } else {
      const g = rows.slice();
      const at = m.length === 1 ? 1 : 2 - m.length;
      m.forEach((r, k) => {
        if (at + k >= 0) g[at + k] = pad(r, r.length);
      });
      out.set(c, { w, rows: g, top: at < 0 ? [pad(m[0], m[0].length)] : [] });
    }
  }
  for (const [c, to] of Object.entries(SAME)) if (out.has(to)) out.set(c, out.get(to));
  parsed = { regular: out, bold };
  return parsed;
}

/** A glyph's bold: drawn by hand, else the smear; a composed letter's base drawn by hand keeps
 *  its mark. */
function boldOf(c, g) {
  const own = tables().bold.get(c);
  if (own) return { ...own, top: [] };
  const base = COMPOSED[c]?.[0];
  const hand = base && tables().bold.get(base);
  if (hand) {
    const b = embolden(g);
    const k = (hand.w - (g.w + 1)) / 2;
    const mark = (r) => (k > 0 ? ".".repeat(Math.floor(k)) + r + ".".repeat(Math.ceil(k)) : r);
    const rows = hand.rows.map((r, y) => (y < 2 && b.rows[y].includes("#") ? mark(b.rows[y]) : r));
    return { w: hand.w, rows, top: b.top.map(mark) };
  }
  return embolden(g);
}

/** The pixel bold of a glyph: each row ORed with itself shifted 1 px right. */
function embolden(g) {
  const smear = (r) => {
    let s = "";
    for (let x = 0; x <= r.length; x++) s += r[x] === "#" || r[x - 1] === "#" ? "#" : ".";
    return s;
  };
  return { w: g.w + 1, rows: g.rows.map(smear), top: g.top.map(smear) };
}

/** A glyph's outline: rectangles of ink (rows' runs, stacked where they repeat) in font units. */
function rects(g) {
  const lines = [...g.top.map((r, k) => ({ r, y: -g.top.length + k })), ...g.rows.map((r, y) => ({ r, y }))];
  const open = new Map(); // "x0,x1" -> rect
  const done = [];
  for (const { r, y } of lines) {
    const runs = [];
    for (let x = 0; x < r.length; ) {
      if (r[x] !== "#") {
        x++;
        continue;
      }
      let e = x;
      while (r[e] === "#") e++;
      runs.push(`${x},${e}`);
      x = e;
    }
    for (const [k, rc] of [...open]) {
      if (!runs.includes(k)) {
        done.push(rc);
        open.delete(k);
      }
    }
    for (const k of runs) {
      if (open.has(k)) open.get(k).y1 = y + 1;
      else {
        const [x0, x1] = k.split(",").map(Number);
        open.set(k, { x0, x1, y0: y, y1: y + 1 });
      }
    }
  }
  done.push(...open.values());
  // Grid rows down from the cap line -> font units up from the baseline.
  return done.map((q) => ({ x0: q.x0 * PX, x1: q.x1 * PX, top: (BASE_ROW + 1 - q.y0) * PX, bottom: (BASE_ROW + 1 - q.y1) * PX }));
}

// --- a minimal TrueType writer ---------------------------------------------------------------

class Buf {
  constructor() {
    this.b = [];
  }
  u8(v) {
    this.b.push(v & 255);
    return this;
  }
  u16(v) {
    return this.u8(v >> 8).u8(v);
  }
  i16(v) {
    return this.u16(v < 0 ? v + 65536 : v);
  }
  u32(v) {
    return this.u16(Math.floor(v / 65536) & 65535).u16(v & 65535);
  }
  bytes(a) {
    for (const x of a) this.b.push(x & 255);
    return this;
  }
  pad4() {
    while (this.b.length % 4) this.b.push(0);
    return this;
  }
  get length() {
    return this.b.length;
  }
}

function checksum(bytes) {
  let s = 0;
  for (let i = 0; i < bytes.length; i += 4) s = (s + ((bytes[i] << 24) | ((bytes[i + 1] ?? 0) << 16) | ((bytes[i + 2] ?? 0) << 8) | (bytes[i + 3] ?? 0))) >>> 0;
  return s;
}

/** The font file (TrueType) of the glyph table, regular or bold. */
export function buildFont({ bold = false } = {}) {
  const table = glyphTable();
  const chars = [...table.keys()].filter((c) => c.length === 1 && c !== "ı").sort((a, b) => a.codePointAt(0) - b.codePointAt(0));
  const glyphs = [{ code: null, adv: 4 * PX, rects: [] }, { code: 32, adv: (SPACE + (bold ? 1 : 0)) * PX, rects: [] }];
  for (const c of chars) {
    const g = bold ? boldOf(c, table.get(c)) : table.get(c);
    glyphs.push({ code: c.codePointAt(0), adv: (g.w + 1) * PX, rects: rects(g) });
  }
  if (!table.has(" ")) glyphs.push({ code: 0xa0, adv: glyphs[1].adv, rects: [] });
  const glyf = new Buf();
  const loca = [];
  let maxPoints = 0;
  let maxContours = 0;
  const bbox = { x0: 0, y0: -3 * PX, x1: 0, y1: 8 * PX };
  for (const g of glyphs) {
    loca.push(glyf.length);
    if (!g.rects.length) continue;
    const xs = g.rects.flatMap((r) => [r.x0, r.x1]);
    const ys = g.rects.flatMap((r) => [r.top, r.bottom]);
    const b = { x0: Math.min(...xs), y0: Math.min(...ys), x1: Math.max(...xs), y1: Math.max(...ys) };
    g.bbox = b;
    bbox.x1 = Math.max(bbox.x1, b.x1);
    bbox.y0 = Math.min(bbox.y0, b.y0);
    bbox.y1 = Math.max(bbox.y1, b.y1);
    glyf.i16(g.rects.length).i16(b.x0).i16(b.y0).i16(b.x1).i16(b.y1);
    g.rects.forEach((_, k) => glyf.u16(k * 4 + 3));
    glyf.u16(0);
    const pts = g.rects.flatMap((r) => [[r.x0, r.top], [r.x1, r.top], [r.x1, r.bottom], [r.x0, r.bottom]]);
    maxPoints = Math.max(maxPoints, pts.length);
    maxContours = Math.max(maxContours, g.rects.length);
    for (let k = 0; k < pts.length; k++) glyf.u8(1);
    let px = 0;
    for (const [x] of pts) glyf.i16(x - px), (px = x);
    let py = 0;
    for (const [, y] of pts) glyf.i16(y - py), (py = y);
    glyf.pad4();
  }
  loca.push(glyf.length);
  const n = glyphs.length;
  const family = "Herald";
  const sub = bold ? "Bold" : "Regular";

  const head = new Buf().u32(0x10000).u32(0x10000).u32(0).u32(0x5f0f3cf5).u16(0x000b).u16(UPM)
    .u32(0).u32(0).u32(0).u32(0)
    .i16(bbox.x0).i16(bbox.y0).i16(bbox.x1).i16(bbox.y1)
    .u16(bold ? 1 : 0).u16(8).i16(2).i16(1).i16(0);
  const advMax = Math.max(...glyphs.map((g) => g.adv));
  const hhea = new Buf().u32(0x10000).i16(8 * PX).i16(-3 * PX).i16(0).u16(advMax).i16(0).i16(0).i16(bbox.x1)
    .i16(1).i16(0).i16(0).i16(0).i16(0).i16(0).i16(0).i16(0).u16(n);
  const maxp = new Buf().u32(0x10000).u16(n).u16(maxPoints).u16(maxContours).u16(0).u16(0).u16(2)
    .u16(0).u16(0).u16(0).u16(0).u16(0).u16(0).u16(0).u16(0);
  const hmtx = new Buf();
  for (const g of glyphs) hmtx.u16(g.adv).i16(g.bbox ? g.bbox.x0 : 0);
  const codes = glyphs.map((g, i) => [g.code, i]).filter(([c]) => c !== null).sort((a, b) => a[0] - b[0]);
  const os2 = new Buf().u16(4).i16(Math.round(glyphs.reduce((s, g) => s + g.adv, 0) / n)).u16(bold ? 700 : 400).u16(5).u16(0)
    .i16(650).i16(600).i16(0).i16(75).i16(650).i16(600).i16(0).i16(350).i16(PX).i16(4 * PX)
    .i16(0).bytes([2, 0, bold ? 8 : 5, 0, 0, 0, 0, 0, 0, 0]).u32(3).u32(0).u32(0).u32(0)
    .bytes([...("FFH2")].map((c) => c.charCodeAt(0)))
    .u16((bold ? 0x20 : 0x40) | 0x80).u16(codes[0][0]).u16(Math.min(0xffff, codes.at(-1)[0]))
    .i16(8 * PX).i16(-3 * PX).i16(0).u16(8 * PX).u16(3 * PX).u32(1).u32(0)
    .i16(5 * PX).i16(7 * PX).u16(0).u16(32).u16(1);
  // cmap: format 4, a segment per character.
  const segs = codes.map(([c, g]) => [c, c, (g - c + 65536) % 65536]).concat([[0xffff, 0xffff, 1]]);
  const sc = segs.length;
  const sr = 2 * 2 ** Math.floor(Math.log2(sc));
  const f4 = new Buf().u16(4).u16(16 + sc * 8).u16(0).u16(sc * 2).u16(sr).u16(Math.log2(sr / 2)).u16(sc * 2 - sr);
  for (const s of segs) f4.u16(s[1]);
  f4.u16(0);
  for (const s of segs) f4.u16(s[0]);
  for (const s of segs) f4.u16(s[2]);
  for (let k = 0; k < sc; k++) f4.u16(0);
  const cmap = new Buf().u16(0).u16(2).u16(0).u16(3).u32(20).u16(3).u16(1).u32(20).bytes(f4.b);
  const names = [[1, family], [2, sub], [3, `${family}-${sub}-ffh2`], [4, `${family} ${sub}`], [6, `${family}-${sub}`]];
  const strs = names.map(([, s]) => [...s].flatMap((c) => [0, c.charCodeAt(0)]));
  const name = new Buf().u16(0).u16(names.length).u16(6 + names.length * 12);
  let off = 0;
  names.forEach(([id], k) => {
    name.u16(3).u16(1).u16(0x409).u16(id).u16(strs[k].length).u16(off);
    off += strs[k].length;
  });
  for (const s of strs) name.bytes(s);
  const post = new Buf().u32(0x30000).u32(0).i16(-2 * PX).i16(PX).u32(0).u32(0).u32(0).u32(0).u32(0);
  const locaB = new Buf();
  for (const o of loca) locaB.u32(o);

  const tables = { "OS/2": os2, cmap, glyf, head, hhea, hmtx, loca: locaB, maxp, name, post };
  const tags = Object.keys(tables).sort();
  const nt = tags.length;
  const es = Math.floor(Math.log2(nt));
  const out = new Buf().u32(0x10000).u16(nt).u16(16 * 2 ** es).u16(es).u16(nt * 16 - 16 * 2 ** es);
  let at = 12 + nt * 16;
  const recs = [];
  for (const t of tags) {
    const data = tables[t].b;
    recs.push({ t, sum: checksum(data), at, len: data.length });
    at += Math.ceil(data.length / 4) * 4;
  }
  for (const r of recs) out.bytes([...r.t].map((c) => c.charCodeAt(0))).u32(r.sum).u32(r.at).u32(r.len);
  for (const t of tags) out.bytes(tables[t].b).pad4();
  const bytes = new Uint8Array(out.b);
  // head.checkSumAdjustment
  const adj = (0xb1b0afba - checksum(bytes)) >>> 0;
  const hp = recs.find((r) => r.t === "head").at + 8;
  bytes[hp] = adj >>> 24;
  bytes[hp + 1] = (adj >>> 16) & 255;
  bytes[hp + 2] = (adj >>> 8) & 255;
  bytes[hp + 3] = adj & 255;
  return bytes.buffer;
}

let installed = null;
/** Adds Herald (regular and bold) to the page's fonts; resolves when both can draw. */
export function installIsoFont() {
  if (installed) return installed;
  installed = (async () => {
    if (typeof FontFace === "undefined" || !globalThis.document?.fonts) return false;
    const faces = [new FontFace("Herald", buildFont(), { weight: "400" }), new FontFace("Herald", buildFont({ bold: true }), { weight: "700" })];
    for (const f of faces) document.fonts.add(await f.load());
    return true;
  })().catch((e) => {
    console.warn("Herald: the interface font did not load, the system serif is used:", e);
    return false;
  });
  return installed;
}
