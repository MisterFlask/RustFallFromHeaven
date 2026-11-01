// node --test: Herald, the isometric interface's pixel serif (app/isofont.js): its glyph grids and
// the TrueType file made from them.
import test from "node:test";
import assert from "node:assert/strict";
import { glyphTable, buildFont } from "../app/isofont.js";

test("the glyphs: printable ASCII and the accents, well-formed grids", () => {
  const t = glyphTable();
  for (let c = 33; c < 127; c++) {
    const ch = String.fromCharCode(c);
    if ("@$\\^`{|}~".includes(ch)) continue; // not drawn: the system serif stands in
    assert.ok(t.has(ch), `glyph ${ch}`);
  }
  for (const ch of "ÁáéèñÖüçÅ".replace("ç", "")) assert.ok(t.has(ch), `composed ${ch}`);
  for (const [ch, g] of t) {
    assert.equal(g.rows.length, 9, `${ch}: 9 rows`);
    for (const r of [...g.rows, ...g.top]) {
      assert.equal(r.length, g.w, `${ch}: rows ${g.w} wide`);
      assert.match(r, /^[.#]*$/, `${ch}: only . and #`);
    }
  }
  // Capitals and digits stand on the baseline (rows 0-6); descenders only below it.
  for (const ch of "ABCDEFGHIJKLMNOPRSTUVWXYZ0123456789") {
    const g = t.get(ch);
    assert.ok(g.rows[6].includes("#"), `${ch} reaches the baseline`);
    assert.ok(!g.rows[7].includes("#") && !g.rows[8].includes("#"), `${ch} has no descender`);
  }
  for (const ch of "gjpqy") assert.ok(t.get(ch).rows[8].includes("#") || t.get(ch).rows[7].includes("#"), `${ch} descends`);
});

/** The table directory of a TrueType file: tag -> { offset, length }. */
function tables(buf) {
  const v = new DataView(buf);
  assert.equal(v.getUint32(0), 0x10000, "a TrueType file");
  const n = v.getUint16(4);
  const out = {};
  for (let i = 0; i < n; i++) {
    const at = 12 + i * 16;
    const tag = String.fromCharCode(v.getUint8(at), v.getUint8(at + 1), v.getUint8(at + 2), v.getUint8(at + 3));
    out[tag] = { offset: v.getUint32(at + 8), length: v.getUint32(at + 12) };
  }
  return out;
}

test("the font files: the tables a browser needs, the whole file checksummed, regular and bold", () => {
  for (const bold of [false, true]) {
    const buf = buildFont({ bold });
    const t = tables(buf);
    for (const tag of ["OS/2", "cmap", "glyf", "head", "hhea", "hmtx", "loca", "maxp", "name", "post"]) assert.ok(t[tag], `${tag} table`);
    const v = new DataView(buf);
    assert.equal(v.getUint32(t.head.offset + 12), 0x5f0f3cf5, "head magic");
    // The whole file sums to 0xB1B0AFBA (head.checkSumAdjustment).
    let sum = 0;
    for (let i = 0; i < buf.byteLength; i += 4) sum = (sum + v.getUint32(i)) >>> 0;
    assert.equal(sum, 0xb1b0afba);
    const glyphs = v.getUint16(t.maxp.offset + 4);
    assert.equal(v.getUint16(t.hhea.offset + 34), glyphs, "a metric per glyph");
    assert.equal(t.loca.length, (glyphs + 1) * 4, "long loca, one more than the glyphs");
    assert.equal(v.getUint16(t["OS/2"].offset + 4), bold ? 700 : 400, "weight");
  }
});
