"""The stone cards of the HD-2D mode, drawn by code (docs/HD2D.md §5b): the boulders of hills,
peaks and banks (`rock_*`), the stones that rim shores and river banks (`shore_*`) and pebbles
(`pebbles_*`); and the haystacks of farms and pastures (`hay_*`), which are the trees' own lumps
in straw.

The recipe is the woods' trees' (trees.py), cut for stone: a card is a few separate stones, each
an angular silhouette (a polygon, flat at its foot) broken into planar facets; a facet takes one
of five hard tones of one ramp from the way it faces the light at the upper left (no gradients,
no noise), so a stone reads as a lit top, a middle face and a shaded side. A lit facet's edge
against a darker one carries a one-pixel highlight, a nearer stone throws a crease of shade on
the one behind, a few cracks are two- and three-pixel dashes, and a one-pixel outline lies inside
the silhouette, dark on the shaded side and softer on the lit rim. Shore stones wear a cap of
moss that follows their top edge, and a dark wet foot.

The cards carry their own outline (no second one is added). Deterministic (seeded);
std + numpy + Pillow.

  python3 tools/hd2d/rocks.py [--sheet OUT.png]   # every stone card, then `art.py index`
"""
import math
import random
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image

from trees import blob_tones

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "crates" / "ffh-play" / "assets" / "app" / "r3d" / "hd2d" / "art"


def rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


# Ramps: 0 the outline, 1..5 dark to light, 6 an edge catching the sun.
GREY = [rgb(c) for c in ("#15161c", "#2b2e38", "#454a55", "#666b72", "#8b8e8d", "#b3b2a8", "#d9d6c6")]
WARM = [rgb(c) for c in ("#1a1612", "#35302a", "#544c42", "#766c5c", "#9a8f78", "#bfb497", "#e0d8ba")]
WET = [rgb(c) for c in ("#10141a", "#222a33", "#38434c", "#536067", "#738080", "#98a39c", "#c0c8ba")]
MOSS = [rgb(c) for c in ("#0f180a", "#1f3010", "#35501b", "#507226", "#739635", "#9cba4c", "#c2d86c")]

LIGHT = np.array([-0.55, -0.62, 0.56])
LIGHT = LIGHT / np.linalg.norm(LIGHT)

SIZE = 96  # the working canvas; a card is cropped from it
FOOT = 80  # the row the stones stand on


class Card:
    def __init__(self, seed):
        self.rng = random.Random(seed)
        self.tone = np.full((SIZE, SIZE), -1, int)
        self.moss = np.zeros((SIZE, SIZE), bool)
        self.ids = np.full((SIZE, SIZE), -1, int)
        self.zbuf = np.full((SIZE, SIZE), -1e9)
        self.lit = np.zeros((SIZE, SIZE))
        self.n = 0

    def stone(self, cx, rx, ry, z=0.0, moss=0.0, sink=0.22, facets=None):
        """One stone standing on the foot row at `cx`, `rx` wide and about `2 ry (1 - sink)` tall
        (its lowest part is under the ground); `moss` is how much of its top wears moss."""
        rng = self.rng
        cy = FOOT - ry * (1 - 2 * sink)
        yy, xx = np.mgrid[0:SIZE, 0:SIZE]
        u, v = (xx + 0.5 - cx) / rx, (yy + 0.5 - cy) / ry
        # The silhouette: a polygon, the intersection of half-planes round an ellipse.
        k = max(6, int(round(5 + (rx + ry) / 3)))
        inside = yy <= FOOT
        a0 = rng.uniform(0, math.tau)
        for i in range(k):
            a = a0 + math.tau * (i + rng.uniform(-0.3, 0.3)) / k
            inside &= (u * math.cos(a) + v * math.sin(a)) < rng.uniform(0.8, 1.0)
        # Facets: each pixel takes the normal of the nearest of a few points on the stone.
        nf = facets or min(9, max(4, int(round(3 + rx * ry / 14))))
        pts = [(0.0, -0.45)]
        for _ in range(400):
            if len(pts) >= nf:
                break
            p = (rng.uniform(-0.85, 0.85), rng.uniform(-0.8, 0.8))
            if p[0] ** 2 + p[1] ** 2 < 0.8 and all((p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2 > 0.12 for q in pts):
                pts.append(p)
        best = np.full((SIZE, SIZE), 1e9)
        face = np.zeros((SIZE, SIZE), int)
        shade = np.zeros((SIZE, SIZE))
        for j, (pu, pv) in enumerate(pts):
            d = (u - pu) ** 2 + ((v - pv) * 1.25) ** 2
            n = np.array([pu, pv * 1.15 - 0.12, math.sqrt(max(0.05, 1 - pu * pu - pv * pv))])
            n /= np.linalg.norm(n)
            near = d < best
            best[near] = d[near]
            face[near] = j
            shade[near] = float(n @ LIGHT) + rng.uniform(-0.05, 0.05)
        depth = z + np.sqrt(np.clip(1 - u * u - v * v, 0, None)) * min(rx, ry)
        upd = inside & (depth > self.zbuf)
        t = np.where(shade > 0.74, 5, np.where(shade > 0.5, 4, np.where(shade > 0.24, 3, np.where(shade > 0.0, 2, 1))))
        # A lit facet's edge against a darker one below or to its right catches the light.
        for dx, dy in ((-1, 0), (0, -1)):
            nt = np.roll(np.roll(t, dx, axis=1), dy, axis=0)
            nf_ = np.roll(np.roll(face, dx, axis=1), dy, axis=0)
            t = np.where((nf_ != face) & (nt < t) & (t >= 3), t + 1, t)
        # The wet, shaded foot.
        t = np.where(yy >= FOOT - 1, np.minimum(t, 2), t)
        self.tone[upd] = t[upd]
        self.ids[upd] = self.n
        self.zbuf[upd] = depth[upd]
        self.lit[upd] = shade[upd]
        self.moss[upd] = False
        if moss > 0:
            # A cap of moss that follows the top edge, ragged below.
            cols = np.nonzero(inside.any(axis=0))[0]
            depth_px = rng.uniform(1.5, 3.0) * moss + 1
            for x in cols:
                top = int(np.argmax(inside[:, x]))
                depth_px = min(max(depth_px + rng.uniform(-1.0, 1.0), 1.0), 2.0 + ry * 0.7 * moss)
                edge = abs((x + 0.5 - cx) / rx)
                for y in range(top, top + int(round(depth_px * (1.15 - edge)))):
                    if y < FOOT - 2 and upd[y, x]:
                        self.moss[y, x] = True
        self.n += 1

    def cracks(self, n):
        """A few dark dashes on the lit faces."""
        rng = self.rng
        ys, xs = np.nonzero((self.tone >= 3) & ~self.moss)
        for _ in range(n):
            if not len(ys):
                return
            i = rng.randrange(len(ys))
            x, y = int(xs[i]), int(ys[i])
            dx = rng.choice((-1, 1))
            for s in range(rng.choice((2, 3))):
                px, py = x + dx * s, y + s
                if self.tone[py, px] >= 3 and not self.moss[py, px] and self.ids[py, px] == self.ids[y, x]:
                    self.tone[py, px] = 2

    def save(self, name, ramp, moss=MOSS):
        tone, mask = self.tone, self.tone >= 0
        # The shade a nearer stone throws on the one behind it, on the side away from the light.
        crease = np.zeros_like(mask)
        for dx, dy in ((1, 0), (0, 1), (1, 1)):
            nid = np.roll(np.roll(self.ids, dx, axis=1), dy, axis=0)
            nz = np.roll(np.roll(self.zbuf, dx, axis=1), dy, axis=0)
            crease |= mask & (nid >= 0) & (nid != self.ids) & (nz > self.zbuf + 0.8)
        tone[crease] = 1
        # The outline, one pixel, inside the silhouette; softer on the lit rim.
        pad = np.pad(mask, 1)
        inner = pad[:-2, 1:-1] & pad[2:, 1:-1] & pad[1:-1, :-2] & pad[1:-1, 2:]
        edge = mask & ~inner
        tone[edge & (self.lit <= 0.45)] = 0
        tone[edge & (self.lit > 0.45)] = 1
        img = np.zeros((SIZE, SIZE, 4), np.uint8)
        for y, x in zip(*np.nonzero(mask)):
            t = min(int(tone[y, x]), 6)
            mossy = self.moss[y, x] and t >= 1
            img[y, x] = (*(moss if mossy else ramp)[t], 255)
        ys, xs = np.nonzero(mask)
        out = np.zeros((ys.max() - ys.min() + 3, xs.max() - xs.min() + 3, 4), np.uint8)
        out[1:-1, 1:-1] = img[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
        Image.fromarray(out, "RGBA").save(ART / f"{name}.png", optimize=True)
        print(f"{name}: card {out.shape[1]}x{out.shape[0]}")


def boulder(name, seed, ramp, wide=False, moss=0.0):
    """A boulder with one or two smaller stones at its foot, about 22 art pixels tall."""
    c = Card(seed)
    r = c.rng
    rx, ry = (15, 10.5) if wide else (12, 12.5)
    rx, ry = rx + r.uniform(-1, 1), ry + r.uniform(-1, 1)
    c.stone(48, rx, ry, z=0, moss=moss)
    side = r.choice((-1, 1))
    c.stone(48 + side * (rx + 1), r.uniform(5, 6.5), r.uniform(5, 6), z=4, moss=moss * 0.6)
    if r.random() < 0.7:
        c.stone(48 - side * (rx - 1), r.uniform(3.5, 5), r.uniform(3.5, 4.5), z=6)
    c.cracks(4)
    c.save(name, ramp)


def shore(name, seed, n, moss=0.8):
    """Two to four separate stones in a loose row, wet and mossy, the largest about 11 art
    pixels tall: what rims a shore or a river bank."""
    c = Card(seed)
    r = c.rng
    x = 30.0
    sizes = [r.uniform(5.5, 8.5)] + [r.uniform(2.6, 5.2) for _ in range(n - 1)]
    r.shuffle(sizes)
    for s in sizes:
        rx, ry = s * r.uniform(1.15, 1.5), s * r.uniform(0.8, 1.0)
        x += rx
        c.stone(x, rx, ry, z=r.uniform(0, 3), moss=moss if s > 3.2 else 0.0, sink=0.25)
        x += rx * r.uniform(0.55, 1.05)
    c.cracks(n)
    c.save(name, WET)


def pebbles(name, seed, n):
    """A scatter of small dry stones."""
    c = Card(seed)
    r = c.rng
    for k in range(n):
        s = r.uniform(1.8, 3.4)
        c.stone(48 + r.uniform(-11, 11), s * 1.35, s, z=k, sink=0.15 + r.uniform(0, 0.25), facets=3)
    c.save(name, GREY)


STRAW = [rgb(c) for c in ("#1c1408", "#4a3414", "#75561e", "#a07a2a", "#c8a03c", "#e6c458", "#f6e088")]
STACK = [  # x, y, z, r: a haystack, a point over a broad skirt
    (0, -23, 1, 3), (0, -19, 2, 5), (0, -14, 3, 7), (-4.5, -8, 1, 7.5), (4.5, -8, 0, 7.5), (0, -8, 4, 8.5),
    (-7, -5, 0, 6.5), (7, -5, -1, 6.5),
]


def hay(name, seed, scale=1.0, squat=1.0):
    """A haystack: the trees' cluster of lumps in straw, cut flat at the ground, its straw as
    short strokes running down the stack."""
    rng = random.Random(seed)
    spheres = [((x + rng.uniform(-0.6, 0.6)) * scale, y * scale * squat, z * scale, r * scale) for (x, y, z, r) in STACK]
    tone, x0, y0 = blob_tones(spheres, seed, bias=0.08, speck=0.0)
    tone = tone[: -y0 + 1]  # nothing under the ground
    h, w = tone.shape
    foot = tone[h - 1] >= 0
    for x in range(w):
        for y in range(1, h - 3):
            t = tone[y, x]
            if t < 2 or rng.random() > 0.085:
                continue
            n = rng.choice((2, 3))
            if all(tone[y + i, x] == t for i in range(n)):
                d = 1 if rng.random() < 0.6 else -1
                if t + d >= 2:
                    for i in range(n):
                        tone[y + i, x] = t + d
    tone[h - 2][foot] = np.minimum(tone[h - 2][foot], 2)
    tone[h - 1][foot] = 0
    out = np.zeros((h + 2, w + 2, 4), np.uint8)
    for y, x in zip(*np.nonzero(tone >= 0)):
        out[y + 1, x + 1] = (*STRAW[min(int(tone[y, x]), 6)], 255)
    xs = np.nonzero(out[..., 3].any(axis=0))[0]
    ys = np.nonzero(out[..., 3].any(axis=1))[0]
    out = out[max(0, ys.min() - 1):ys.max() + 2, max(0, xs.min() - 1):xs.max() + 2]
    Image.fromarray(out, "RGBA").save(ART / f"{name}.png", optimize=True)
    print(f"{name}: card {out.shape[1]}x{out.shape[0]}")


CARDS = [
    lambda: hay("hay_1", 641),
    lambda: hay("hay_2", 642, scale=0.85, squat=0.9),
    lambda: hay("hay_3", 643, scale=1.0, squat=0.8),
    lambda: boulder("rock_1", 611, GREY),
    lambda: boulder("rock_2", 612, GREY, wide=True),
    lambda: boulder("rock_3", 613, WARM),
    lambda: boulder("rock_4", 614, GREY, wide=True, moss=0.5),
    lambda: shore("shore_1", 621, 2),
    lambda: shore("shore_2", 622, 3),
    lambda: shore("shore_3", 623, 3, moss=0.4),
    lambda: shore("shore_4", 624, 4),
    lambda: shore("shore_5", 625, 2, moss=1.0),
    lambda: shore("shore_6", 626, 4, moss=0.6),
    lambda: pebbles("pebbles_1", 631, 3),
    lambda: pebbles("pebbles_2", 632, 5),
    lambda: pebbles("pebbles_3", 633, 4),
]


def sheet(out, k=6):
    """A contact sheet of this file's cards, `k` times enlarged, on the meadow's green."""
    names = sorted(p.stem for p in ART.glob("*.png") if p.stem.split("_")[0] in ("rock", "shore", "pebbles", "hay"))
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
