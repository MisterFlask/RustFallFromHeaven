"""Small prop cards of the HD-2D mode drawn by code (docs/HD2D.md §5b): the clutter that fills the
look target's tiles between the image model's trees and rocks -- grass tufts, reeds, pebbles, the
stones that rim its shores and river banks, standing stones, fences, haystacks. Each is shaded as
the other cards are lit (from the upper left), in a few muted tones a material, with the same dark
one-pixel outline (tools/hd2d/art.py), written to crates/ffh-play/assets/app/r3d/hd2d/art/NAME.png
with its foot at the bottom centre. Deterministic (seeded); std + numpy + Pillow.

  python3 tools/hd2d/draw.py            # every card, then `art.py index`
"""
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "crates" / "ffh-play" / "assets" / "app" / "r3d" / "hd2d" / "art"

# Tone ramps, dark to light (muted, as the look target's land: docs/HD2D.md §6).
STONE = ["#2b2c2e", "#3f4043", "#56575a", "#6f6f70", "#8a8986", "#a8a59e"]
WET = ["#1e2224", "#2c3234", "#3b4244", "#4e5656", "#646c6a", "#7f8682"]
MOSS = ["#2a3318", "#3a4620", "#4d5a2a", "#626e36", "#7a8444"]
GRASS = ["#26301a", "#36431f", "#4a5828", "#5f6c31", "#78843d", "#97a050"]
DRY = ["#4a3c22", "#66532c", "#806a36", "#9c8444", "#b89e58"]
WOOD = ["#2c1e14", "#433020", "#5a422a", "#735636", "#8c6c46"]
STRAW = ["#5c4824", "#7c6230", "#9a7c3c", "#b6964a", "#cfb062", "#e0c47a"]
REED = ["#3a3a1e", "#545426", "#6c6c30", "#86863c"]
CATTAIL = ["#3a2414", "#55341c", "#6e4626"]

L = np.array([-0.55, -0.6, 0.58])  # light from the upper left, toward the viewer
L = L / np.linalg.norm(L)


def rgb(h):
    return [int(h[i : i + 2], 16) for i in (1, 3, 5)]


class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.c = np.zeros((h, w, 3), np.uint8)
        self.a = np.zeros((h, w), bool)
        self.z = np.full((h, w), -1e9)

    def px(self, x, y, col, z=0.0):
        x, y = int(round(x)), int(round(y))
        if 0 <= x < self.w and 0 <= y < self.h and z >= self.z[y, x]:
            self.c[y, x] = rgb(col) if isinstance(col, str) else col
            self.a[y, x] = True
            self.z[y, x] = z

    def blob(self, cx, cy, rx, ry, ramp, rnd, z=0.0, moss=None, moss_amt=0.0, bumps=0.35):
        """A rounded lump (a stone, a haystack's dome): an ellipsoid with a bumpy surface, its
        normal shaded into the ramp's tones; moss on the faces turned up."""
        x0, x1 = int(cx - rx - 1), int(cx + rx + 2)
        y0, y1 = int(cy - ry - 1), int(cy + ry + 2)
        ph = rnd.random(4) * 6.28
        for y in range(y0, y1):
            for x in range(x0, x1):
                u, v = (x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry
                ang = np.arctan2(v, u)
                rr = 1.0 + bumps * 0.25 * (np.sin(ang * 3 + ph[0]) + 0.6 * np.sin(ang * 5 + ph[1]))
                d = (u * u + v * v) / (rr * rr)
                if d > 1.0:
                    continue
                nz = np.sqrt(max(0.0, 1.0 - d))
                n = np.array([u, v, nz * 1.1])
                n /= np.linalg.norm(n)
                lit = float(n @ L)
                t = np.clip(0.5 + 0.55 * lit + (rnd.random() - 0.5) * 0.18, 0, 0.999)
                col = ramp[int(t * len(ramp))]
                if moss and v < -0.15 and rnd.random() < moss_amt * (-v):
                    col = moss[int(np.clip(t + 0.1, 0, 0.999) * len(moss))]
                self.px(x, y, col, z + nz)

    def blade(self, x, y, dx, h, ramp, z=0.0):
        """A blade of grass or a reed from (x, y) up, leaning dx over its height: darker at the
        root, lighter toward the tip."""
        for k in range(h):
            t = k / max(1, h - 1)
            bx = x + dx * t * t
            col = ramp[min(len(ramp) - 1, int(t * len(ramp) * 0.999 + 0.3))]
            self.px(bx, y - k, col, z + t * 0.1)

    def rect(self, x0, y0, x1, y1, ramp, light_left=True, z=0.0):
        """A plank or a post: lit on its left (or top) edge, shadowed on its right."""
        for y in range(y0, y1):
            for x in range(x0, x1):
                t = 0.55 + (0.3 if x == x0 and light_left else 0) - (0.3 if x == x1 - 1 else 0) + (0.15 if y == y0 else 0)
                self.px(x, y, ramp[int(np.clip(t, 0, 0.999) * len(ramp))], z)

    def save(self, name):
        # Crop to the opaque box, a pixel of margin for the outline, the foot at the bottom.
        ys, xs = np.nonzero(self.a)
        y0, y1, x0, x1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
        c, a = self.c[y0:y1, x0:x1], self.a[y0:y1, x0:x1]
        H, W = a.shape
        out = np.zeros((H + 2, W + 2, 4), np.uint8)
        out[1:-1, 1:-1, :3] = c
        out[1:-1, 1:-1, 3] = np.where(a, 255, 0)
        # The cards' dark outline (as tools/hd2d/art.py): a deep version of the colour inside.
        op = out[..., 3] > 0
        ring = np.zeros_like(op)
        near = np.zeros(op.shape + (3,), np.float32)
        cnt = np.zeros(op.shape, np.float32)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            m = np.roll(np.roll(op, dy, 0), dx, 1)
            ring |= m
            near += np.roll(np.roll(out[..., :3].astype(np.float32), dy, 0), dx, 1) * m[..., None]
            cnt += m
        ring &= ~op
        dark = near / np.maximum(cnt, 1)[..., None] * 0.28 + 6
        out[ring, :3] = dark[ring].astype(np.uint8)
        out[ring, 3] = 255
        ART.mkdir(parents=True, exist_ok=True)
        Image.fromarray(out, "RGBA").save(ART / f"{name}.png", optimize=True)
        print(f"{name}: card {out.shape[1]}x{out.shape[0]}")


def grass(name, seed, ramp=GRASS, n=11, h=(9, 17), spread=12):
    rnd = np.random.default_rng(seed)
    cv = Canvas(40, 30)
    for k in range(n):
        x = 20 + rnd.normal(0, spread / 4)
        hh = int(rnd.integers(*h))
        cv.blade(x, 27, rnd.normal(0, 3.5) + (x - 20) * 0.35, hh, ramp, z=rnd.random())
    cv.save(name)


def reeds(name, seed):
    rnd = np.random.default_rng(seed)
    cv = Canvas(36, 40)
    for k in range(9):
        x = 18 + rnd.normal(0, 4)
        hh = int(rnd.integers(16, 30))
        dx = rnd.normal(0, 2.5)
        cv.blade(x, 37, dx, hh, REED, z=rnd.random())
        if rnd.random() < 0.45:
            tx = x + dx
            for j in range(4):
                for i in range(2):
                    cv.px(tx + i - 0.5, 37 - hh + 3 + j, CATTAIL[min(2, (1 if i == 0 else 0) + (j == 0))], 2.0)
    cv.save(name)


def pebbles(name, seed, n=5, ramp=STONE):
    rnd = np.random.default_rng(seed)
    cv = Canvas(40, 20)
    for k in range(n):
        r = rnd.uniform(1.6, 3.4)
        cv.blob(20 + rnd.normal(0, 6), 14 + rnd.normal(0, 1.6), r * 1.3, r, ramp, rnd, z=rnd.random() * 0.1)
    cv.save(name)


def shore(name, seed, n=6, w=44):
    """A row of wet, mossy stones along a waterline: a band much wider than tall."""
    rnd = np.random.default_rng(seed)
    cv = Canvas(w + 12, 22)
    x = 6.0
    while x < w + 4:
        r = rnd.uniform(2.6, 5.0)
        cv.blob(x + r, 15 + rnd.normal(0, 1.0), r * rnd.uniform(1.1, 1.5), r * rnd.uniform(0.7, 0.95), WET, rnd, z=rnd.random() * 0.1, moss=MOSS, moss_amt=0.9)
        x += r * rnd.uniform(1.3, 1.9)
    # A few tufts of grass between the stones' tops.
    for k in range(int(rnd.integers(1, 4))):
        bx = rnd.uniform(8, w)
        for j in range(4):
            cv.blade(bx + j - 1.5, 13, rnd.normal(0, 1.5), int(rnd.integers(4, 8)), GRASS, z=0.5)
    cv.save(name)


def standing(name, seed, n=3):
    rnd = np.random.default_rng(seed)
    cv = Canvas(48, 44)
    xs = np.linspace(12, 36, n) + rnd.normal(0, 1.5, n)
    for k, x in enumerate(xs):
        h = int(rnd.integers(18, 30))
        w = int(rnd.integers(6, 9))
        base = 40 - (k % 2) * 2
        for y in range(h):
            t = y / h
            ww = w * (1.0 - 0.25 * t) + (1 if y < 2 else 0) * -1
            for i in range(int(ww)):
                u = (i + 0.5) / ww
                lit = 0.75 - u * 0.6 + (0.15 if y > h - 3 else 0) + (rnd.random() - 0.5) * 0.15
                col = STONE[int(np.clip(lit, 0, 0.999) * len(STONE))]
                if t > 0.7 and rnd.random() < 0.35:
                    col = MOSS[int(np.clip(lit, 0, 0.999) * len(MOSS))]
                if t < 0.12 and rnd.random() < 0.4:
                    col = MOSS[int(np.clip(lit * 0.8, 0, 0.999) * len(MOSS))]
                cv.px(x - ww / 2 + i, base - y, col, float(k))
    for k in range(10):
        gx = rnd.uniform(8, 40)
        cv.blade(gx, 41, rnd.normal(0, 1.5), int(rnd.integers(3, 7)), GRASS, z=10)
    cv.save(name)


def fence(name, seed, posts=4):
    rnd = np.random.default_rng(seed)
    gap = 11
    cv = Canvas(gap * (posts - 1) + 10, 24)
    for k in range(posts):
        x = 4 + k * gap
        top = 6 + int(rnd.integers(0, 2))
        cv.rect(x, top, x + 3, 22, WOOD, z=1.0)
    for ry in (9, 15):
        for k in range(posts - 1):
            x0 = 4 + k * gap
            sag = int(rnd.integers(0, 2))
            cv.rect(x0 + 1, ry + sag, x0 + gap + 2, ry + sag + 2, WOOD, light_left=False, z=0.5)
    cv.save(name)


def hay(name, seed):
    rnd = np.random.default_rng(seed)
    cv = Canvas(34, 34)
    # A stack, not a loaf: a tall dome over a wider, darker skirt.
    cv.blob(17, 24, 12, 7, STRAW[:4], rnd, bumps=0.3)
    cv.blob(17, 15, 9, 12, STRAW, rnd, z=1.0, bumps=0.25)
    # Straw stripes and a few loose wisps.
    for k in range(40):
        x, y = rnd.uniform(7, 27), rnd.uniform(8, 27)
        if cv.a[int(y), int(x)]:
            cv.px(x, y, STRAW[int(rnd.integers(0, 2))], 9)
            cv.px(x + 1, y, STRAW[int(rnd.integers(0, 2))], 9)
    for k in range(6):
        cv.blade(rnd.uniform(8, 26), 31, rnd.normal(0, 2), int(rnd.integers(2, 5)), DRY, z=10)
    cv.save(name)


def main():
    grass("grass_1", 1)
    grass("grass_2", 2, n=14, h=(7, 13))
    grass("grass_3", 3, ramp=DRY, n=10)
    reeds("reeds_1", 4)
    reeds("reeds_2", 5)
    pebbles("pebbles_1", 6)
    pebbles("pebbles_2", 7, n=7)
    for k in range(4):
        shore(f"shore_{k + 1}", 20 + k, w=38 + 6 * k)
    standing("stones_1", 30)
    standing("stones_2", 31, n=4)
    fence("fence_1", 40)
    fence("fence_2", 41, posts=3)
    hay("hay_1", 50)
    hay("hay_2", 51)
    subprocess.run([sys.executable, str(ROOT / "tools" / "hd2d" / "art.py"), "index"], check=True)


if __name__ == "__main__":
    main()
