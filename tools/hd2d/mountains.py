"""The mountain cards of the HD-2D mode, drawn by code (docs/HD2D.md §5b): what stands on a peak
plot, about 150 art pixels wide.

The recipe is the trees' and the stones' (trees.py, rocks.py) at a mountain's size: hard tones of
one ramp, no gradients, an outline inside the silhouette. A mountain is modelled, not painted: a
few horns, each a summit with spokes running down from it, alternately ridges (long, convex) and
gullies (short, concave), their vertices jittered so the ridges zigzag; the triangles between
them are flat facets. The model is drawn as the map's camera sees it (from about 32 degrees
above the horizon), each facet one tone by the way it faces the sun at the upper left: lit
western faces, shaded eastern ones, the ridges between them sharp. Above a ragged snowline the
facets take the snow's ramp (white in the sun, blue in the shade); a ridge's lit edge carries a
highlight, a nearer spur a crease of shade on what stands behind it, and the rock a few strata
dashes.

The cards carry their own outline (no second one is added). Deterministic (seeded);
std + numpy + Pillow.

  python3 tools/hd2d/mountains.py [--sheet OUT.png]   # every mountain card, then `art.py index`
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


# Ramps: 0 the outline, 1..5 dark to light, 6 an edge catching the sun.
ROCK = [rgb(c) for c in ("#1a1720", "#3a3440", "#584f59", "#796d6d", "#9c8f81", "#beb097", "#dfd3b3")]
SNOW = [rgb(c) for c in ("#27304a", "#4c5d86", "#7389b2", "#a3b9d6", "#d3e0ee", "#f6f9fc", "#ffffff")]
SAND = [rgb(c) for c in ("#1c130e", "#3a2619", "#5e3d25", "#875a33", "#b07d46", "#d2a362", "#eccb8a")]
ASH = [rgb(c) for c in ("#0c0a0e", "#1c181e", "#2e282e", "#453c40", "#5f5454", "#7d6f6a", "#a08f84")]
EMBER = [rgb(c) for c in ("#3a0c04", "#7a1a06", "#b8340a", "#e8621a", "#ff9a30", "#ffc860", "#ffe8a0")]

ELEV = math.radians(32)  # the camera over the horizon
SUN = np.array([-0.62, 0.30, 0.72])  # x east, y toward the viewer, z up
SUN = SUN / np.linalg.norm(SUN)
SIZE = 256


def horn(rng, cx, cy, height, radius, spokes=12, rings=6):
    """A summit and its flanks as triangles [(p0, p1, p2)], points (x, y, z): spokes run down
    from the top, alternately ridges and gullies, a ring of vertices at each step down. Ridges
    are major (long, with a shoulder) or minor; every vertex is pushed off its spoke, so a ridge
    zigzags and its flanks break into small facets."""
    top = np.array([cx + rng.uniform(-2, 2), cy, height])
    a0 = rng.uniform(0, math.tau)
    lines = []
    for s in range(spokes):
        ridge = s % 2 == 0
        major = ridge and (s % 4 == 0 or rng.random() < 0.3)
        a = a0 + math.tau * (s + rng.uniform(-0.25, 0.25)) / spokes
        reach = radius * (rng.uniform(0.98, 1.15) if major else rng.uniform(0.8, 0.95) if ridge else rng.uniform(0.62, 0.78))
        # A shoulder: a step on a major ridge where it holds its height before dropping again.
        knee = rng.uniform(0.3, 0.6)
        pts = []
        for k in range(1, rings + 1):
            t = k / rings
            if ridge:
                z = height * (1 - t) ** (0.8 if major else 1.05)
                if major and abs(t - knee) < 0.6 / rings:
                    z *= 1.12
                z *= rng.uniform(0.93, 1.07)
            else:
                z = height * (1 - t) ** 1.9 * rng.uniform(0.9, 1.05)
            ak = a + rng.uniform(-0.2, 0.2) * (1.0 if k < rings else 0.4)
            r = reach * (t ** 0.9) * rng.uniform(0.93, 1.07)
            if k == rings:
                # The foot is a rough round, not a star: every spoke ends near the same reach.
                r = radius * (rng.uniform(0.96, 1.06) if ridge else rng.uniform(0.86, 0.96))
            pts.append(np.array([cx + math.cos(ak) * r, cy + math.sin(ak) * r * 0.8, max(0.0, z) if k < rings else 0.0]))
        lines.append(pts)
    tris = []
    for s in range(spokes):
        a, b = lines[s], lines[(s + 1) % spokes]
        tris.append((top, a[0], b[0]))
        for k in range(rings - 1):
            # Split each quad along the diagonal that keeps the ridge line an edge.
            if s % 2 == 0:
                tris += [(a[k], a[k + 1], b[k]), (a[k + 1], b[k + 1], b[k])]
            else:
                tris += [(a[k], a[k + 1], b[k + 1]), (a[k], b[k + 1], b[k])]
    return tris


def draw(name, seed, horns, rock=ROCK, snow=SNOW, snowline=0.5, cracks=None):
    """One mountain card from `horns` [(x, y, height, radius)]: y is depth, toward the viewer."""
    rng = random.Random(seed)
    tris = []
    for (x, y, h, r) in horns:
        tris += horn(rng, x, y, h, r, spokes=12 if r < 40 else 16, rings=5 if r < 40 else 7)
    top = max(h for (_, _, h, _) in horns)
    ce, se = math.cos(ELEV), math.sin(ELEV)
    ox, oy = SIZE // 2, SIZE - 70
    yy, xx = np.mgrid[0:SIZE, 0:SIZE]
    PX, PY = xx + 0.5, yy + 0.5
    zbuf = np.full((SIZE, SIZE), -1e9)
    tone = np.full((SIZE, SIZE), -1, int)
    face = np.full((SIZE, SIZE), -1, int)
    lit = np.zeros((SIZE, SIZE))
    alt = np.zeros((SIZE, SIZE))
    snowy = np.zeros((SIZE, SIZE), bool)
    for f, tri in enumerate(tris):
        n = np.cross(tri[1] - tri[0], tri[2] - tri[0])
        if n[2] < 0:
            n = -n
        ln = np.linalg.norm(n)
        if ln < 1e-6:
            continue
        n = n / ln
        shade = float(n @ SUN) + rng.uniform(-0.04, 0.04)
        # To the screen: x east, down the screen is toward the viewer and down the mountain.
        sx = [ox + p[0] for p in tri]
        sy = [oy + p[1] * se - p[2] * ce for p in tri]
        dz = [p[1] * ce + p[2] * se for p in tri]
        x0, x1 = max(0, int(min(sx)) - 1), min(SIZE, int(max(sx)) + 2)
        y0, y1 = max(0, int(min(sy)) - 1), min(SIZE, int(max(sy)) + 2)
        den = (sy[1] - sy[2]) * (sx[0] - sx[2]) + (sx[2] - sx[1]) * (sy[0] - sy[2])
        if abs(den) < 1e-9 or x0 >= x1 or y0 >= y1:
            continue
        px, py = PX[y0:y1, x0:x1], PY[y0:y1, x0:x1]
        w0 = ((sy[1] - sy[2]) * (px - sx[2]) + (sx[2] - sx[1]) * (py - sy[2])) / den
        w1 = ((sy[2] - sy[0]) * (px - sx[2]) + (sx[0] - sx[2]) * (py - sy[2])) / den
        w2 = 1 - w0 - w1
        e = -0.02
        d = w0 * dz[0] + w1 * dz[1] + w2 * dz[2]
        upd = (w0 >= e) & (w1 >= e) & (w2 >= e) & (d > zbuf[y0:y1, x0:x1])
        z = w0 * tri[0][2] + w1 * tri[1][2] + w2 * tri[2][2]
        t = 5 if shade > 0.82 else 4 if shade > 0.58 else 3 if shade > 0.32 else 2 if shade > 0.02 else 1
        # The snowline is ragged: lower on a facet turned up or north, per facet.
        line = top * (snowline + rng.uniform(-0.07, 0.07) - 0.1 * (n[2] - 0.6))
        for arr, val in ((zbuf, d), (alt, z)):
            arr[y0:y1, x0:x1][upd] = val[upd]
        tone[y0:y1, x0:x1][upd] = t
        face[y0:y1, x0:x1][upd] = f
        lit[y0:y1, x0:x1][upd] = shade
        snowy[y0:y1, x0:x1][upd] = (z > line)[upd] if snow else False
    mask = tone >= 0
    # A lit facet's edge against a darker facet to its right or below catches the light: the
    # ridge lines. And a crease of shade where a nearer spur stands before what is behind it.
    for dx, dy in ((-1, 0), (0, -1)):
        nt = np.roll(np.roll(tone, dx, axis=1), dy, axis=0)
        nf = np.roll(np.roll(face, dx, axis=1), dy, axis=0)
        tone = np.where(mask & (nf != face) & (nt >= 0) & (nt < tone - 1) & (tone >= 3), tone + 1, tone)
    crease = np.zeros_like(mask)
    for dx, dy in ((1, 0), (0, 1), (-1, 0)):
        nf = np.roll(np.roll(face, dx, axis=1), dy, axis=0)
        nz = np.roll(np.roll(zbuf, dx, axis=1), dy, axis=0)
        crease |= mask & (nf >= 0) & (nf != face) & (nz > zbuf + 5.0)
    tone[crease] = np.minimum(tone[crease], 1)
    # Strata: short dashes a tone darker on the rock, a few blue ones on the snow.
    ys, xs = np.nonzero(mask & (tone >= 2) & ~crease)
    for _ in range(len(ys) // 70):
        i = rng.randrange(len(ys))
        x, y = int(xs[i]), int(ys[i])
        if snowy[y, x] and rng.random() < 0.7:
            continue
        for s in range(rng.choice((2, 3, 4))):
            if x + s < SIZE and face[y, x + s] == face[y, x] and tone[y, x + s] >= 2:
                tone[y, x + s] -= 1
    # Glowing cracks (hell's mountains): thin lines running down the gullies.
    glow = np.zeros_like(mask)
    if cracks:
        for _ in range(cracks):
            i = rng.randrange(len(ys))
            x, y = int(xs[i]), int(ys[i])
            for s in range(rng.randint(8, 22)):
                if not (0 <= x < SIZE and y < SIZE and mask[y, x]):
                    break
                glow[y, x] = True
                y += 1
                x += rng.choice((-1, 0, 0, 1))
    # The outline, one pixel, inside the silhouette; softer on the lit rim.
    pad = np.pad(mask, 1)
    inner = pad[:-2, 1:-1] & pad[2:, 1:-1] & pad[1:-1, :-2] & pad[1:-1, 2:]
    edge = mask & ~inner
    tone[edge & (lit <= 0.5)] = 0
    tone[edge & (lit > 0.5)] = 1
    img = np.zeros((SIZE, SIZE, 4), np.uint8)
    for y, x in zip(*np.nonzero(mask)):
        t = min(int(tone[y, x]), 6)
        if glow[y, x] and not edge[y, x]:
            img[y, x] = (*EMBER[min(6, t + 2)], 255)
        else:
            # Snow throws the light back: a tone lighter than rock turned the same way.
            on_snow = bool(snow) and snowy[y, x]
            img[y, x] = (*(snow[min(6, t + 1) if t >= 1 else 0] if on_snow else rock[t]), 255)
    ys, xs = np.nonzero(mask)
    out = np.zeros((ys.max() - ys.min() + 3, xs.max() - xs.min() + 3, 4), np.uint8)
    out[1:-1, 1:-1] = img[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
    Image.fromarray(out, "RGBA").save(ART / f"{name}.png", optimize=True)
    print(f"{name}: card {out.shape[1]}x{out.shape[0]}")


# Horns: x, depth (toward the viewer), height, radius.
LONE = [(0, 0, 104, 64), (34, 16, 52, 34)]
TWIN = [(-22, 0, 96, 50), (26, 6, 78, 46), (0, 26, 40, 34)]
MASSIF = [(-36, 2, 70, 40), (2, -4, 100, 54), (40, 8, 62, 38), (-8, 28, 38, 32)]
HORN = [(4, 0, 116, 56), (-36, 14, 48, 32), (38, 18, 44, 30)]
RANGE = [(-44, 0, 64, 36), (-10, 4, 88, 44), (30, -2, 76, 42), (56, 14, 40, 26), (-26, 26, 34, 28)]

CARDS = [
    lambda: draw("mountain_1", 711, LONE),
    lambda: draw("mountain_2", 712, TWIN),
    lambda: draw("mountain_3", 713, MASSIF),
    lambda: draw("mountain_4", 714, HORN, snowline=0.42),
    lambda: draw("mountain_5", 715, RANGE, snowline=0.56),
    lambda: draw("mountain_dry_1", 721, TWIN, rock=SAND, snow=None),
    lambda: draw("mountain_dry_2", 722, MASSIF, rock=SAND, snow=None),
    lambda: draw("mountain_dry_3", 723, LONE, rock=SAND, snow=None),
    lambda: draw("mountain_ash_1", 731, HORN, rock=ASH, snow=None, cracks=5),
    lambda: draw("mountain_ash_2", 732, MASSIF, rock=ASH, snow=None, cracks=6),
    lambda: draw("mountain_ash_3", 733, TWIN, rock=ASH, snow=None, cracks=5),
]


def sheet(out, k=3):
    """A contact sheet of this file's cards, `k` times enlarged, on the meadow's green."""
    names = sorted(p.stem for p in ART.glob("mountain_*.png"))
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
