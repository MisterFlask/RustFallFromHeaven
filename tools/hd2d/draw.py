"""Small prop cards of the HD-2D mode drawn by code (docs/HD2D.md §5b): the clutter that fills the
look target's tiles between the image model's trees and rocks -- grass tufts, reeds, pebbles, the
stones that rim its shores and river banks, standing stones, fences, haystacks -- and, until the
image model draws better ones (tools/hd2d/gen.py), its small designed things: huts, a well, ruins,
gardens, and each region's own (corrupted obelisks and altars, hell's bones and vents, elven
lanterns). Each is shaded as
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
DAUB = ["#4a3e30", "#6a5a44", "#8a785c", "#a69476", "#c0ae8e"]
SHINGLE = ["#2e1e1a", "#462a22", "#5e382c", "#764838", "#8c5a44"]
SLATE = ["#141218", "#211d28", "#2e2938", "#3d3649", "#4e465c"]
BONE = ["#5a5446", "#80786a", "#a89f8c", "#cbc2ac", "#e6dfca"]
SILVER = ["#3c4248", "#5e666e", "#868e96", "#b0b8be", "#d8dee2"]
EMBER = "#ff8a2a"
RUNE = "#c060ff"
LAMP = "#7fd8ff"
WINDOW = "#ffc860"

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


def thatch_roof(cv, x0, x1, y_eave, h, ramp, rnd, z=2.0, ridge_in=3):
    """A pitched roof's front slope seen from above the eaves: a trapezoid from the eave line up to a
    shorter ridge, lighter on its left (the sun's) half, streaked along the slope."""
    for k in range(h):
        y = y_eave - k
        t = k / max(1, h - 1)
        a = x0 + ridge_in * t
        b = x1 - ridge_in * t
        for x in range(int(a), int(b) + 1):
            u = (x - a) / max(1.0, b - a)
            lit = 0.78 - 0.5 * u + 0.12 * t + (0.1 if (x * 7 + k * 3) % 5 == 0 else 0) + (rnd.random() - 0.5) * 0.12
            col = ramp[int(np.clip(lit, 0, 0.999) * len(ramp))]
            cv.px(x, y, col, z)
    # The eave's shadow line and the ridge.
    for x in range(int(x0), int(x1) + 1):
        cv.px(x, y_eave + 1, ramp[0], z - 0.5)
    for x in range(int(x0 + ridge_in), int(x1 - ridge_in) + 1):
        cv.px(x, y_eave - h + 1, ramp[min(len(ramp) - 1, 3)], z + 0.1)


def wall_box(cv, x0, x1, y_top, y_bot, depth, ramp, rnd, z=1.0):
    """A house's front wall (facing the camera) and its right side (in shade), as a 3/4 view."""
    for y in range(y_top, y_bot):
        for x in range(x0, x1):
            lit = 0.62 + (0.18 if x == x0 else 0) + (rnd.random() - 0.5) * 0.1
            cv.px(x, y, ramp[int(np.clip(lit, 0, 0.999) * len(ramp))], z)
    for k in range(depth):
        for y in range(y_top - k // 2, y_bot - k // 2):
            cv.px(x1 + k, y, ramp[1], z - 0.2)


def hut(name, seed, stone=False):
    rnd = np.random.default_rng(seed)
    cv = Canvas(40, 40)
    wall = STONE if stone else DAUB
    wall_box(cv, 10, 28, 24, 34, 4, wall, rnd)
    # The door and a window lit from within.
    for y in range(28, 34):
        for x in range(13, 16):
            cv.px(x, y, WOOD[0] if x > 13 else WOOD[1], 1.5)
    for y in range(27, 29):
        for x in range(21, 24):
            cv.px(x, y, WINDOW, 1.5)
    thatch_roof(cv, 8, 31, 24, 11, SHINGLE if stone else STRAW, rnd, ridge_in=4)
    if stone:
        for y in range(9, 17):
            for x in range(24, 27):
                cv.px(x, y, STONE[3 if x == 24 else 1], 3.0)
    # A little grass round its foot.
    for k in range(8):
        cv.blade(rnd.uniform(7, 33), 35, rnd.normal(0, 1.2), int(rnd.integers(2, 5)), GRASS, z=4.0)
    cv.save(name)


def well(name, seed):
    rnd = np.random.default_rng(seed)
    cv = Canvas(30, 34)
    cv.blob(15, 26, 9, 5, STONE, rnd, bumps=0.1)
    for y in range(24, 27):
        for x in range(10, 21):
            if ((x - 15) / 5.0) ** 2 + ((y - 25.2) / 1.6) ** 2 < 1:
                cv.px(x, y, "#1a2a34", 2.0)
    for x in (7, 22):
        cv.rect(x, 9, x + 2, 26, WOOD, z=2.5)
    thatch_roof(cv, 5, 25, 11, 5, SHINGLE, rnd, z=3.0, ridge_in=2)
    cv.save(name)


def ruin(name, seed):
    rnd = np.random.default_rng(seed)
    cv = Canvas(48, 34)
    x = 4
    while x < 42:
        w = int(rnd.integers(4, 8))
        h = int(rnd.integers(4, 22))
        for y in range(h):
            for i in range(w):
                if y == h - 1 and rnd.random() < 0.5:
                    continue
                course = ((y // 3) + (i + y // 3 * 2) // 4) % 2
                lit = 0.6 + (0.2 if i == 0 else 0) - (0.25 if i == w - 1 else 0) + course * 0.08 + (rnd.random() - 0.5) * 0.1
                col = STONE[int(np.clip(lit, 0, 0.999) * len(STONE))]
                if (y > h - 4 or y < 2) and rnd.random() < 0.45:
                    col = MOSS[int(np.clip(lit, 0, 0.999) * len(MOSS))]
                cv.px(x + i, 30 - y, col, 1.0)
        x += w + int(rnd.integers(0, 3))
    for k in range(5):
        cv.blob(rnd.uniform(6, 42), 31, rnd.uniform(1.5, 2.5) * 1.3, rnd.uniform(1.5, 2.5), STONE, rnd, z=2.0)
    for k in range(10):
        cv.blade(rnd.uniform(4, 44), 32, rnd.normal(0, 1.2), int(rnd.integers(2, 6)), GRASS, z=3.0)
    cv.save(name)


def obelisk(name, seed):
    rnd = np.random.default_rng(seed)
    cv = Canvas(22, 50)
    h, w0 = 38, 9
    for y in range(h):
        t = y / h
        w = max(2, int(round(w0 * (1 - 0.45 * t))))
        x0 = 11 - w // 2
        for i in range(w):
            lit = 0.75 - 0.6 * i / max(1, w - 1) + (rnd.random() - 0.5) * 0.1
            cv.px(x0 + i, 45 - y, SLATE[int(np.clip(lit, 0, 0.999) * len(SLATE))], 1.0)
    for k in range(3):
        cv.px(11, 45 - h - k, SLATE[3 - k], 1.0)
    # Runes down its face, glowing.
    for y in range(12, 42, 4):
        for dx in (0, 1) if rnd.random() < 0.6 else (0,):
            cv.px(10 + dx, 45 - y, RUNE, 2.0)
    cv.blob(11, 46, 7, 2.5, SLATE, rnd, z=0.5)
    cv.save(name)


def altar(name, seed):
    rnd = np.random.default_rng(seed)
    cv = Canvas(34, 26)
    wall_box(cv, 6, 26, 14, 21, 3, SLATE, rnd)
    for y in range(12, 14):
        for x in range(6, 28):
            cv.px(x, y, SLATE[3], 2.0)
    for x in (9, 16, 23):
        for y in range(8, 12):
            cv.px(x, y, BONE[3], 2.5)
        cv.px(x, 7, RUNE, 3.0)
        cv.px(x, 6, "#e8b8ff", 3.0)
    for k in range(5):
        cv.blob(rnd.uniform(4, 30), 22, 2.2, 1.6, SLATE, rnd, z=0.5)
    cv.save(name)


def bones(name, seed):
    rnd = np.random.default_rng(seed)
    cv = Canvas(30, 18)
    for k in range(7):
        x, y = rnd.uniform(6, 24), rnd.uniform(11, 15)
        ang = rnd.uniform(0, np.pi)
        for t in np.linspace(-3, 3, 9):
            cv.px(x + np.cos(ang) * t, y + np.sin(ang) * t * 0.5, BONE[2 + int(t < 0)], 1.0 + k * 0.01)
    # A skull on top.
    cv.blob(15, 9, 3.6, 3.0, BONE, rnd, z=3.0, bumps=0.05)
    cv.px(14, 9, "#1a1410", 4.0)
    cv.px(16, 9, "#1a1410", 4.0)
    cv.px(15, 11, "#3a3026", 4.0)
    cv.save(name)


def vent(name, seed):
    rnd = np.random.default_rng(seed)
    cv = Canvas(34, 22)
    cv.blob(17, 15, 13, 6, SLATE, rnd, bumps=0.5)
    for y in range(10, 13):
        for x in range(12, 23):
            if ((x - 17) / 5.0) ** 2 + ((y - 11.5) / 1.5) ** 2 < 1:
                cv.px(x, y, EMBER if (x + y) % 3 else "#ffd070", 3.0)
    for k in range(4):
        x = rnd.uniform(6, 28)
        for j in range(3):
            cv.px(x + j * 0.7, 14 + j, EMBER, 2.5)
    cv.save(name)


def lantern(name, seed):
    rnd = np.random.default_rng(seed)
    cv = Canvas(14, 40)
    for y in range(8, 37):
        cv.px(6, y, SILVER[3], 1.0)
        cv.px(7, y, SILVER[1], 1.0)
    for x in range(4, 10):
        cv.px(x, 7, SILVER[4], 1.5)
    for y in range(3, 7):
        for x in range(5, 9):
            cv.px(x, y, LAMP if y > 3 else "#d8f6ff", 2.0)
    cv.px(6, 2, SILVER[4], 2.0)
    for x in range(3, 11):
        cv.px(x, 37, SILVER[0], 1.0)
    for k in range(5):
        cv.blade(rnd.uniform(2, 12), 38, rnd.normal(0, 1), int(rnd.integers(2, 5)), GRASS, z=3.0)
    cv.save(name)


def garden(name, seed):
    """A small vegetable patch: rows of green between furrows, as round a village."""
    rnd = np.random.default_rng(seed)
    cv = Canvas(40, 18)
    for r in range(4):
        y = 6 + r * 3
        for x in range(4 + r, 36 - r):
            cv.px(x, y + 1, DRY[1], 0.5)
            if rnd.random() < 0.8:
                cv.px(x, y, GRASS[int(rnd.integers(2, 5))], 1.0)
                if rnd.random() < 0.4:
                    cv.px(x, y - 1, GRASS[int(rnd.integers(3, 6))], 1.2)
    cv.save(name)


def falls(name, seed, w=30, h=26):
    """A waterfall's face, seen from below its lip: ribbons of white water over the darker teal of
    the falling sheet, between ragged wet rock, a lip of foam on top and a churned pool at the
    foot."""
    rnd = np.random.default_rng(seed)
    cv = Canvas(w + 8, h + 10)
    SHEET = ["#26485a", "#335e72", "#457a8e", "#5c94a8"]
    FOAM = ["#9cc4d2", "#c4e0e8", "#e6f4f8", "#fbfeff"]
    left = [4 + int(abs(rnd.normal(0, 1.2))) for _ in range(h + 2)]
    right = [w + 3 - int(abs(rnd.normal(0, 1.2))) for _ in range(h + 2)]
    ribbons = sorted(rnd.uniform(6, w + 2, int(w / 4)))
    for y in range(4, h + 4):
        t = (y - 4) / h
        for x in range(4, w + 4):
            if x < left[y - 4] or x > right[y - 4]:
                cv.px(x, y, WET[int(rnd.integers(0, 3))], 1.0)
                continue
            d = min(abs(x - r - np.sin(y * 0.4 + r) * 0.6) for r in ribbons)
            if d < 0.9 + 0.8 * t:
                col = FOAM[int(np.clip(0.95 - d * 0.35 + (rnd.random() - 0.5) * 0.3, 0, 0.999) * len(FOAM))]
            else:
                col = SHEET[int(np.clip(0.35 + 0.4 * rnd.random() + 0.2 * (1 - t), 0, 0.999) * len(SHEET))]
            cv.px(x, y, col, 1.0)
    for x in range(left[0], right[0] + 1):
        cv.px(x, 3, FOAM[3] if rnd.random() < 0.75 else FOAM[2], 2.0)
    for k in range(int(w * 0.7)):
        cv.blob(rnd.uniform(4, w + 4), h + 5 + rnd.normal(0, 0.6), rnd.uniform(1.6, 3.0), rnd.uniform(1.0, 1.8), FOAM, rnd, z=3.0, bumps=0.2)
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
    hut("hut_1", 60)
    hut("hut_2", 61)
    hut("hut_stone_1", 62, stone=True)
    well("well_1", 63)
    ruin("ruin_1", 64)
    ruin("ruin_2", 65)
    obelisk("obelisk_1", 66)
    altar("altar_1", 67)
    bones("bones_1", 68)
    bones("bones_2", 69)
    vent("vent_1", 70)
    vent("vent_2", 71)
    lantern("lantern_1", 72)
    garden("garden_1", 74)
    garden("garden_2", 75)
    falls("falls_1", 80)
    falls("falls_2", 81, w=36, h=22)
    subprocess.run([sys.executable, str(ROOT / "tools" / "hd2d" / "art.py"), "index"], check=True)


if __name__ == "__main__":
    main()
