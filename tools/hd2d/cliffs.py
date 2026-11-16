"""The ground's faces, drawn by code (docs/HD2D.md §5f): the strips of rock the HD-2D map lays on
the sheer faces where its land meets its water and where a hill's crown stands over its scarp
(world.js `cliff`, terrain.js; shaders.js CLIFF_FACE).

The recipe is the stones' (rocks.py): hard tones of one ramp, no gradients. A strip is living
rock broken into blocks of very uneven size -- each pixel belongs to the nearest of a scatter of
points, weighed by the point's size -- each standing out of the face, level with it or set back.
A block that stands out is a tone lighter and its upper and left edges catch the light; the one
beside or below it lies in its shade, the crevice between them dark. Blocks standing level meet
in a change of tone and a broken hairline: no block is ringed in dark (a face of ringed cells
reads as paving). Each block is two facets a step apart; a few cracks, a ledge in reaches across
the face. The rock is lightest under the rim, where the sun reaches it, and darkens toward its
foot. Under the rim a thin line of dark earth, moss below it in a few places.

A strip is 256 art pixels long (it tiles along the face) and ROWS tall, laid down from the rim: a
face shows as many rows as it is high (a shore's about 28, a hill's scarp up to 25, a crown over
the sea more), the last row running on below. Its alpha is not coverage: it is the lip, the turf
that hangs over the rim, as a shade (0: rock shows; else the ground's own colour times this),
four to seven rows deep, darker downward, ragged below like blades -- deep enough that a face a
few pixels high, where a scarp tapers out, is all turf. Two strips, one above the other, for the
faces that look south or north and those that look east or west.

  python3 tools/hd2d/cliffs.py [--sheet OUT.png]   # writes art/gnd_face.png

Deterministic (seeded); numpy + Pillow.
"""
import math
import random
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "crates" / "ffh-play" / "assets" / "app" / "r3d" / "hd2d" / "art"

W = 256
ROWS = 64  # a strip's rows (hd2d/index.js CLIFF_ART)


def rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


# Ramps: 0 a crack, 1..5 dark to light, 6 an edge catching the sun (the cards' colour space: the
# map's finish shows the middle tone #686561 as about #807c78).
STONE = [rgb(c) for c in ("#1b191b", "#302f31", "#4a4849", "#686561", "#89847a", "#aba597", "#cac6b7")]
EARTH = [rgb(c) for c in ("#20160e", "#33251a", "#4a3826")]
MOSS = [rgb(c) for c in ("#2c4420", "#44632c", "#5f7f38")]


def strip(seed):
    """One strip: (rgb uint8 [ROWS, W, 3], lip shade uint8 [ROWS, W])."""
    rng = random.Random(seed)
    yy, xx = np.mgrid[0:ROWS, 0:W]
    # The blocks: points scattered over the strip, each with a size of its own (a few large
    # blocks among many small ones) and a way it stands: out of the face, level, or set back.
    pts = []
    tries = 0
    while len(pts) < 80 and tries < 8000:
        tries += 1
        x, y = rng.uniform(0, W), rng.uniform(-8, ROWS + 8)
        size = rng.choice((0.6, 0.75, 0.9, 1.0, 1.2, 1.5, 1.9))
        ok = True
        for px, py, ps, _ in pts:
            dx = min(abs(x - px), W - abs(x - px))
            if (dx / 7.0) ** 2 + ((y - py) / 10.0) ** 2 < (0.75 * (size + ps)) ** 2:
                ok = False
                break
        if ok:
            pts.append((x, y, size, rng.choice((0, 1, 1, 2))))
    best = np.full((ROWS, W), 1e9)
    sid = np.zeros((ROWS, W), int)
    for k, (px, py, ps, _) in enumerate(pts):
        dx = np.abs(xx + 0.5 - px)
        dx = np.minimum(dx, W - dx)  # the strip tiles
        d = ((dx / 6.0) ** 2 + ((yy + 0.5 - py) / 9.0) ** 2) / (ps * ps)
        near = d < best
        best[near] = d[near]
        sid[near] = k
    out_of = np.array([p[3] for p in pts])[sid]
    # A block's tone: by how it stands, each of its two facets a step apart.
    tone = np.zeros((ROWS, W), int)
    for k, (px, py, ps, rel) in enumerate(pts):
        m = sid == k
        if not m.any():
            continue
        dxs = (xx + 0.5 - px + W / 2) % W - W / 2
        a = rng.uniform(-0.6, 0.6) + (math.pi / 2 if rng.random() < 0.65 else 0)
        which = (dxs * math.cos(a) + (yy + 0.5 - py) * math.sin(a)) > rng.uniform(-2, 2) * ps
        t = 2 + rel + rng.choice((0, 0, 1))
        lighter = rng.random() < 0.5
        tone[m & which] = min(5, t + (1 if lighter else 0))
        tone[m & ~which] = min(5, max(1, t - (0 if lighter else 1)))
    # Where a block stands out of the one above or to its left, its edge there catches the light;
    # where the block to its left or above stands out of it, it lies in that one's shade, and the
    # crevice between them is dark. Blocks standing level meet without a line.
    left, up = np.roll(out_of, 1, axis=1), np.vstack([out_of[:1], out_of[:-1]])
    lsid, usid = np.roll(sid, 1, axis=1), np.vstack([sid[:1], sid[:-1]])
    edge_l, edge_u = lsid != sid, usid != sid
    lit = ((edge_l & (out_of > left)) | (edge_u & (out_of > up)))
    tone = np.where(lit, np.minimum(tone + 1, 6), tone)
    shade = ((edge_l & (out_of < left)) | (edge_u & (out_of < up)))
    tone = np.where(shade, 0, tone)
    cast = np.roll(shade, 1, axis=1) & ~shade & (sid == np.roll(sid, 1, axis=1))
    tone = np.where(cast & (tone > 1), tone - 1, tone)
    # Level blocks: a hairline between some of them, broken (never a closed ring).
    hair = (edge_l | edge_u) & (out_of == np.where(edge_l, left, up)) & (((xx * 7 + yy * 13 + sid * 5) % 11) < 4)
    tone = np.where(hair & (tone > 1), tone - 1, tone)
    # A ledge across the face: a wavering line of shade with a lit edge under it, in reaches.
    for base in (rng.uniform(15, 20), rng.uniform(36, 44)):
        ph1, ph2 = rng.uniform(0, 6.28), rng.uniform(0, 6.28)
        for x in range(W):
            y = int(round(base + 1.8 * math.sin(x * 2 * math.pi * 3 / W + ph1) + 1.0 * math.sin(x * 2 * math.pi * 7 / W + ph2)))
            if 0 <= y < ROWS - 1 and math.sin(x * 2 * math.pi * 5 / W + ph1 * 2) > 0.1:
                if tone[y, x] > 1:
                    tone[y, x] -= 1
                if 2 <= tone[y + 1, x] < 6:
                    tone[y + 1, x] += 1
    # Cracks: short dark dashes down the lighter blocks.
    for _ in range(46):
        x, y = rng.randrange(W), rng.randrange(8, ROWS - 5)
        dx = rng.choice((-1, 0, 1))
        for s in range(rng.choice((2, 3, 3, 4))):
            px, py = (x + dx * (s // 2)) % W, y + s
            if tone[py, px] >= 3 and sid[py, px] == sid[y, x]:
                tone[py, px] -= 2
    # Lightest under the rim, where the sun reaches; darker toward the foot (the bands' edges
    # follow the blocks: a block is lit or shaded whole, by where its middle lies).
    mid = np.array([p[1] for p in pts])[sid]
    tone = np.where((mid < 13) & (tone >= 2) & (tone < 6), tone + 1, tone)
    tone = np.where((mid >= 33) & (tone > 1), tone - 1, tone)
    tone = np.where((mid >= 50) & (tone > 1), tone - 1, tone)
    out = np.zeros((ROWS, W, 3), np.uint8)
    for t in range(7):
        out[tone == t] = STONE[t]
    # The lip: turf hanging over the rim, four to seven pixels deep, in blades (single columns
    # that hang a pixel or two lower), darker downward: its lowest pixels are the shade under it.
    lip = np.zeros((ROWS, W), np.uint8)
    depth = 5.0
    lipd = np.zeros(W, int)
    for x in range(W):
        depth = min(max(depth + rng.uniform(-0.9, 0.9), 4.0), 6.4)
        d = int(round(depth)) + (2 if rng.random() < 0.14 else 1 if rng.random() < 0.25 else 0)
        lipd[x] = d
        for y in range(d):
            lip[y, x] = 255 if y == 0 else 225 if y < d - 3 else 190 if y < d - 1 else 140
    # Under the lip a thin line of the earth the turf grows in, and the rock a tone darker in the
    # lip's shade.
    for x in range(W):
        d0 = lipd[x]
        de = 1 if rng.random() < 0.7 else 2
        for y in range(d0, min(ROWS, d0 + de)):
            out[y, x] = EARTH[0 if y == d0 else 1]
        y = d0 + de
        if y < ROWS:
            out[y, x] = (out[y, x] * 0.72).astype(np.uint8)
    # Moss: a few patches hanging below the earth, where water seeps.
    for _ in range(4):
        cx, w = rng.randrange(W), rng.randint(5, 11)
        for i in range(w):
            x = (cx + i) % W
            top = lipd[x] + 1
            hang = int(round(2 + 3.0 * math.sin(math.pi * (i + 0.5) / w) + rng.uniform(-1, 1)))
            for y in range(top, min(ROWS, top + max(1, hang))):
                out[y, x] = MOSS[2 if y == top else 1 if y < top + hang - 1 else 0]
    return out, lip


def build():
    strips = [strip(7101), strip(7102)]
    img = np.zeros((ROWS * 2, W, 4), np.uint8)
    for k, (c, lip) in enumerate(strips):
        img[k * ROWS:(k + 1) * ROWS, :, :3] = c
        img[k * ROWS:(k + 1) * ROWS, :, 3] = lip
    return img


def main():
    img = build()
    ART.mkdir(parents=True, exist_ok=True)
    Image.fromarray(img, "RGBA").save(ART / "gnd_face.png", optimize=True)
    if "--sheet" in sys.argv:
        # For looking at: the strips at 4x on a meadow green, the lip painted as the shader does.
        out = sys.argv[sys.argv.index("--sheet") + 1]
        grass = np.array([113, 149, 70], float)
        rgbf = img[:, :, :3].astype(float)
        a = img[:, :, 3:4].astype(float) / 255
        shown = np.where(a > 0, grass * (0.5 + 0.5 * a), rgbf).astype(np.uint8)
        Image.fromarray(shown, "RGB").resize((shown.shape[1] * 4, shown.shape[0] * 4), Image.NEAREST).save(out)
    print("gnd_face.png", img.shape[1], "x", img.shape[0])


if __name__ == "__main__":
    main()
