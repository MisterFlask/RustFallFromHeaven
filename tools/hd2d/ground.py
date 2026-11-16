"""The HD-2D map's ground, drawn by code (docs/HD2D.md §5g): one tile of pixel art a ground, in the
hand of the cards drawn by code (trees.py, rocks.py) -- hard tones of one ramp, lit from the upper
left, marks a few pixels across, no gradients.

A tile is 256 x 256 art pixels (a texel each) and wraps. The terrain shader (shaders.js
PIXEL_GROUND) lays it at the small cards' density, 124 art pixels a plot east-west and about 68
north-south (the view foreshortens the ground: an art pixel comes out square on the screen), so a
tile is drawn as the screen will show it, like a card: a blade stands upright, north is up. Its
colours are the cards': the map shows them on open ground in full sun, in shade a tint of them.

A tile holds a ground's marks -- blades, stalks, ripples, pebbles, flecks of light and shade a few
pixels across -- on its base tone. The broad patches of tone, a plot or so across, are the
shader's (they do not repeat with the tile): it steps a pixel a tone darker or lighter by a noise
of the map, pushed by the tile's alpha. So alpha is not coverage: it is how a pixel leans in that
cut, about 0.5, and gives a patch's edge its shape -- upright teeth on grass (each column its own
lean), lying ones on straw, an even curve on sand.

  gnd_grass, gnd_straw, gnd_sand, gnd_dirt, gnd_lichen, gnd_snow, gnd_rock, gnd_marsh,
  gnd_burning, gnd_broken, gnd_perdition   the grounds
  gnd_soil, gnd_veg, gnd_grain             the crops of a field: tilled soil, beds, ripe grain
  gnd_pix                                  the utility tile: r, g lobes a few pixels across (two
                                           sets: the shader pushes a border between grounds by
                                           them), b a grain, a the cracks of a frozen sea

  python3 tools/hd2d/ground.py [NAME ...] [--sheet OUT.png]   # writes art/gnd_NAME.png (all by default)

Deterministic (seeded); numpy + Pillow. (The faces' strips are cliffs.py's.)
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "crates" / "ffh-play" / "assets" / "app" / "r3d" / "hd2d" / "art"
N = 256


def rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def ramp(*cs):
    return [rgb(c) for c in cs]


# --- fields (all wrap over the tile) -----------------------------------------------------------


def noise(cx, cy, seed):
    """Smooth value noise, cx by cy lattice cells over the tile, 0..1."""
    g = np.random.default_rng(seed).random((cy, cx))
    xs, ys = np.arange(N) * cx / N, np.arange(N) * cy / N
    x0, y0 = np.floor(xs).astype(int), np.floor(ys).astype(int)
    fx, fy = xs - x0, ys - y0
    ux, uy = (fx * fx * (3 - 2 * fx))[None, :], (fy * fy * (3 - 2 * fy))[:, None]
    x1, y1 = (x0 + 1) % cx, (y0 + 1) % cy
    return (g[np.ix_(y0, x0)] * (1 - ux) + g[np.ix_(y0, x1)] * ux) * (1 - uy) + (g[np.ix_(y1, x0)] * (1 - ux) + g[np.ix_(y1, x1)] * ux) * uy


def fbm(cx, cy, seed, octaves=3):
    s, w, t = 0.0, 1.0, 0.0
    for o in range(octaves):
        s = s + w * noise(cx << o, cy << o, seed + o * 17)
        t += w
        w *= 0.5
    return s / t


def rank(a):
    """A field ranked to a uniform 0..1: a threshold is then the share of the tile above it."""
    return (a.argsort(axis=None, kind="stable").argsort(kind="stable").reshape(a.shape) + 0.5) / a.size


def sh(a, dx, dy):
    """`a` moved dx to the right and dy down (wrapping)."""
    return np.roll(np.roll(a, dx, axis=1), dy, axis=0)


def clean(m, rounds=1):
    """A mask without specks or pinholes: each pixel takes the majority of its 3 x 3."""
    for _ in range(rounds):
        s = sum(sh(m.astype(int), dx, dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1))
        m = s >= 5
    return m


def blobs(cx, cy, seed, share, octaves=2, rounds=2):
    """Hard-edged blobs holding about `share` of the tile."""
    return clean(rank(fbm(cx, cy, seed, octaves)) > 1 - share, rounds)


def scatter(rng, count, gap):
    """Up to `count` points at least `gap` apart (wrapping), by dart throwing on a grid of cells
    that divide the tile (each at least `gap` wide: a point's rivals are in the cells round it)."""
    nc = max(1, int(N // max(1, gap)))
    cell = N / nc
    g = {}
    pts = []
    tries = 0
    while len(pts) < count and tries < count * 30:
        tries += 1
        x, y = int(rng.integers(0, N)), int(rng.integers(0, N))
        cx, cy = int(x / cell), int(y / cell)
        ok = True
        for j in (-1, 0, 1):
            for i in (-1, 0, 1):
                for px, py in g.get(((cx + i) % nc, (cy + j) % nc), ()):
                    if min(abs(x - px), N - abs(x - px)) ** 2 + min(abs(y - py), N - abs(y - py)) ** 2 < gap * gap:
                        ok = False
        if ok:
            g.setdefault((cx, cy), []).append((x, y))
            pts.append((x, y))
    return pts


def teeth(rng, amp=3, band=8, lying=False):
    """An offset for each pixel (0..amp), its own in every column over every `band` rows (lying:
    in every row over every `band` columns): a field looked up through it has its edges cut into
    upright teeth, like blades of grass (lying: like stalks combed flat)."""
    j = rng.integers(0, amp + 1, (N // band, N))
    j = np.repeat(j, band, axis=0)
    return j.T if lying else j


def through(field, jit, lying=False):
    """`field` looked up `jit` rows down (lying: columns across), wrapping."""
    yy, xx = np.mgrid[0:N, 0:N]
    return field[yy, (xx + jit) % N] if lying else field[(yy + jit) % N, xx]


def lean(rng, amp=0.3, band=8, lying=False):
    """The alpha of a ground whose patches of tone end in teeth: about 0.5, each column its own
    over every `band` rows (lying: each row over every `band` columns)."""
    j = np.repeat(rng.random((N // band, N)), band, axis=0)
    return 0.5 + ((j.T if lying else j) - 0.5) * 2 * amp


class Tile:
    """Tones (indices into a ramp, or -1 - k for accent k) and the alpha, a pixel each."""

    def __init__(self, base, h=0.5):
        self.t = np.full((N, N), base, int)
        self.h = np.full((N, N), 0.5, float) if np.isscalar(h) else h
        if np.isscalar(h):
            self.h[:] = h

    def put(self, mask, tone):
        self.t[mask] = tone

    def stamp(self, x, y, rows, tones):
        """A small drawing at (x, y) (its top left): `rows` of characters, '.' clear, else a key
        of `tones`."""
        for j, row in enumerate(rows):
            for i, ch in enumerate(row):
                if ch != ".":
                    self.t[(y + j) % N, (x + i) % N] = tones[ch]

    def dashes(self, rng, count, gap, where, tone, n=(2, 4), lying=False, tip=None, tips=1.0):
        """Short strokes (blades; lying: stalks along the wind, rising a pixel in three): from each
        of `count` scattered points inside `where` a run of n pixels in `tone`, the last in `tip`
        for a share `tips` of them."""
        for x, y in scatter(rng, count, gap):
            if not where[y, x]:
                continue
            k = int(rng.integers(n[0], n[1] + 1))
            tipped = tip is not None and rng.random() < tips
            for i in range(k):
                xx, yy = ((x + i) % N, (y - i // 3) % N) if lying else (x, (y - i) % N)
                self.t[yy, xx] = tip if tipped and i == k - 1 else tone

    def image(self, pal, accents=()):
        out = np.zeros((N, N, 4), np.uint8)
        for k, c in enumerate(pal):
            out[self.t == k, :3] = c
        for k, c in enumerate(accents):
            out[self.t == -1 - k, :3] = c
        out[..., 3] = np.clip(self.h * 255, 0, 255).astype(np.uint8)
        return out


# --- the grounds ---------------------------------------------------------------------------------

# Ramps run dark to light, in the cards' colour space: the map's finish (its brightness and
# saturation, config.js HD2D) lifts what is authored here -- the grass's base #719546 is shown as
# #89b050 on open ground in full sun.
GRASS = ramp("#375d2a", "#517a37", "#719546", "#90ad59", "#b0c674")
STRAW = ramp("#94763a", "#b7974e", "#ceb263", "#dbc880", "#e5daa4")
SAND = ramp("#b49165", "#ceaf80", "#e0cb9e", "#ead8b4", "#eee4cf")
DIRT = ramp("#64472b", "#84643c", "#a08152", "#b89b6b", "#cbb387")
TUNDRA = ramp("#585d49", "#72745c", "#8a8970", "#a09e87", "#b8b6a2")
SNOW = ramp("#8599bc", "#a7b9d5", "#c5d2e6", "#dbe4f3", "#f4f6fb")
ROCK = ramp("#22222a", "#3a3c42", "#55575a", "#747472", "#96948a", "#b6b2a4")
MARSH = ramp("#243220", "#38492a", "#4e6036", "#677a40", "#84924e")
POOL = ramp("#12262e", "#1a3840", "#2e5656", "#628a84")
BURNING = ramp("#40200f", "#623419", "#885026", "#aa6a34", "#c88846")
BROKEN = ramp("#19171d", "#2c2a33", "#42404b", "#5a5863", "#74727c", "#8e8c95")
PERDITION = ramp("#30101a", "#4c1a26", "#6a2634", "#883644", "#a44a56")
FLOWERS = ramp("#e5e7df", "#ddc350", "#9f87c8", "#f97182")
EMBER = ramp("#ff7a1a", "#ffc44a")


def grass(seed=11):
    rng = np.random.default_rng(seed)
    t = Tile(2, lean(rng))
    # Flecks of sunlit and of shaded turf, a few pixels across, their edges cut into blades.
    f = rank(through(fbm(24, 36, seed, 2), teeth(rng, 2)))
    shade, sun = clean(f < 0.1, 1), clean(f > 0.89, 1)
    t.put(shade, 1)
    t.put(sun, 3)
    # Blades: upright dashes, light (a few with a bright tip) and dark.
    t.dashes(rng, 2000, 3, ~shade, 3, tip=4, tips=0.3)
    t.dashes(rng, 1500, 3, ~sun, 1, n=(2, 3))
    # Tufts: a fan of blades over a dark root.
    for x, y in scatter(rng, 110, 13):
        t.stamp(x, y, ["l...l", ".l.l.", ".dld.", "..d.."] if rng.random() < 0.6 else [".l.l", "l.l.", ".dl.", ".d.."], {"l": 4, "d": 0})
    # Flowers, in loose drifts: a bright head on a dark foot.
    drift = rank(fbm(5, 5, seed + 9)) > 0.55
    for x, y in scatter(rng, 110, 8):
        if drift[y, x]:
            k = int(rng.choice(3, p=[0.5, 0.35, 0.15]))
            t.stamp(x, y, ["ff", "s."] if rng.random() < 0.5 else ["f", "s"], {"f": -1 - k, "s": 0})
    return t.image(GRASS, FLOWERS)


def straw(seed=23):
    rng = np.random.default_rng(seed)
    t = Tile(2, lean(rng, lying=True))
    # Dry grass combed by the wind: flecks of shade and of pale straw drawn out along it.
    yy, xx = np.mgrid[0:N, 0:N]
    f = rank(through(fbm(10, 40, seed, 2)[(yy + xx // 4) % N, xx], teeth(rng, 4, lying=True), lying=True))
    shade, sun = clean(f < 0.1, 1), clean(f > 0.88, 1)
    t.put(shade, 1)
    t.put(sun, 3)
    # Stalks: dashes along the wind, pale and dark.
    t.dashes(rng, 1500, 4, ~shade, 3, n=(3, 6), lying=True, tip=4, tips=0.4)
    t.dashes(rng, 1100, 4, ~sun, 1, n=(3, 5), lying=True)
    # A dry tuft, a green one now and then.
    for x, y in scatter(rng, 70, 16):
        if rng.random() < 0.3:
            t.stamp(x, y, ["g.g", ".g.", ".d."], {"g": -1, "d": -2})
        else:
            t.stamp(x, y, ["l..l", ".ll.", ".d.."], {"l": 4, "d": 0})
    return t.image(STRAW, [GRASS[2], GRASS[0]])


def sand(seed=37):
    rng = np.random.default_rng(seed)
    t = Tile(2, 0.42 + 0.16 * fbm(20, 30, seed + 1, 2))
    # Wind ripples: short crests in loose rows across the tile, each a lit edge over a line of
    # shade, bowed a little; between them the sand lies flat.
    wob = fbm(4, 4, seed, 2) * 14 + fbm(11, 11, seed + 3, 1) * 5
    for x, y in scatter(rng, 300, 13):
        n = int(rng.integers(6, 15))
        y0 = int(y + wob[y, x]) % N
        for i in range(n):
            # (Bowed: the ends a pixel lower than the middle.)
            yy = (y0 + (1 if i < 2 or i >= n - 2 else 0)) % N
            xx = (x + i) % N
            t.t[yy, xx] = 1
            t.t[(yy - 1) % N, xx] = 3
    # Grains of coarser sand, two pixels each, and pebbles: a lit top on a dark foot.
    for x, y in scatter(rng, 260, 9):
        if t.t[y, x] == 2 and t.t[y, (x + 1) % N] == 2:
            t.stamp(x, y, ["cc"], {"c": 1 if rng.random() < 0.6 else 3})
    for x, y in scatter(rng, 40, 24):
        t.stamp(x, y, ["pp", "dd"] if rng.random() < 0.6 else ["p", "d"], {"p": 4, "d": 0})
    return t.image(SAND)


def dirt(seed=41):
    rng = np.random.default_rng(seed)
    t = Tile(2, 0.4 + 0.2 * fbm(30, 40, seed + 1, 2))
    # Packed earth: flecks of damp and of trodden pale earth, pebbles and grit, a few cracks.
    f = rank(fbm(22, 30, seed, 2))
    t.put(clean(f < 0.1, 1), 1)
    t.put(clean(f > 0.9, 1), 3)
    for x, y in scatter(rng, 200, 8):
        r = rng.random()
        t.stamp(x, y, ["pp", "dd"] if r < 0.3 else [".p.", "pqd", ".d."] if r < 0.4 else ["p", "d"], {"p": 4, "q": 3, "d": 0})
    t.dashes(rng, 420, 6, t.t == 2, 1, n=(2, 3), lying=True)
    for x, y in scatter(rng, 14, 36):
        dx = int(rng.choice((-1, 1)))
        for i in range(int(rng.integers(4, 9))):
            t.t[(y + i) % N, (x + dx * (i // 2)) % N] = 0
    return t.image(DIRT)


def lichen(seed=53):
    rng = np.random.default_rng(seed)
    t = Tile(2, lean(rng, 0.2))
    # Tundra: pale stony ground, cold; flecks of lighter and darker ground, low tufts of moss in
    # two close greens, frost in the hollows, pebbles, a rust dwarf shrub here and there.
    f = rank(fbm(26, 36, seed + 9, 2))
    t.put(clean(f > 0.9, 1), 3)
    t.put(clean(f < 0.1, 1), 1)
    for x, y in scatter(rng, 420, 6):
        r = rng.random()
        if r < 0.45:
            t.stamp(x, y, [".m.", "mnm", ".n."] if rng.random() < 0.5 else ["mm", "nm"], {"m": -2, "n": -1})
        elif r < 0.62:
            t.stamp(x, y, ["ff"] if rng.random() < 0.6 else ["f", "f"], {"f": -5})
        elif r < 0.85:
            t.stamp(x, y, ["pp", "dd"] if rng.random() < 0.5 else ["p", "d"], {"p": 4, "d": 0})
        elif r < 0.92:
            t.stamp(x, y, [".r.", "rsr", ".d."], {"r": -4, "s": -3, "d": 0})
    return t.image(TUNDRA, [rgb("#63704a"), rgb("#78855a"), rgb("#74402a"), rgb("#965a38"), rgb("#d3dbe6")])


def snow(seed=67):
    rng = np.random.default_rng(seed)
    t = Tile(3, lean(rng, 0.25, lying=True))
    # Wind-carved snow: a few long drifts lying along the wind, each a blue hollow under a lit
    # crest.
    drift = clean(rank(noise(6, 34, seed) + 0.5 * noise(12, 68, seed + 1)) > 0.8, 2)
    t.put(drift, 2)
    t.put(drift & ~sh(drift, 0, -1), 1)
    t.put(sh(drift, 0, 1) & ~drift, 4)
    # Sparkle, and a stone or a grass tip through the snow.
    for x, y in scatter(rng, 150, 11):
        if t.t[y, x] == 3:
            t.stamp(x, y, ["ss"] if rng.random() < 0.4 else ["s"], {"s": 4})
    for x, y in scatter(rng, 12, 44):
        t.stamp(x, y, ["s", "d"], {"s": -1, "d": 0})
    return t.image(SNOW, [rgb("#565c64")])


def stones(seed, size=(13, 9)):
    """Stones fitted together (each pixel the nearest of a jittered lattice of points), each two
    planar facets in the light from the upper left. Returns (stone ids, tones 1..5 with the
    joints 0)."""
    rng = np.random.default_rng(seed)
    sx, sy = size
    pts = []
    for j in range(0, N, sy):
        off = rng.integers(0, sx)
        for i in range(0, N, sx):
            pts.append(((i + off + rng.uniform(-0.3, 0.3) * sx) % N, (j + rng.uniform(-0.3, 0.3) * sy) % N, rng.uniform(0.8, 1.3), rng.uniform(0.8, 1.25)))
    yy, xx = np.mgrid[0:N, 0:N]
    best = np.full((N, N), 1e9)
    sid = np.zeros((N, N), int)
    for k, (px, py, wx, wy) in enumerate(pts):
        dx = np.abs(xx + 0.5 - px)
        dx = np.minimum(dx, N - dx)
        dy = np.abs(yy + 0.5 - py)
        dy = np.minimum(dy, N - dy)
        d = (dx / (sx * wx)) ** 2 + (dy / (sy * wy)) ** 2
        near = d < best
        best[near] = d[near]
        sid[near] = k
    light = np.array([-0.5, -0.62, 0.6])
    light /= np.linalg.norm(light)
    tone = np.zeros((N, N), int)
    for k, (px, py, wx, wy) in enumerate(pts):
        m = sid == k
        if not m.any():
            continue
        u = ((xx + 0.5 - px + N / 2) % N - N / 2) / (sx * wx)
        v = ((yy + 0.5 - py + N / 2) % N - N / 2) / (sy * wy)
        tilt = rng.uniform(-0.6, 0.6), rng.uniform(-0.6, 0.3)
        a = rng.uniform(0, np.pi)
        cut = (np.cos(a), np.sin(a), rng.uniform(-0.2, 0.2))
        planes = [tilt, (tilt[0] + rng.uniform(-0.8, 0.8), tilt[1] + rng.uniform(-0.7, 0.7))]
        which = ((u * cut[0] + v * cut[1]) > cut[2]).astype(int)
        for f, (tx, ty) in enumerate(planes):
            n = np.array([tx, ty, 1.0])
            n /= np.linalg.norm(n)
            s = float(n @ light)
            tone[m & (which == f)] = 5 if s > 0.84 else 4 if s > 0.7 else 3 if s > 0.5 else 2 if s > 0.3 else 1
    joint = (sh(sid, -1, 0) != sid) | (sh(sid, 0, -1) != sid)
    lit = ((sh(sid, 1, 0) != sid) | (sh(sid, 0, 1) != sid)) & ~joint
    tone = np.where(lit & (tone >= 2), np.minimum(tone + 1, 5), tone)
    tone = np.where(joint, 0, tone)
    return sid, tone


def rock(seed=71):
    rng = np.random.default_rng(seed)
    sid, tone = stones(seed)
    t = Tile(0)
    t.t = tone.copy()
    # Cracks down the lighter stones.
    ys, xs = np.nonzero(tone >= 3)
    for _ in range(120):
        i = rng.integers(len(ys))
        x, y = int(xs[i]), int(ys[i])
        for s in range(int(rng.integers(2, 4))):
            yy_, xx_ = (y + s) % N, (x + s // 2) % N
            if sid[yy_, xx_] == sid[y, x] and t.t[yy_, xx_] >= 3:
                t.t[yy_, xx_] -= 2
    return t.image(ROCK)


def marsh(seed=83):
    rng = np.random.default_rng(seed)
    t = Tile(2, lean(rng))
    # Wet sedge: flecks of light and shade as the grass's, and pools catching the sky, a bank's
    # shade along their near edge and the far edge lit.
    f = rank(through(fbm(24, 36, seed, 2), teeth(rng, 2)))
    shade, sun = clean(f < 0.12, 1), clean(f > 0.9, 1)
    t.put(shade, 1)
    t.put(sun, 3)
    pool = blobs(8, 13, seed + 5, 0.15, rounds=3)
    t.put(pool, -2)
    t.put(pool & ~sh(pool, 0, 1), -1)
    t.put(pool & ~sh(pool, 0, -1), -3)
    for x, y in scatter(rng, 40, 14):
        if pool[y, x] and pool[y, (x + 2) % N] and pool[(y + 1) % N, x] and pool[(y - 1) % N, x]:
            t.stamp(x, y, ["gg"], {"g": -4})
    # Reeds: upright stalks in straw about the pools; sedge blades elsewhere.
    near = (sh(pool, 3, 0) | sh(pool, -3, 0) | sh(pool, 0, 3) | sh(pool, 0, -2)) & ~pool
    ys, xs = np.nonzero(near)
    for k in rng.choice(len(ys), min(len(ys), 380), replace=False):
        x, y = int(xs[k]), int(ys[k])
        n = int(rng.integers(2, 5))
        for j in range(n):
            if not pool[(y - j) % N, x]:
                t.t[(y - j) % N, x] = -5 if j == n - 1 else 4
    t.dashes(rng, 1100, 4, ~pool & ~near, 3, tip=4, tips=0.3)
    t.dashes(rng, 900, 4, ~pool & ~near, 1, n=(2, 3))
    return t.image(MARSH, [POOL[0], POOL[1], POOL[2], POOL[3], rgb("#ae9850")])


def burning(seed=97):
    rng = np.random.default_rng(seed)
    t = Tile(2, 0.42 + 0.16 * fbm(20, 30, seed + 1, 2))
    # Burning sands: the desert charred -- its ripples in soot, flecks of ash and of char, embers
    # alight here and there.
    wob = fbm(4, 4, seed, 2) * 14 + fbm(11, 11, seed + 3, 1) * 5
    gap = rank(fbm(9, 5, seed + 7, 2))
    yy, xx = np.mgrid[0:N, 0:N]
    line = ((yy + wob) % 16 < 1) & (gap > 0.3)
    t.put(sh(line, 0, -1) & ~line, 3)
    t.put(line, 0)
    f = rank(fbm(22, 30, seed + 11, 2))
    t.put(clean(f < 0.12, 1) & (t.t == 2), 1)
    t.put(clean(f > 0.92, 1) & (t.t == 2), 3)
    for x, y in scatter(rng, 60, 16):
        t.stamp(x, y, ["e", "f"] if rng.random() < 0.5 else ["ef"], {"e": -1, "f": -2})
    return t.image(BURNING, EMBER)


def broken(seed=101):
    rng = np.random.default_rng(seed)
    sid, tone = stones(seed, size=(38, 24))
    t = Tile(0)
    # Broken lands: plates of basalt, the seams between them dark, a few alight.
    t.t = tone.copy()
    joint = tone == 0
    hot = joint & (rank(fbm(5, 5, seed + 3, 2)) > 0.82)
    t.put(hot, -1)
    t.put(hot & sh(hot, 1, 0) & sh(hot, -1, 0) & (rank(noise(40, 40, seed + 5)) > 0.6), -2)
    # Cracks and grit on the plates.
    for x, y in scatter(rng, 260, 9):
        if t.t[y, x] >= 2 and t.t[y, (x + 1) % N] >= 2:
            t.stamp(x, y, ["cc"] if rng.random() < 0.5 else ["c", "c"], {"c": max(1, int(t.t[y, x]) - 1)})
    return t.image(BROKEN, EMBER)


def perdition(seed=109):
    rng = np.random.default_rng(seed)
    t = Tile(2, lean(rng, 0.2))
    # Fields of perdition: clotted wine-red ground -- clots lit on their upper left -- dark
    # festering pools, pale bone here and there.
    body = blobs(20, 28, seed, 0.3)
    t.put(body & ~sh(body, 1, 1), 3)
    t.put(sh(body, 1, 1) & ~body, 1)
    pool = blobs(7, 10, seed + 5, 0.14, rounds=3) & ~body
    t.put(pool, 0)
    t.put(pool & ~sh(pool, 0, -1), -3)
    for x, y in scatter(rng, 30, 26):
        if not pool[y, x]:
            t.stamp(x, y, ["bb", ".d"] if rng.random() < 0.5 else ["b", "d"], {"b": -1, "d": 0})
    return t.image(PERDITION, [rgb("#c0b498"), rgb("#f0f0f0"), rgb("#64162a")])


SOIL = ramp("#3a2914", "#4f391f", "#664e2c", "#80633c")
VEG = ramp("#2c5222", "#447431", "#659845", "#89b65c")
GRAIN = ramp("#7e6020", "#a8842c", "#cca63c", "#e6c456", "#f4dc84")


def soil(seed=131):
    rng = np.random.default_rng(seed)
    t = Tile(2)
    # Tilled soil: furrows running east-west, four pixels apart -- a line of shade, the ridge lit
    # above it -- broken into clods.
    yy, xx = np.mgrid[0:N, 0:N]
    t.put(yy % 4 == 0, 0)
    t.put(yy % 4 == 3, 1)
    t.put(yy % 4 == 1, 3)
    for x, y in scatter(rng, 900, 4):
        if y % 4 in (1, 2):
            t.stamp(x, y, ["cc"], {"c": 1 if rng.random() < 0.5 else 2})
        elif y % 4 == 0 and rng.random() < 0.5:
            t.stamp(x, y, ["c"], {"c": 1})
    return t.image(SOIL)


def veg(seed=137):
    rng = np.random.default_rng(seed)
    t = Tile(1)
    # Vegetable beds: rows of leafy heads on dark tilled earth, the rows eight pixels apart, a
    # head every five or six along a row; a furrow of shade between two rows.
    yy, xx = np.mgrid[0:N, 0:N]
    t.put(yy % 8 == 7, 0)
    t.put(yy % 8 == 0, 2)
    for row in range(0, N, 8):
        x = int(rng.integers(0, 4))
        while x < N - 3:
            k = rng.random()
            t.stamp(x, row + 2, [".l.", "lmm", "mmd", ".d."] if k < 0.6 else ["ll", "md", "d."], {"l": -3, "m": -2, "d": -1})
            x += int(rng.integers(5, 7))
    return t.image(SOIL, [VEG[0], VEG[2], VEG[3]])


def grain(seed=139):
    rng = np.random.default_rng(seed)
    t = Tile(2)
    # Ripe grain: rows of stalks running north-south, eight pixels apart -- a row's lit side, its
    # body, the shade between two rows -- crowned with pale ears.
    yy, xx = np.mgrid[0:N, 0:N]
    t.put(xx % 8 == 7, 0)
    t.put(xx % 8 == 6, 1)
    t.put(xx % 8 == 0, 3)
    for x, y in scatter(rng, 1300, 4):
        if x % 8 < 6:
            t.stamp(x, y, ["e", "e", "s"] if rng.random() < 0.6 else ["e", "s"], {"e": 4, "s": 1})
    return t.image(GRAIN)


def pix(seed=127):
    rng = np.random.default_rng(seed)
    out = np.zeros((N, N, 4), np.uint8)
    # Lobes: bumps about eight art pixels across with a finer wobble, two sets.
    for c in (0, 1):
        f = 0.65 * noise(32, 32, seed + c * 31) + 0.35 * noise(64, 64, seed + 7 + c * 31)
        out[..., c] = np.clip(rank(f) * 255, 0, 255)
    out[..., 2] = np.repeat(rng.integers(0, 256, (N, N // 2), dtype=np.uint8), 2, axis=1)
    # Cracks: the edges of cells about forty pixels across, two pixels wide.
    pts = [(rng.uniform(0, N), rng.uniform(0, N)) for _ in range(40)]
    yy, xx = np.mgrid[0:N, 0:N]
    best = np.full((N, N), 1e9)
    cid = np.zeros((N, N), int)
    for k, (px, py) in enumerate(pts):
        dx = np.abs(xx - px)
        dx = np.minimum(dx, N - dx)
        dy = np.abs(yy - py)
        dy = np.minimum(dy, N - dy)
        d = dx * dx + (dy * 1.6) ** 2
        near = d < best
        best[near] = d[near]
        cid[near] = k
    crack = (sh(cid, -1, 0) != cid) | (sh(cid, 0, -1) != cid)
    crack = crack | sh(crack, 1, 0) | sh(crack, 0, 1)
    out[..., 3] = np.where(crack, 255, 0)
    return out


TILES = {
    "grass": grass, "straw": straw, "sand": sand, "dirt": dirt, "lichen": lichen, "snow": snow, "rock": rock,
    "marsh": marsh, "burning": burning, "broken": broken, "perdition": perdition, "soil": soil, "veg": veg, "grain": grain,
    "pix": pix,
}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    sheet = None
    if "--sheet" in sys.argv:
        sheet = sys.argv[sys.argv.index("--sheet") + 1]
        args.remove(sheet)
    names = args or list(TILES)
    ART.mkdir(parents=True, exist_ok=True)
    made = {}
    for name in names:
        img = TILES[name]()
        made[name] = img
        Image.fromarray(img, "RGBA").save(ART / f"gnd_{name}.png", optimize=True)
        print(f"gnd_{name}.png", len(set(map(tuple, img[..., :3].reshape(-1, 3).tolist()))), "colours")
    if sheet:
        # For looking at: a 128 x 96 corner of each tile at 3x, four across.
        cells = [np.repeat(np.repeat(img[:96, :128, :3], 3, 0), 3, 1) for img in made.values()]
        cols = min(len(cells), 4)
        rows = (len(cells) + cols - 1) // cols
        out = np.zeros((rows * 288, cols * 384, 3), np.uint8)
        for k, c in enumerate(cells):
            out[(k // cols) * 288:(k // cols) * 288 + 288, (k % cols) * 384:(k % cols) * 384 + 384] = c
        Image.fromarray(out, "RGB").save(sheet)


if __name__ == "__main__":
    main()
