"""The woods' tree cards of the HD-2D mode, drawn by code (docs/HD2D.md §5b): broadleaf trees,
pines and snowy pines, about 45 art pixels tall.

The recipe is the owner's isometric tile sketch (terrain_tiles.py, 2026-11-14): a crown is a
cluster of spheres, the nearest surface at each pixel shaded from the upper left into five hard
tones of one ramp (no gradients, no noise); a nearer lump throws a crease of shade on the one
behind it; leaves are two-pixel dashes a tone lighter or darker; and a one-pixel outline lies
inside the silhouette, dark on the shaded side and softer on the lit rim. A broadleaf crown is
eight lumps round a middle, a conifer is tiers of lumps, the upper tiers in front so their
undersides shade the tier below. Trunks are a few columns of bark with a foot.

The cards carry their own outline (no second one is added). Deterministic (seeded);
std + numpy + Pillow.

  python3 tools/hd2d/trees.py [--sheet OUT.png]   # every tree card, then `art.py index`
"""
import math
import random
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "crates" / "ffh-play" / "assets" / "app" / "r3d" / "hd2d" / "art"


def rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


# Ramps: 0 the outline, 1..5 dark to light, 6 a leaf catching the sun.
LEAF = [rgb(c) for c in ("#0b1408", "#1b2a0e", "#33491a", "#4e6b22", "#71902f", "#9cb746", "#c4d968")]
LEAF_DEEP = [rgb(c) for c in ("#0a1409", "#152810", "#28441c", "#3f6425", "#5d8832", "#86ad48", "#b2d068")]
LEAF_WARM = [rgb(c) for c in ("#12140a", "#2a2c0e", "#4a4c18", "#6c6e20", "#92902c", "#bab444", "#dcd468")]
CONIFER = [LEAF[0]] + [rgb(c) for c in ("#16290f", "#27461a", "#3b6424", "#578530", "#7fa844", "#a9c95e")]
SNOWY = CONIFER[:4] + [rgb(c) for c in ("#8aa6c2", "#cfdeea", "#fbfdff")]
BARK = [rgb(c) for c in ("#1e120a", "#4a2e17", "#6f4a25", "#94683a")]

LIGHT = np.array([-0.55, -0.62, 0.56])
LIGHT = LIGHT / np.linalg.norm(LIGHT)


def blob_tones(spheres, seed, bias=0.0, speck=0.13):
    """Shade a cluster of spheres as lumpy foliage: (tone map, origin x, origin y). Sphere
    coordinates are relative to the trunk's base, y up is negative, z is toward the viewer."""
    rng = random.Random(seed)
    x0 = math.floor(min(s[0] - s[3] for s in spheres)) - 1
    x1 = math.ceil(max(s[0] + s[3] for s in spheres)) + 1
    y0 = math.floor(min(s[1] - s[3] for s in spheres)) - 1
    y1 = math.ceil(max(s[1] + s[3] for s in spheres)) + 1
    w, h = x1 - x0, y1 - y0
    yy, xx = np.mgrid[0:h, 0:w]
    X, Y = xx + x0 + 0.5, yy + y0 + 0.5
    zbuf = np.full((h, w), -1e9)
    ids = np.full((h, w), -1, int)
    shade = np.zeros((h, w))
    for k, (sx, sy, sz, r) in enumerate(spheres):
        d2 = (X - sx) ** 2 + (Y - sy) ** 2
        inside = d2 < r * r
        zz = np.sqrt(np.clip(r * r - d2, 0, None))
        upd = inside & (sz + zz > zbuf)
        zbuf[upd] = (sz + zz)[upd]
        ids[upd] = k
        s = ((X - sx) * LIGHT[0] + (Y - sy) * LIGHT[1] + zz * LIGHT[2]) / r
        shade[upd] = s[upd]
    mask = ids >= 0
    ys = np.nonzero(mask.any(axis=1))[0]
    vrel = np.clip((yy - ys[0]) / max(1, ys[-1] - ys[0]), 0, 1)
    s2 = shade - 0.24 * vrel + 0.06 + bias
    tone = np.where(s2 > 0.80, 5, np.where(s2 > 0.55, 4, np.where(s2 > 0.28, 3, np.where(s2 > 0.02, 2, 1))))
    tone[~mask] = -1
    # The shade a nearer lump throws on the one behind it, on the side away from the light.
    crease = np.zeros((h, w), bool)
    for dx, dy in ((1, 0), (0, 1), (1, 1)):
        nid = np.roll(np.roll(ids, dx, axis=1), dy, axis=0)
        nz = np.roll(np.roll(zbuf, dx, axis=1), dy, axis=0)
        crease |= mask & (nid >= 0) & (nid != ids) & (nz > zbuf + 1.2)
    tone[crease] = 1
    # Leaves, in two-pixel dashes.
    for y in range(h):
        for x in range(w - 1):
            if tone[y, x] < 2 or tone[y, x + 1] != tone[y, x] or crease[y, x] or crease[y, x + 1]:
                continue
            r = rng.random()
            if r < speck:
                tone[y, x] += 1
                tone[y, x + 1] += 1
            elif r < speck * 1.7 and tone[y, x] >= 3:
                tone[y, x] -= 1
                tone[y, x + 1] -= 1
    # The outline, one pixel, inside the silhouette; softer on the lit rim.
    pad = np.pad(mask, 1)
    inner = pad[:-2, 1:-1] & pad[2:, 1:-1] & pad[1:-1, :-2] & pad[1:-1, 2:]
    edge = mask & ~inner
    tone[edge & (shade <= 0.45)] = 0
    tone[edge & (shade > 0.45)] = 1
    return tone, x0, y0


def jitter(spheres, rng, amt=1.2, scale=1.0):
    return [((x + rng.uniform(-amt, amt)) * scale, (y + rng.uniform(-amt, amt)) * scale, z * scale,
             (r + rng.uniform(-0.6, 0.6)) * scale) for (x, y, z, r) in spheres]


ROUND = [  # x, y, z, r: a broadleaf crown
    (0, -31, 2, 9), (-7, -26, 3, 8), (7, -27, 1, 8),
    (-9, -18, 2, 7), (9, -19, 0, 7), (0, -21, 7, 9),
    (-5, -13, 5, 7), (5, -14, 4, 7),
]
WIDE = [  # a spreading crown
    (-3, -29, 2, 8), (6, -28, 1, 8), (-12, -22, 2, 7.5), (13, -22, 0, 7),
    (-14, -15, 1, 6.5), (14, -15, 0, 6.5), (0, -21, 7, 9), (-6, -13, 5, 7), (6, -14, 4, 7),
]
TALL = [  # a high crown
    (1, -39, 2, 7.5), (-5, -33, 3, 8), (6, -32, 1, 7.5), (0, -27, 6, 8.5),
    (-8, -22, 2, 7), (8, -22, 0, 7), (-4, -15, 5, 7), (5, -15, 4, 7),
]
CONE = [  # tiers; upper tiers sit in front, so their undersides shade the tier below
    (0, -41, 9, 4.5),
    (-3, -35, 6, 5), (3, -35, 5, 5), (0, -34, 7.5, 5),
    (-5.5, -28, 4, 5.5), (5.5, -28, 3, 5.5), (0, -27, 6, 6),
    (-8, -20, 2, 6), (8, -20, 1, 6), (-2.5, -19, 4, 6), (3, -19, 3.5, 6),
    (-10, -13, 0, 6), (10, -13, -1, 6), (-4, -12, 2, 6.5), (4, -12, 1.5, 6.5),
]
SPIRE = [(0, -49, 10, 4)] + [(x, y - 7 if y < -30 else y - 3 if y < -24 else y, z, r) for (x, y, z, r) in CONE]


def paint_tones(img, tone, ox, oy, ramp):
    H, W = img.shape[:2]
    for y, x in zip(*np.nonzero(tone >= 0)):
        px, py = ox + int(x), oy + int(y)
        if 0 <= px < W and 0 <= py < H:
            img[py, px] = (*ramp[min(int(tone[y, x]), 6)], 255)


def paint_trunk(img, bx, by, height):
    cols = [(-2, 0), (-1, 3), (0, 2), (1, 1), (2, 0)]
    for y in range(by - height, by - 1):
        for dx, c in cols:
            img[y, bx + dx] = (*BARK[c], 255)
    for dx, c in [(-3, 0), (-2, 3), (-1, 3), (0, 2), (1, 1), (2, 1), (3, 0)]:
        img[by - 1, bx + dx] = (*BARK[c], 255)
    for dx in range(-2, 3):
        img[by, bx + dx] = (*BARK[0], 255)


def tree(name, shape, seed, ramp, conifer=False, scale=1.0):
    """One tree card: the trunk, then the crown over it; cropped, a clear pixel round it."""
    rng = random.Random(seed)
    img = np.zeros((96, 96, 4), np.uint8)
    bx, by = 48, 90
    spheres = jitter(shape, rng, 0.7 if conifer else 1.2, scale)
    bottom = max(s[1] + s[3] for s in spheres)
    paint_trunk(img, bx, by, int(-bottom) + 5)
    tone, x0, y0 = blob_tones(spheres, seed, bias=-0.04 if conifer else 0.0)
    paint_tones(img, tone, bx + x0, by + y0, ramp)
    ys, xs = np.nonzero(img[..., 3])
    out = np.zeros((ys.max() - ys.min() + 3, xs.max() - xs.min() + 3, 4), np.uint8)
    out[1:-1, 1:-1] = img[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
    Image.fromarray(out, "RGBA").save(ART / f"{name}.png", optimize=True)
    print(f"{name}: card {out.shape[1]}x{out.shape[0]}")


CARDS = [
    lambda: tree("tree_oak_1", ROUND, 511, LEAF),
    lambda: tree("tree_oak_2", WIDE, 512, LEAF_DEEP),
    lambda: tree("tree_oak_3", TALL, 513, LEAF),
    lambda: tree("tree_oak_4", ROUND, 514, LEAF_WARM, scale=0.9),
    lambda: tree("tree_oak_5", TALL, 515, LEAF_DEEP, scale=1.05),
    lambda: tree("tree_pine_1", CONE, 521, CONIFER, conifer=True),
    lambda: tree("tree_pine_2", SPIRE, 522, CONIFER, conifer=True),
    lambda: tree("tree_pine_3", CONE, 523, CONIFER, conifer=True, scale=0.9),
    lambda: tree("tree_pine_snow_1", CONE, 531, SNOWY, conifer=True),
    lambda: tree("tree_pine_snow_2", SPIRE, 532, SNOWY, conifer=True),
]


def sheet(out, k=6):
    """A contact sheet of this file's tree cards, `k` times enlarged, on the meadow's green."""
    names = sorted(p.stem for p in ART.glob("tree_*.png") if "oak" in p.stem or "pine" in p.stem)
    ims = [Image.open(ART / f"{n}.png").convert("RGBA") for n in names]
    W = sum(im.width + 6 for im in ims) + 6
    H = max(im.height for im in ims) + 10
    bg = Image.new("RGBA", (W, H), (98, 130, 42, 255))
    x = 6
    for im in ims:
        bg.alpha_composite(im, (x, H - 5 - im.height))
        x += im.width + 6
    bg.resize((W * k, H * k), Image.Resampling.NEAREST).save(out)
    print(f"sheet: {out}")


def main():
    for c in CARDS:
        c()
    subprocess.run([sys.executable, str(Path(__file__).resolve().parent / "art.py"), "index"], check=True)
    if "--sheet" in sys.argv:
        sheet(sys.argv[sys.argv.index("--sheet") + 1])


if __name__ == "__main__":
    main()
