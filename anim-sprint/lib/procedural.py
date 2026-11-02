"""Procedural (code-only, no image model) battle animations for the units that are not characters: ships, siege
engines and vehicles, elementals and other amorphous bodies.

  python lib/procedural.py                               # every ship, siege and elemental unit in roster_classified.json
  python lib/procedural.py --units unit_frigate unit_cannon
  python lib/procedural.py --plans ship --review         # also work/procedural/ship_review.png

ANIMATING_UNITS.md's rules 2 and 4 (the image model draws the big actions; never move or rotate the whole sprite) are
about characters: a body with limbs that has to change pose. A hull, a cart or a cloud of flame has no pose to change,
and rigid or fluid motion of its own pixels is how it really moves, so here every frame is the unit's own sprite moved,
tilted, bent, cut or dithered away. The output contract is the recipe's (same folders, frames, manifests and sheets):

  final_<key>_nofx/<anim>/frame_NN.png, <anim>/manifest.json, sheet_<anim>.png, unit_poses.json
  final_<key>/  the same frames, with the recipe's white hit flash on flinch and death frame 0

One recipe per body plan; `facing` (+1 lower-right, -1 lower-left) turns every motion round:
  ship     idle: a 1 px bob and a ~1 degree roll about the keel.  strike: rears back (bow up), then lurches at the
           target (pitched 6 degrees onto its bow, pushed 6 px toward it) on frame 4, back to rest.  flinch: a jolt
           away and a roll away, rebounding.  death: lists away from the target and sinks through a fixed waterline
           (the tightest line under the hull at rest, sloping down toward the bow as the iso water plane does);
           everything below it is cut, with a Bayer-dithered foam edge in the sprite's own light colours, ending with
           about 10% of the ship (mast tops, a pennant or the prow) above a foam slick where it went down.
           Tilts are scaled down where they would push the sprite off the canvas (the Queen of the Line has 1 px of
           headroom).
  siege    idle: a tiny creak (rocks half a degree on its wheels, settles 1 px).  strike: a cannon recoils away from the
           target with the muzzle kicking up (extreme on frame 4); a ram, catapult or wagon rears back and lurches
           forward onto its front wheels.  flinch: a shake.  death: tilts, darkens, cracks, breaks along a jagged line
           into a base that sags into the ground and an upper part that topples away and lands behind it (a gun
           barrel instead slumps nose-down off its carriage): wreckage.
  elemental idle: a slow breathing swell (smooth mesh warp, pinned at the ground).  strike: a surge toward the target
           (leans and stretches toward it, extreme on frame 4).  flinch: a squash away from the target.  death: an
           ordered (Bayer) dither eats it from the top and the edges while it slumps and spreads, ending as a faint
           puddle (water, tar, earth, ice) or a few wisps (smoke, light, flame, spirits).
Pixel art throughout: warps snap displacements to whole pixels (rig.mls_warp's sharpen trick), translations are whole
pixels, cuts and fades are hard-alpha dithers. The airship flies, so it crashes like a machine (siege death) rather
than sinking.
"""
import argparse, json, math, sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).resolve().parent))
import animkit as ak  # noqa: E402
import beats  # noqa: E402

ROOT = ak.ROOT
W, H = ak.W, ak.H  # the canvas of the unit being animated (set_canvas): 160x128, or taller/wider for oversize units
ANCHOR = ak.ANCHOR
TALL = (192, 144)  # oversize units are animated on this and cropped to 160, 176 or 192 wide (anchor W/2, H-2)
SPRITE_DIRS = [ROOT.parent / "tools/m7/roster/units", ROOT / "m7/roster/units"]
ROSTER = ROOT / "roster_classified.json"
PLANS = ("ship", "siege_or_vehicle", "elemental_or_amorphous")
SHARPEN = 3.0
BAYER4 = (np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]], np.float64) + 0.5) / 16.0

# Per-unit choices the description decides (anything not listed takes the plan's default).
UNIT = {
    # the roster calls it "front"; the face sits right of the funnel's centre and the larger claw reaches right
    "unit_air_elemental": {"facing": "lower-right"},
    # siege strikes: a gun recoils; a ram, catapult or wagon lurches forward
    # a gun barrel slumps nose-down off its smashed carriage (the gunner slumps over it); other upper works
    # topple away from the target (the default picks whichever way lands lowest)
    "unit_cannon": {"strike": "recoil", "topple": "nose-down"},
    "unit_dwarven_cannon": {"strike": "recoil", "topple": "nose-down"},
    "unit_war_machine": {"strike": "recoil"},
    "unit_battering_ram": {"strike": "lurch"},
    "unit_catapult": {"strike": "lurch"},
    "unit_supplies": {"strike": "lurch"},
    # the airship flies: it crashes onto the tile like a machine instead of sinking
    "unit_airship": {"death": "collapse", "break_at": 0.42},
    # elementals end as a puddle or as wisps
    "unit_water_elemental": {"end": "puddle"},
    "unit_tar_demon": {"end": "puddle"},
    "unit_earth_elemental": {"end": "puddle"},
    "unit_ice_elemental": {"end": "puddle"},
    # flying
    "unit_hawk": {"recipe": "bird"},
    # the roster calls it "front" (it is symmetric); it strikes lower-right like most of the roster
    "unit_ophanim": {"recipe": "wheel", "facing": "lower-right"},
    # in the water
    "unit_leviathan": {"recipe": "serpent"},
    "unit_sea_serpent": {"recipe": "serpent"},
    "unit_kraken": {"recipe": "serpent", "canvas": "tall", "power": 1.6},
    # oversize beasts: legs = the share of the height that folds under them when they die
    "unit_abashi": {"recipe": "beast", "canvas": "tall", "legs": 0.28},
    "unit_acheron": {"recipe": "beast", "canvas": "tall", "legs": 0.28},
    "unit_drifa": {"recipe": "beast", "canvas": "tall", "legs": 0.28},
    "unit_eurabatres": {"recipe": "beast", "canvas": "tall", "legs": 0.28},
    "unit_beast_of_agares": {"recipe": "beast", "canvas": "tall", "legs": 0.34},
    "unit_elephant": {"recipe": "beast", "canvas": "tall", "legs": 0.36},
    "unit_margalard": {"recipe": "beast", "canvas": "tall", "legs": 0.32},
    "unit_giant_tortoise": {"recipe": "beast", "canvas": "tall", "legs": 0.3, "tortoise": True},
    # oversize bipeds: hip = share of the height where the upper body bends; arm = (shoulder, polygon) in sprite
    # pixels for a weapon arm that swings further than the body (a weapon carried across the shoulders just rides
    # with the upper body)
    "unit_auric_ascended": {"recipe": "giant", "canvas": "tall",
                            "arm": ((96, 60), [(86, 0), (115, 0), (115, 128), (86, 128)])},
    "unit_flagbearer": {"recipe": "giant", "canvas": "tall", "hip": 0.4,
                        "arm": ((76, 82), [(0, 0), (96, 0), (96, 136), (68, 136), (68, 86), (70, 66), (64, 56),
                                           (44, 52), (30, 58), (18, 76), (0, 76)])},
    "unit_gurid": {"recipe": "giant", "canvas": "tall", "hip": 0.4,
                   "arm": ((80, 50), [(78, 8), (111, 8), (111, 68), (80, 68), (74, 50)])},
    "unit_hill_giant": {"recipe": "giant", "canvas": "tall"},
    "unit_mithril_golem": {"recipe": "giant", "canvas": "tall"},
    "unit_wilboman": {"recipe": "giant", "canvas": "tall"},
    # the roster calls it "front"; the axe head is on the screen-left, so it chops lower-left
    "unit_wrath": {"recipe": "giant", "canvas": "tall", "facing": "lower-left"},
}


# ---------------------------------------------------------------- image helpers
def premul(im):
    a = np.asarray(im.convert("RGBA")).astype(np.float64)
    return np.dstack([a[..., :3] * a[..., 3:4] / 255.0, a[..., 3]])


def unpremul(pm):
    A = np.clip(pm[..., 3], 0, 255)
    rgb = np.clip(pm[..., :3] / np.maximum(A[..., None] / 255.0, 1e-4), 0, 255)
    out = np.dstack([rgb, A])
    out[A < 2] = 0
    return Image.fromarray(np.round(out).astype(np.uint8), "RGBA")


def set_canvas(w, h):
    """Every helper below works on the current canvas; the anchor is (W/2, H-2), as animkit's (80, 126) on 160x128."""
    global W, H, ANCHOR
    W, H, ANCHOR = int(w), int(h), (int(w) // 2, int(h) - 2)


def grid():
    ys, xs = np.mgrid[0:H, 0:W]
    return xs.astype(np.float64), ys.astype(np.float64)


def sample(pm, src_x, src_y, sharpen=SHARPEN, order=1):
    """Backward-map a premultiplied image: output pixel (x, y) takes the source at (src_x, src_y). Displacements are
    snapped toward whole pixels (rig.mls_warp's sharpen), so most pixels move by exact pixels and stay crisp and the
    sub-pixel blend is confined to narrow seams. Returns a premultiplied float array."""
    xs, ys = grid()
    sx, sy = np.array(src_x, np.float64), np.array(src_y, np.float64)
    if sharpen > 0:
        for s, g in ((sx, xs), (sy, ys)):
            d = s - g
            fl = np.floor(d + 1e-9)
            fr = np.clip((d - fl - 0.5) * (1.0 + sharpen) + 0.5, 0.0, 1.0)
            s[...] = g + fl + fr
    return np.dstack([ndimage.map_coordinates(pm[..., c], [sy, sx], order=order, mode="constant", cval=0.0)
                      for c in range(4)])


def rigid_src(lean=0.0, pivot=(80.0, 125.0), dx=0, dy=0, sx=1.0, sy=1.0):
    """Source coordinates for a rigid move: scale (sx, sy) and turn by `lean` degrees about `pivot`, then translate by
    whole pixels (dx, dy). lean > 0 turns the top toward +x (clockwise on screen)."""
    a = math.radians(lean)
    c, s = math.cos(a), math.sin(a)
    M = np.array([[c, -s], [s, c]]) @ np.diag([sx, sy])
    Mi = np.linalg.inv(M)
    xs, ys = grid()
    ox, oy = xs - pivot[0] - dx, ys - pivot[1] - dy
    return Mi[0, 0] * ox + Mi[0, 1] * oy + pivot[0], Mi[1, 0] * ox + Mi[1, 1] * oy + pivot[1]


def shift_pm(pm, dx, dy):
    """Whole-pixel translation, exact."""
    out = np.zeros_like(pm)
    h, w = pm.shape[:2]
    xs0, xs1 = max(0, dx), min(w, w + dx)
    ys0, ys1 = max(0, dy), min(h, h + dy)
    out[ys0:ys1, xs0:xs1] = pm[ys0 - dy:ys1 - dy, xs0 - dx:xs1 - dx]
    return out


def over(dst, src):
    """Premultiplied 'src over dst'."""
    a = src[..., 3:4] / 255.0
    return src + dst * (1.0 - a)


def bayer(shape=None):
    h, w = shape or (H, W)
    return np.tile(BAYER4, (h // 4 + 1, w // 4 + 1))[:h, :w]


def light_colours(pm, k=2):
    """The sprite's own light colours: the brightest opaque pixels, as k colours from the top few percent."""
    rgba = unpremul(pm)
    a = np.asarray(rgba).astype(np.float64)
    op = a[..., 3] >= 200
    px = a[op][:, :3]
    lum = px @ np.array([0.299, 0.587, 0.114])
    order = np.argsort(lum)
    cols = []
    for q in (0.995, 0.96)[:k]:
        i = order[min(len(order) - 1, int(q * len(order)))]
        lo = max(0, int((q - 0.02) * len(order)))
        hi = min(len(order), int((q + 0.004) * len(order)) + 1)
        cols.append(tuple(np.median(px[order[lo:hi]], axis=0)))
    return cols


def darken(pm, k, soot=(34, 26, 22)):
    """Scorch: desaturate and multiply toward a dark soot colour, alpha kept (premultiplied). k = 1 is unchanged."""
    out = pm.copy()
    a = pm[..., 3:4] / 255.0
    lum = (pm[..., :3] @ np.array([0.299, 0.587, 0.114]))[..., None]
    desat = (1.0 - k) * 1.2
    rgb = pm[..., :3] * (1 - desat) + lum * desat
    out[..., :3] = rgb * k + np.array(soot, np.float64) * a * (1 - k) * 0.5
    return out


# ---------------------------------------------------------------- the unit
def support(mask, side):
    """The point a rigid body rocks on when it tips toward `side` (+1 right, -1 left): its extreme opaque pixel in the
    direction (side, 2 down). Every other pixel is then higher than it by at least half its sideways distance, so a
    tip of up to about 30 degrees on this point never pushes anything below it (the ground stays the ground)."""
    ys, xs = np.nonzero(mask)
    i = int(np.argmax(side * xs + 2 * ys))
    return float(xs[i]), float(ys[i])


# ---------------------------------------------------------------- the unit
class Body:
    def __init__(self, name, facing, canvas=(ak.W, ak.H)):
        set_canvas(*canvas)
        self.name = name
        self.key = name[5:] if name.startswith("unit_") else name
        path = next((d / f"{name}.png" for d in SPRITE_DIRS if (d / f"{name}.png").exists()), None)
        if path is None:
            raise FileNotFoundError(name)
        sp = Image.open(path).convert("RGBA")
        if sp.width > W or sp.height > H:
            raise ValueError(f"{name} is {sp.width}x{sp.height}, larger than the {W}x{H} canvas")
        self.sprite_path = path
        self.offset = (int(ANCHOR[0] - sp.width / 2), H - sp.height)  # as recipe.load_unit: M7 feet 2 px above bottom
        base = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        base.alpha_composite(sp, self.offset)
        self.base = base
        self.pm = premul(base)
        self.facing = facing
        self.f = 1 if facing == "lower-right" else -1
        A = self.pm[..., 3]
        self.mask = A >= 128
        rows = np.nonzero(self.mask.any(1))[0]
        cols = np.nonzero(self.mask.any(0))[0]
        self.top, self.ground = int(rows.min()), int(rows.max())
        self.left, self.right = int(cols.min()), int(cols.max())
        self.cx = (self.left + self.right) / 2.0
        self.height = self.ground - self.top + 1

    def contact(self, side):
        """The point the body rocks on when it tips toward `side` (+1 right, -1 left)."""
        return support(self.mask, side)

    def rigid(self, lean=0.0, dx=0, dy=0, pivot=None, sx=1.0, sy=1.0, pm=None):
        """Tilt by `lean` degrees (> 0 toward +x) on the contact point of that side (or `pivot`), then move by whole
        pixels. Returns premultiplied float."""
        pm = self.pm if pm is None else pm
        if abs(lean) < 1e-6 and sx == 1.0 and sy == 1.0:
            return shift_pm(pm, int(dx), int(dy))
        if pivot is None:
            pivot = self.contact(1 if lean > 0 else -1)
        return sample(pm, *rigid_src(lean, pivot, int(dx), int(dy), sx, sy))

    def fit(self, lean, dx, dy, pivot, margin=0):
        """The largest tilt (scaled down from `lean` in 10% steps) and push that keep the whole body on the canvas:
        a tall sprite such as the Queen of the Line has no headroom, and a tilt on its keel would cut its masts."""
        ys, xs = np.nonzero(self.mask)
        for k in range(11):
            a = math.radians(lean * (1 - 0.1 * k))
            c, s = math.cos(a), math.sin(a)
            X = c * (xs - pivot[0]) - s * (ys - pivot[1]) + pivot[0] + dx
            Y = s * (xs - pivot[0]) + c * (ys - pivot[1]) + pivot[1] + dy
            if X.min() >= margin and X.max() <= W - 1 - margin and Y.min() >= 0 and Y.max() <= H - 1:
                return lean * (1 - 0.1 * k), dx
            if k == 10:
                break
        return 0.0, dx


def frame(pm):
    return unpremul(pm)


# ---------------------------------------------------------------- ships
def ship_waterline(b):
    """The water surface at rest, fixed on screen: the tightest straight line under the hull's bottom contour whose
    slope runs down toward the bow side (iso: the water meets the near side of the hull along the ship's long axis,
    nearer the viewer at the bow), at most 0.55; the whole hull sits on or above it. It never drops below the ground
    row, so the cut and its foam are always on the canvas. Returns wl[x] (row) for every canvas column."""
    m = b.mask
    xs, bots = [], []
    for x in range(b.left, b.right + 1):
        r = np.nonzero(m[:, x])[0]
        if len(r):
            xs.append(x)
            bots.append(r.max())
    xs, bots = np.array(xs, float), np.array(bots, float)
    best = None
    for s in np.linspace(0.0, 0.55, 56) * b.f:
        icpt = float(np.max(bots - s * xs))
        gap = float(np.sum(s * xs + icpt - bots))
        if best is None or gap < best[0] - 1e-9:
            best = (gap, s, icpt)
    _, slope, icpt = best
    return np.minimum(slope * np.arange(W) + icpt, float(b.ground))


FOAM_ROWS = ((0, 0.9), (-1, 0.5), (1, 0.4))  # (row offset from the waterline, Bayer density) where the hull crosses


def foam(pm, wl, crossing, cols, spread=2, slick=None, slick_density=0.0):
    """Dithered foam on the water surface: on the columns where the hull goes under (`crossing`, bool per column,
    widened by `spread`), the waterline row and one row either side get the sprite's light colours through a Bayer
    threshold; `slick` (bool per column) adds a sparser lens of churned water where the hull has gone under,
    thickest in its middle."""
    out = pm.copy()
    bm = bayer()
    cross = ndimage.binary_dilation(crossing, iterations=spread) if spread else crossing
    lens = np.zeros(W)
    if slick is not None and slick.any() and slick_density > 0:
        xs = np.nonzero(slick)[0]
        c, hw = (xs.min() + xs.max()) / 2.0, (xs.max() - xs.min()) / 2.0 + 1.0
        lens = np.clip(1.0 - ((np.arange(W) - c) / hw) ** 2, 0, 1) ** 0.5 * slick * slick_density
    c1, c2 = cols
    for x in range(W):
        for dy, dens in FOAM_ROWS:
            d = max(dens if cross[x] else 0.0, lens[x] * (1.0 if dy == 0 else 0.45))
            if d <= 0:
                continue
            y = int(math.floor(wl[x])) + dy
            if not (0 <= y < H):
                continue
            t = bm[y, x]
            if t < d:
                c = c1 if t < d * 0.5 else c2
                out[y, x] = [c[0], c[1], c[2], 255.0]
    return out


def cut_below(pm, wl):
    """Clear everything below the line wl[x] (exclusive), hard."""
    out = pm.copy()
    ys = np.arange(H)[:, None]
    out[ys > np.floor(wl)[None, :]] = 0
    return out


def ship_anims(b, opts):
    f = b.f
    piv_toward, piv_away = b.contact(f), b.contact(-f)
    P = {}
    # idle: roll about the keel's middle (~1 degree) and a 1 px bob down, a smooth loop (0, +, bob, -)
    keel = (b.cx, float(b.ground))
    idle = [dict(lean=0.0), dict(lean=0.9 * f, pivot=keel), dict(lean=0.0, dy=1), dict(lean=-0.9 * f, pivot=keel)]
    # strike: rear back (bow lifts), lurch at the target: pitched toward it on its bow, pushed along the diagonal
    strike = [dict(lean=0.0),
              dict(lean=-1.5 * f, dx=-1 * f),
              dict(lean=-3.0 * f, dx=-3 * f),
              dict(lean=3.0 * f, dx=3 * f),
              dict(lean=6.0 * f, dx=6 * f, dy=1),
              dict(lean=4.0 * f, dx=4 * f, dy=1),
              dict(lean=1.5 * f, dx=2 * f),
              dict(lean=-0.5 * f)]
    flinch = [dict(lean=-4.0 * f, dx=-4 * f),
              dict(lean=-5.0 * f, dx=-3 * f),
              dict(lean=1.2 * f, dx=0),
              dict(lean=-0.4 * f)]
    P["idle"], P["strike"], P["flinch"] = idle, strike, flinch

    def render(p):
        piv = p.get("pivot") or (piv_toward if p["lean"] * f > 0 else piv_away)
        p["lean"], p["dx"] = b.fit(p["lean"], p.get("dx", 0), p.get("dy", 0), piv)
        return b.rigid(p["lean"], p["dx"], p.get("dy", 0), pivot=piv)

    frames = {a: [b.pm if (a in ("idle", "strike") and i == 0) else render(p) for i, p in enumerate(ps)]
              for a, ps in P.items()}
    if opts.get("death") == "collapse":
        frames["death"], P["death"] = collapse_death(b, frames["flinch"][0], P["flinch"][0], opts)
    else:
        frames["death"], P["death"] = ship_death(b, frames["flinch"][0], P["flinch"][0], piv_away)
    return frames, P


def ship_death(b, flinch0, fl0, pivot, target=0.10, drift0=4):
    """List away from the target (on the same rocking point as the flinch, so frame 0 -> 1 does not jump) and sink
    through the fixed waterline. The sink depth is chosen per ship so that about `target` of its pixels still show on
    the held frame (mast tops or the prow)."""
    f = b.f
    wl = ship_waterline(b)
    cols = light_colours(b.pm)
    lean = [None, 5, 9, 13, 17, 20, 22, 23]
    sink = [None, 0.05, 0.13, 0.26, 0.45, 0.64, 0.82, 1.0]
    drift = [None] + [d - 4 + drift0 for d in (4, 4, 5, 5, 6, 6, 6)]  # continues the flinch's jolt
    area0 = (b.pm[..., 3] >= 128).sum()
    hull_cols = b.mask.any(0)

    g = 1.0  # the list, scaled down for a wide ship whose leaning masts would leave the canvas above the water

    def build(S, k):
        s = int(round(sink[k] * S))
        raw = b.rigid(-lean[k] * g * f, -drift[k] * f, s, pivot=pivot)
        ys = np.arange(H)[:, None]
        # the surface churns wherever some of the ship is under it
        crossing = ((raw[..., 3] >= 64) & (ys > np.floor(wl)[None, :])).any(0)
        out = cut_below(raw, wl)
        # once the hull is under, the water above it churns: a sparse foam slick over where the hull went down
        slick_d = [0, 0, 0, 0.2, 0.45, 0.5, 0.45, 0.38][k]
        # shift the original hull's columns with the drift so the slick stays over the wreck
        slick = np.roll(hull_cols, -drift[k] * f)
        return foam(out, wl, crossing, cols, spread=2, slick=slick, slick_density=slick_d), s

    def sink_depth():
        for s_try in range(16, 2 * H, 2):
            fr, _ = build(s_try, 7)
            if (fr[..., 3] >= 128).sum() <= target * area0:
                return s_try
        return 2 * H

    def visible_on_canvas(S):
        ys, xs = np.nonzero(b.mask)
        for k in range(1, 8):
            a = math.radians(-lean[k] * g * f)
            c, sn = math.cos(a), math.sin(a)
            X = c * (xs - pivot[0]) - sn * (ys - pivot[1]) + pivot[0] - drift[k] * f
            Y = sn * (xs - pivot[0]) + c * (ys - pivot[1]) + pivot[1] + round(sink[k] * S)
            vis = Y <= wl[np.clip(np.round(X).astype(int), 0, W - 1)]
            if vis.any() and (X[vis].min() < 0 or X[vis].max() > W - 1):
                return False
        return True

    for g in (1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4):
        S = sink_depth()
        if visible_on_canvas(S):
            break
    frames, params = [flinch0], [dict(fl0, note="= flinch frame 0")]
    for k in range(1, 8):
        fr, s = build(S, k)
        frames.append(fr)
        params.append(dict(lean=round(-lean[k] * g * f, 2), dx=-drift[k] * f, sink=s,
                           waterline=[round(float(wl[0]), 1), round(float(wl[-1]), 1)]))
    return frames, params


# ---------------------------------------------------------------- siege engines and vehicles
def siege_anims(b, opts):
    f = b.f
    P = {}
    idle = [dict(lean=0.0), dict(lean=0.5 * f), dict(lean=0.0, sy=1.0 - 1.0 / b.height), dict(lean=-0.4 * f)]
    if opts.get("strike", "lurch") == "recoil":
        # a gun: holds, fires on frame 3, recoil extreme on frame 4 (back along the ground diagonal, muzzle up),
        # then rolls forward to rest
        strike = [dict(lean=0.0),
                  dict(lean=0.4 * f),
                  dict(lean=0.8 * f, dx=1 * f),
                  dict(lean=-1.5 * f, dx=-2 * f, dy=-1),
                  dict(lean=-4.0 * f, dx=-5 * f, dy=-2),
                  dict(lean=-2.5 * f, dx=-4 * f, dy=-2),
                  dict(lean=-0.8 * f, dx=-2 * f, dy=-1),
                  dict(lean=0.3 * f)]
    else:
        # a ram, catapult or wagon: rears back, then lurches forward onto its front wheels
        strike = [dict(lean=0.0),
                  dict(lean=-1.5 * f, dx=-1 * f),
                  dict(lean=-3.0 * f, dx=-2 * f),
                  dict(lean=2.0 * f, dx=3 * f),
                  dict(lean=4.0 * f, dx=5 * f, dy=1),
                  dict(lean=2.0 * f, dx=3 * f, dy=1),
                  dict(lean=0.6 * f, dx=1 * f),
                  dict(lean=-0.3 * f)]
    flinch = [dict(lean=-2.5 * f, dx=-2 * f),
              dict(lean=1.2 * f, dx=1 * f),
              dict(lean=-0.6 * f, dx=-1 * f),
              dict(lean=0.2 * f)]
    P["idle"], P["strike"], P["flinch"] = idle, strike, flinch

    def render(p):
        piv = b.contact(1 if p["lean"] > 0 else -1) if p.get("sy", 1.0) == 1.0 else (b.cx, float(b.ground))
        p["lean"], p["dx"] = b.fit(p["lean"], p.get("dx", 0), p.get("dy", 0), piv)
        return b.rigid(p["lean"], p["dx"], p.get("dy", 0), sy=p.get("sy", 1.0), pivot=piv)

    frames = {a: [b.pm if (a in ("idle", "strike") and i == 0) else render(p) for i, p in enumerate(ps)]
              for a, ps in P.items()}
    frames["death"], P["death"] = collapse_death(b, frames["flinch"][0], P["flinch"][0], opts)
    return frames, P


def break_masks(b, at=0.5):
    """Split the body along a jagged, slightly sloping line at `at` of its height: (upper, lower) bool masks.
    The line slopes down toward the facing side so it follows an iso body's long axis."""
    xs, ys = grid()
    y0 = b.ground - at * b.height
    zig = np.array([0, 1, 2, 1, 0, -1, -2, -1, 0, 2, 1, -1, 0, 1, -2, -1], float)
    line = y0 + 0.18 * b.f * (xs[0] - b.cx) + zig[(np.arange(W) // 2) % len(zig)]
    upper = ys < line[None, :]
    return upper, ~upper, line


def collapse_death(b, flinch0, fl0, opts):
    """Tilt, darken, crack, break: the base sags into the ground (cut at the ground row, with a broken edge) and the
    upper part topples away from the target (or slumps nose-down) and lands behind it. Everything stays where the
    flinch's jolt left it (fl0's push), so frame 0 -> 1 does not jump."""
    f = b.f
    x0 = int(fl0.get("dx", 0))
    upper, lower, line = break_masks(b, opts.get("break_at", 0.5))
    pm_u = b.pm * upper[..., None]
    pm_l = b.pm * lower[..., None]
    mu = pm_u[..., 3] >= 128
    if not mu.any():
        mu = b.mask

    def piv_u_for(side):  # the upper part's own rocking point on `side`
        return support(mu, side)

    piv_l = b.contact(-f)
    G = b.ground

    def topple(ua, us, uf, sense):
        """The upper part tipped by ua degrees, slid us px away, and dropped the fraction uf of the way to resting on
        the ground just behind the base. sense -1: it tips over away from the target on its away-side edge (a tall
        top); +1: its front end drops while its back end stays put (a long barrel slumping nose-down)."""
        if sense < 0:
            rot = b.rigid(-ua * f, -us * f, 0, pivot=piv_u_for(-f), pm=pm_u)
        else:  # a slump past level reads as broken, so a nose-down turns further
            rot = b.rigid(1.4 * ua * f, -us * f, 0, pivot=piv_u_for(-f), pm=pm_u)
        a_u = rot[..., 3] >= 128
        fall = 0
        if a_u.any():
            low = np.nonzero(a_u.any(1))[0].max()
            fall = int(round(uf * max(0, (G - max(1, us // 2)) - low)))
        return shift_pm(rot, 0, fall), fall

    # a piece falls the way that brings it lowest: a tall top topples away, a long barrel tips nose-down
    def centroid_y(pm):
        a = pm[..., 3]
        return float((a * np.arange(H)[:, None]).sum() / max(1.0, a.sum()))
    if opts.get("topple") in ("away", "nose-down"):
        sense = -1 if opts["topple"] == "away" else 1
    else:
        sense = max((-1, 1), key=lambda s: centroid_y(topple(25, 8, 1.0, s)[0]) + (0.5 if s < 0 else 0.0))
    # per frame: upper (tip deg, slide away px, fall 0..1 to the ground), base (lean away, drop px), darkness
    up = [None, (3, 0, 0.0), (6, 2, 0.06), (12, 4, 0.25), (20, 6, 0.6), (26, 8, 1.0), (24, 8, 0.9), (25, 8, 1.0)]
    lo = [None, (3, 0), (2.5, 1), (3, 2), (3.5, 3), (4, 4), (4, 4), (4, 4)]
    dark = [None, 0.9, 0.82, 0.74, 0.66, 0.6, 0.58, 0.56]
    frames, params = [flinch0], [dict(note="= flinch frame 0", topple="away" if sense < 0 else "nose-down")]
    for k in range(1, 8):
        ua, us, uf = up[k]
        la, ld = lo[k]
        if k == 1:  # still whole: tilts away, a crack shows along the break line
            whole = b.rigid(-ua * f, x0, 0, pivot=piv_l)
            crack = np.zeros((H, W), bool)
            xs = np.arange(W)
            ln = np.round(line).astype(int)
            crack[np.clip(ln, 0, H - 1), xs] = True
            crack_src = crack & b.mask & (bayer() < 0.6)
            cr = sample(crack_src.astype(np.float64)[..., None].repeat(4, 2) * 255.0,
                        *rigid_src(-ua * f, piv_l, x0), sharpen=SHARPEN, order=0)[..., 3] > 127
            whole = darken(whole, dark[k])
            whole[cr & (whole[..., 3] > 128)] = [20, 14, 12, 255]
            frames.append(whole)
            params.append(dict(lean=-ua * f, dx=x0, dark=dark[k], crack=True))
            continue
        base_l = b.rigid(-la * f, x0, ld, pivot=piv_l, pm=pm_l)
        rot_u, fall = topple(ua, us, uf, sense)
        rot_u = shift_pm(rot_u, x0, 0)
        # the part that slides away goes behind the base (away = deeper into the screen)
        out = over(rot_u, base_l)
        out = darken(out, dark[k])
        out = dust_cut(out, G, b, depth=ld)
        frames.append(out)
        params.append(dict(upper=dict(lean=ua * sense * f, dx=-us * f + x0, dy=fall), base=dict(lean=-la * f, dx=x0, dy=ld),
                           dark=dark[k]))
    return frames, params


def dust_cut(pm, G, b, depth):
    """The base has sunk `depth` px into rubble: clear below the ground row, and thin the last row above it through a
    Bayer threshold so the cut is a broken edge rather than a ruler line."""
    if depth <= 0:
        return pm
    out = pm.copy()
    out[G + 1:] = 0
    bm = bayer()
    edge = out[G][..., 3] >= 128
    thin = edge & (bm[G] < 0.35)
    out[G][thin] = 0
    return out


# ---------------------------------------------------------------- elementals and amorphous bodies
def field_src(b, lean=0.0, push=0, sx=1.0, sy=1.0, bulge=0.0, reach=False, lift=0, power=1.3, dip=0.0):
    """A smooth bend pinned at the ground: vertical scale about the ground row, a lean (shear) that grows with height
    (lean px at the top, + = toward +x), a horizontal scale about the centre (or about the back edge when `reach`, so
    the front stretches toward the target), a bulge that widens the middle, then a whole-pixel push / lift. `power`
    shapes the lean (higher: the top leads, as a serpent's head); `dip` moves the top down by that many px.
    A stretch (sy > 1) is capped at the headroom above the sprite, so nothing leaves the canvas."""
    xs, ys = grid()
    G, Ht = float(b.ground), float(b.height)
    if sy > 1.0:
        sy = min(sy, 1.0 + max(0, b.top - 1) / Ht)
    y = ys + lift
    y_s = G - (G - y) / sy
    h = np.clip((G - y_s) / Ht, 0.0, 1.0)
    y_s = y_s - dip * h ** power
    x1 = xs - push - lean * h ** power
    ax = (b.left if b.f > 0 else b.right) if reach else b.cx
    x_s = ax + (x1 - ax) / (sx * (1.0 + bulge * np.sin(np.pi * h)))
    return x_s, y_s


def elemental_anims(b, opts):
    f = b.f
    Ht = float(b.height)
    hw = (b.right - b.left) / 2.0
    P = {}
    br = [0.0, 0.5, 1.0, 0.5]
    idle = [dict(sy=1.0 + 1.6 * t / Ht, bulge=1.2 * t / hw) for t in br]
    strike = [dict(),
              dict(lean=-3 * f, sy=0.97, sx=1.02),
              dict(lean=-6 * f, push=-1 * f, sy=0.94, sx=1.04),
              dict(lean=6 * f, push=2 * f, sy=1.02, sx=1.06, reach=True),
              dict(lean=12 * f, push=4 * f, sy=0.97, sx=1.12, reach=True),
              dict(lean=8 * f, push=3 * f, sy=0.98, sx=1.08, reach=True),
              dict(lean=3 * f, push=1 * f, sy=1.0, sx=1.03, reach=True),
              dict(lean=-0.6 * f, sy=1.0 + 0.6 / Ht)]
    flinch = [dict(lean=-7 * f, push=-2 * f, sy=0.88, sx=1.08),
              dict(lean=-9 * f, push=-2 * f, sy=0.9, sx=1.06),
              dict(lean=2 * f, push=0, sy=1.03, sx=0.98),
              dict(lean=-0.6 * f, sy=1.0 - 0.6 / Ht)]
    P["idle"], P["strike"], P["flinch"] = idle, strike, flinch

    def render(p):
        return sample(b.pm, *field_src(b, **p))

    frames = {a: [b.pm if (a in ("idle", "strike") and i == 0) else render(p) for i, p in enumerate(ps)]
              for a, ps in P.items()}
    frames["death"], P["death"] = dissolve_death(b, frames["flinch"][0], P["flinch"][0], opts)
    return frames, P


def dissolve_order(b, seed=7):
    """Where the dissolve eats first (0 = last to go, 1 = first): high up and near the edge first, the low core last,
    with a little low-frequency noise so the front is organic. Lives on the sprite (warped with it)."""
    m = b.mask | (b.pm[..., 3] > 8)
    dist = ndimage.distance_transform_edt(m)
    dmax = max(1.0, np.percentile(dist[m], 95))
    ys = np.arange(H)[:, None].repeat(W, 1).astype(float)
    h = np.clip((b.ground - ys) / b.height, 0, 1)
    rng = np.random.default_rng(seed)
    noise = ndimage.gaussian_filter(rng.standard_normal((H, W)), 3.0)
    noise /= max(1e-6, np.abs(noise).max())
    v = 0.62 * h + 0.38 * (1 - np.clip(dist / dmax, 0, 1)) + 0.1 * noise
    vm = v[m]
    v = (v - vm.min()) / max(1e-6, vm.max() - vm.min())
    return np.clip(v, 0, 1)


def dissolve_death(b, flinch0, fl0, opts):
    f = b.f
    puddle = opts.get("end") == "puddle"
    order_map = dissolve_order(b)
    # progress eaten, vertical slump, spread, drift away
    if puddle:  # the low core stays: a flat, faint pool of its own colours
        eaten = [None, 0.08, 0.2, 0.34, 0.48, 0.58, 0.64, 0.66]
        slump = [None, 0.96, 0.9, 0.8, 0.62, 0.42, 0.25, 0.13]
        spread = [None, 1.02, 1.05, 1.1, 1.18, 1.28, 1.36, 1.42]
    else:  # wisps: almost all of it goes, a few pixels linger where it stood
        eaten = [None, 0.08, 0.2, 0.36, 0.52, 0.66, 0.78, 0.86]
        slump = [None, 0.96, 0.9, 0.8, 0.66, 0.5, 0.38, 0.3]
        spread = [None, 1.02, 1.05, 1.1, 1.16, 1.22, 1.26, 1.28]
    lean = [None, -6, -6, -5, -4, -3, -2, -1]
    drift = [None, -2, -2, -3, -3, -4, -4, -4]
    frames, params = [flinch0], [dict(fl0, note="= flinch frame 0")]
    spread_w = 0.35
    bm = bayer()
    for k in range(1, 8):
        p = dict(lean=lean[k] * f, push=drift[k] * f, sy=slump[k], sx=spread[k])
        src = field_src(b, **p)
        pm = sample(b.pm, *src)
        v = ndimage.map_coordinates(order_map, [src[1], src[0]], order=0, mode="constant", cval=1.0)
        e = eaten[k]
        keep = v + spread_w * (bm - 0.5) < 1.0 + spread_w / 2 - e * (1.0 + spread_w)
        out = pm * keep[..., None]
        if (out[..., 3] >= 128).sum() < 4:  # never a blank held frame: keep the last few pixels to go
            alive = pm[..., 3] >= 128
            if alive.any():
                vv = np.where(alive, v, 9)
                idx = np.argsort(vv.ravel())[:4]
                kk = np.zeros(H * W, bool)
                kk[idx] = True
                out = pm * kk.reshape(H, W)[..., None]
        frames.append(out)
        params.append(dict(p, eaten=e))
    return frames, params


# ---------------------------------------------------------------- shared helpers for the later body plans
def rest_on_ground(pm, G, sink=0):
    """Move a frame up or down by whole pixels so its lowest solid row lies on the ground row (+ sink px into it)."""
    rows = np.nonzero((pm[..., 3] >= 128).any(1))[0]
    if not len(rows):
        return pm, 0
    dy = int(G + sink - rows.max())
    return shift_pm(pm, 0, dy), dy


def keep_above_ground(pm, G):
    """Lift a frame by whole pixels if anything solid went below the ground row (a flier never digs in)."""
    rows = np.nonzero((pm[..., 3] >= 128).any(1))[0]
    if len(rows) and rows.max() > G:
        return shift_pm(pm, 0, int(G - rows.max())), int(G - rows.max())
    return pm, 0


def settle(pm, G):
    """A corpse settling: sink it 1 px and thin the contact row through a Bayer threshold (a broken, dusty edge)."""
    out, _ = rest_on_ground(pm, G, sink=1)
    return dust_cut(out, G, None, depth=1)


def region_weight(b, poly, blur=2.5):
    """A soft 0..1 weight over a polygon given in SPRITE pixel coordinates (moved onto the canvas)."""
    m = Image.new("L", (W, H), 0)
    ox, oy = b.offset
    ImageDraw.Draw(m).polygon([(x + ox, y + oy) for x, y in poly], fill=255)
    w = ndimage.gaussian_filter(np.asarray(m).astype(np.float64) / 255.0, blur)
    return np.clip(w * 1.25, 0, 1)


def ring_rotate_src(src_x, src_y, centre, weight, deg):
    """Compose a further turn of a weighted region (the ophanim's rings, a weapon arm) about `centre` into a backward
    map: the region turns by `deg` (> 0: its top toward +x), fading out over the weight's soft edge."""
    w = ndimage.map_coordinates(weight, [src_y, src_x], order=1, mode="constant", cval=0.0)
    a = np.radians(deg) * w
    c, sn = np.cos(a), np.sin(a)
    ex, ey = src_x - centre[0], src_y - centre[1]
    return c * ex + sn * ey + centre[0], -sn * ex + c * ey + centre[1]


# ---------------------------------------------------------------- flying: a bird (the hawk)
def bird_anims(b, opts):
    """Idle: hovers (1-2 px bob) with a wing flick (the top of the sprite stretches up, then down). Strike: rises and
    draws back, then dives at the target, pitched onto its talons (frame 4). Flinch: jolted up and away, tilted back.
    Death: tumbles over (rotates through 190 degrees away from the target), lands on its back on the ground and
    settles."""
    f, G = b.f, b.ground
    centre = (b.cx, (b.top + b.ground) / 2.0)
    idle = [dict(), dict(lift=1, sy=1.04), dict(lift=2, sy=0.97), dict(lift=1, sy=1.0)]
    strike = [dict(lean=0.0),
              dict(lean=-8 * f, dx=-2 * f, dy=-4),
              dict(lean=-12 * f, dx=-3 * f, dy=-6),
              dict(lean=10 * f, dx=4 * f, dy=-2),
              dict(lean=22 * f, dx=10 * f, dy=0),
              dict(lean=15 * f, dx=8 * f, dy=0),
              dict(lean=5 * f, dx=3 * f, dy=-2),
              dict(lean=0.0, dx=0, dy=-1)]
    flinch = [dict(lean=-14 * f, dx=-4 * f, dy=-5),
              dict(lean=-10 * f, dx=-3 * f, dy=-4),
              dict(lean=3 * f, dx=-1 * f, dy=-1),
              dict(lean=-1 * f, dx=0, dy=0)]
    death = [None,
             dict(lean=-45 * f, dx=-5 * f, dy=-6),
             dict(lean=-100 * f, dx=-6 * f, dy=-3),
             dict(lean=-150 * f, dx=-7 * f, dy=0),
             dict(lean=-185 * f, dx=-8 * f, rest=True),
             dict(lean=-190 * f, dx=-8 * f, sy=0.85, rest=True),
             dict(lean=-188 * f, dx=-8 * f, sy=0.88, rest=True, bounce=2),
             dict(lean=-190 * f, dx=-8 * f, sy=0.85, rest=True, settle=True)]

    def rig(p):
        out = sample(b.pm, *rigid_src(p.get("lean", 0.0), centre, int(p.get("dx", 0)), int(p.get("dy", 0)),
                                      1.0, p.get("sy", 1.0)))
        if p.get("rest"):
            out, _ = rest_on_ground(out, G)
            if p.get("bounce"):
                out = shift_pm(out, 0, -p["bounce"])
            if p.get("settle"):
                out = settle(out, G)
            return out
        return keep_above_ground(out, G)[0]

    frames = {"idle": [b.pm] + [sample(b.pm, *field_src(b, **p)) for p in idle[1:]],
              "strike": [b.pm] + [rig(p) for p in strike[1:]],
              "flinch": [rig(p) for p in flinch]}
    frames["death"] = [frames["flinch"][0]] + [rig(p) for p in death[1:]]
    death[0] = dict(flinch[0], note="= flinch frame 0")
    return frames, {"idle": idle, "strike": strike, "flinch": flinch, "death": death}


# ---------------------------------------------------------------- flying: a rigid fiery wheel (the ophanim)
def wheel_anims(b, opts):
    """Idle: the rings twist a few degrees back and forth inside the still wings (a spin-like shimmer) while the whole
    floats 1 px. Strike: rolls back, then lurches and rolls toward the target, the rings turning further than the
    body (frame 4). Flinch: a wobble. Death: dims, cracks and collapses (the machine collapse)."""
    f, G = b.f, b.ground
    cx, cy, r = opts.get("ring", (b.cx - b.offset[0], (b.top + b.ground) / 2.0 - b.offset[1], 32))
    centre = (cx + b.offset[0], cy + b.offset[1])
    ys, xs = np.mgrid[0:H, 0:W]
    ring_w = np.clip((r + 2 - np.hypot(xs - centre[0], ys - centre[1])) / 4.0, 0, 1)
    ring_w = ndimage.gaussian_filter(ring_w, 1.0)

    idle = [dict(), dict(ring=3 * f, lift=1), dict(ring=0, lift=1), dict(ring=-3 * f, lift=0)]
    strike = [dict(),
              dict(lean=-4 * f, dx=-2 * f, ring=-8 * f),
              dict(lean=-8 * f, dx=-3 * f, ring=-14 * f),
              dict(lean=6 * f, dx=3 * f, ring=12 * f),
              dict(lean=14 * f, dx=8 * f, ring=30 * f),
              dict(lean=10 * f, dx=6 * f, ring=24 * f),
              dict(lean=4 * f, dx=2 * f, ring=8 * f),
              dict(lean=-1 * f, dx=0, ring=-1 * f)]
    flinch = [dict(lean=-8 * f, dx=-3 * f, ring=-10 * f),
              dict(lean=6 * f, dx=-2 * f, ring=6 * f),
              dict(lean=-3 * f, dx=-1 * f, ring=-3 * f),
              dict(lean=1 * f, dx=0, ring=1 * f)]

    def render(p):
        # undo the body move first, then the ring turn (the rings turn in the body's own frame)
        src = rigid_src(p.get("lean", 0.0), centre, int(p.get("dx", 0)), -int(p.get("lift", 0)))
        if p.get("ring"):
            src = ring_rotate_src(*src, centre, ring_w, p["ring"])
        return keep_above_ground(sample(b.pm, *src), G)[0]

    frames = {"idle": [b.pm] + [render(p) for p in idle[1:]],
              "strike": [b.pm] + [render(p) for p in strike[1:]],
              "flinch": [render(p) for p in flinch]}
    P = {"idle": idle, "strike": strike, "flinch": flinch}
    frames["death"], P["death"] = collapse_death(b, frames["flinch"][0], flinch[0], dict(opts, topple="away"))
    return frames, P


# ---------------------------------------------------------------- serpents and the kraken (in water)
def serpent_anims(b, opts):
    """In the water: idle a slow sway (the head leads) and a slight rise; strike: the neck rears back, then the head
    surges and strikes down toward the target (frame 4); flinch: a recoil away; death: lists away and sinks through
    the waterline like a ship, foam included."""
    f, G = b.f, b.ground
    pw = opts.get("power", 2.0)
    idle = [dict(), dict(lean=1.5 * f, sy=1.012, power=pw), dict(lean=0, sy=1.02, power=pw),
            dict(lean=-1.5 * f, sy=1.012, power=pw)]
    strike = [dict(),
              dict(lean=-4 * f, sy=1.02, power=pw),
              dict(lean=-8 * f, sy=1.035, power=pw),
              dict(lean=8 * f, dip=2, power=pw),
              dict(lean=17 * f, dip=7, push=2 * f, power=pw),
              dict(lean=12 * f, dip=4, push=2 * f, power=pw),
              dict(lean=4 * f, dip=1, power=pw),
              dict(lean=-0.6 * f, power=pw)]
    flinch = [dict(lean=-9 * f, sy=0.95, push=-1 * f, power=pw),
              dict(lean=-11 * f, sy=0.94, push=-1 * f, power=pw),
              dict(lean=2 * f, sy=1.0, power=pw),
              dict(lean=-0.6 * f, power=pw)]
    P = {"idle": idle, "strike": strike, "flinch": flinch}
    frames = {a: [b.pm if (a in ("idle", "strike") and i == 0) else sample(b.pm, *field_src(b, **p))
                  for i, p in enumerate(ps)] for a, ps in P.items()}
    frames["death"], P["death"] = ship_death(b, frames["flinch"][0], flinch[0], (b.cx, float(G)), drift0=1)
    return frames, P


# ---------------------------------------------------------------- big beasts (dragons, mammoths, the tortoise)
def lunge_src(b, fwd=0.0, up=0.0, sy=1.0, push=0, lean=0.0, power=1.4):
    """A body warp led by the head and the front: each pixel moves `fwd` px toward the facing and `up` px up in
    proportion to how far forward (toward the facing side) and how high it is, so the hindquarters stay planted and
    the head leads. Plus a vertical scale about the ground, a lean (shear, px at the top) and a whole-pixel push."""
    xs, ys = grid()
    G, Ht = float(b.ground), float(b.height)
    if sy > 1.0:
        sy = min(sy, 1.0 + max(0, b.top - 1) / Ht)
    y0 = G - (G - ys) / sy
    h = np.clip((G - y0) / Ht, 0, 1)
    back = b.left if b.f > 0 else b.right
    fr = np.clip((xs - back) * b.f / max(1.0, b.right - b.left), 0, 1) ** power
    w = fr * (0.3 + 0.7 * h)
    return xs - push - b.f * fwd * w - lean * h ** 1.3, y0 + up * w


def fold_src(b, fold=0.0, legs=0.32, lean=0.0, push=0, back=0.0):
    """Legs folding under a heavy body: the bottom `legs` share of the height compresses by `fold` (0..0.92) and
    everything above drops by as much, so the belly comes down onto the ground; `lean` (px at the top) rolls it onto
    its side, `back` pulls the head and front back toward the body."""
    xs, ys = grid()
    G, Ht = float(b.ground), float(b.height)
    L = legs * Ht
    c = max(0.08, 1.0 - fold)
    ho = G - ys
    hs = np.where(ho < L * c, ho / c, ho + L * (1 - c))
    y_s = G - hs
    h = np.clip(hs / Ht, 0, 1)
    x_s = xs - push - lean * h ** 1.3
    if back:
        bk = b.left if b.f > 0 else b.right
        fr = np.clip((xs - bk) * b.f / max(1.0, b.right - b.left), 0, 1) ** 1.4
        x_s = x_s + b.f * back * fr
    return x_s, y_s


def beast_anims(b, opts):
    """Idle: a slow breath (swell pinned at the ground). Strike: rears back (head up and back), then lunges, the head
    and front leading down toward the target (frame 4). Flinch: the head and front recoil up and back. Death: the legs
    fold under it and it rolls onto its side away from the target, darkening a little, and settles on the tile. The
    giant tortoise instead withdraws its head and flippers and slumps."""
    f, G = b.f, b.ground
    Ht, hw = float(b.height), (b.right - b.left) / 2.0
    br = [0.0, 0.5, 1.0, 0.5]
    idle = [dict(sy=1.0 + 1.4 * t / Ht, bulge=1.0 * t / hw) for t in br]
    strike = [dict(),
              dict(fwd=-3, up=3),
              dict(fwd=-6, up=5),
              dict(fwd=6, up=-1, push=1 * f),
              dict(fwd=14, up=-5, push=2 * f),
              dict(fwd=10, up=-3, push=2 * f),
              dict(fwd=4, up=-1, push=1 * f),
              dict(fwd=-0.5)]
    flinch = [dict(fwd=-7, up=4, sy=0.98, push=-2 * f),
              dict(fwd=-9, up=5, sy=0.98, push=-2 * f),
              dict(fwd=2, up=0),
              dict(fwd=-0.5)]
    legs = opts.get("legs", 0.32)
    if opts.get("tortoise"):
        death = [None] + [dict(fold=fd, legs=legs, back=bk, lean=-ln * f, push=-2 * f, dark=dk) for fd, bk, ln, dk in
                          ((0.05, 3, 1, 0.97), (0.12, 7, 2, 0.94), (0.2, 11, 2, 0.9), (0.28, 13, 3, 0.86),
                           (0.34, 14, 3, 0.83), (0.32, 14, 3, 0.82), (0.34, 14, 3, 0.8))]
    else:
        death = [None] + [dict(fold=fd, legs=legs, lean=-ln * f, push=-2 * f, dark=dk) for fd, ln, dk in
                          ((0.08, 3, 0.97), (0.25, 5, 0.94), (0.45, 8, 0.9), (0.7, 12, 0.86), (0.9, 16, 0.83),
                           (0.85, 15, 0.82), (0.9, 16, 0.8))]
    P = {"idle": idle, "strike": strike, "flinch": flinch, "death": death}
    frames = {"idle": [b.pm] + [sample(b.pm, *field_src(b, **p)) for p in idle[1:]],
              "strike": [b.pm] + [sample(b.pm, *lunge_src(b, **p)) for p in strike[1:]],
              "flinch": [sample(b.pm, *lunge_src(b, **p)) for p in flinch]}
    dfr = [frames["flinch"][0]]
    for k in range(1, 8):
        p = {kk: v for kk, v in death[k].items() if kk != "dark"}
        out = darken(sample(b.pm, *fold_src(b, **p)), death[k]["dark"], soot=(40, 34, 30))
        dfr.append(settle(out, G) if k == 7 else out)
    frames["death"] = dfr
    death[0] = dict(flinch[0], note="= flinch frame 0")
    return frames, P


# ---------------------------------------------------------------- big bipeds (giants, golems, the rearing bear)
def bend_src(b, alpha=0.0, hip=0.45, band=0.14, push=0, sy=1.0, arm=None, beta=0.0):
    """The upper body turns about the hips by `alpha` degrees (> 0: its top toward +x), fading in over a band above
    the hips so the legs stay planted; the weapon arm (a soft region, `arm` = (shoulder, weight)) turns a further
    `beta` degrees about the shoulder. Plus a vertical scale about the ground and a whole-pixel push."""
    xs, ys = grid()
    G, Ht = float(b.ground), float(b.height)
    if sy > 1.0:
        sy = min(sy, 1.0 + max(0, b.top - 1) / Ht)
    y0 = G - (G - ys) / sy
    x0 = xs - push
    hip_y = G - hip * Ht
    row = b.mask[int(round(hip_y))]
    hip_x = float(np.nonzero(row)[0].mean()) if row.any() else b.cx
    t = np.clip((hip_y - y0) / (band * Ht), 0, 1)
    a = np.radians(alpha) * t * t * (3 - 2 * t)
    c, sn = np.cos(a), np.sin(a)
    dx, dy = x0 - hip_x, y0 - hip_y
    qx, qy = c * dx + sn * dy + hip_x, -sn * dx + c * dy + hip_y
    if arm is not None and beta:
        qx, qy = ring_rotate_src(qx, qy, arm[0], arm[1], beta)
    return qx, qy


def giant_anims(b, opts):
    """Idle: a breath. Strike: the upper body (and the weapon arm, where one is marked) leans back, then drives toward
    the target (frame 4) and recovers. Flinch: the upper body recoils away. Death: knees give, then it topples away from
    the target about its far foot, foreshortened as it comes to lie along the tile's diagonal, rests on the ground and
    settles."""
    f, G = b.f, b.ground
    Ht, hw = float(b.height), (b.right - b.left) / 2.0
    hip = opts.get("hip", 0.45)
    arm = None
    if opts.get("arm"):
        sh, poly = opts["arm"]
        arm = ((sh[0] + b.offset[0], sh[1] + b.offset[1]), region_weight(b, poly))
    br = [0.0, 0.5, 1.0, 0.5]
    idle = [dict(sy=1.0 + 1.4 * t / Ht, bulge=1.0 * t / hw) for t in br]
    sw = [(0, 0, 0), (-6, -10, 0), (-11, -22, -1), (8, 15, 1), (17, 35, 3), (13, 28, 3), (5, 10, 1), (-0.6, -1, 0)]
    strike = [dict(alpha=a * f, beta=bt * f, push=pu * f, hip=hip) for a, bt, pu in sw]
    flinch = [dict(alpha=-10 * f, beta=-8 * f, push=-3 * f, hip=hip, sy=0.98),
              dict(alpha=-12 * f, beta=-10 * f, push=-3 * f, hip=hip, sy=0.97),
              dict(alpha=3 * f, beta=3 * f, push=-1 * f, hip=hip),
              dict(alpha=-0.6 * f, hip=hip)]
    # topple: degrees away, foreshortening of the height, share of the way the corpse slides back under the anchor,
    # knee squash (the first frames)
    tp = [None, (8, 1.0, 0.0, 0.97), (18, 0.98, 0.0, 0.94), (30, 0.93, 0.1, 0.92), (46, 0.88, 0.3, 1.0),
          (62, 0.8, 0.5, 1.0), (59, 0.81, 0.5, 1.0), (62, 0.8, 0.5, 1.0)]
    death = [None] + [dict(lean=-d * f, sy=s * k, centre=c) for d, s, c, k in tp[1:]]
    P = {"idle": idle, "strike": strike, "flinch": flinch, "death": death}
    frames = {"idle": [b.pm] + [sample(b.pm, *field_src(b, **p)) for p in idle[1:]],
              "strike": [b.pm] + [sample(b.pm, *bend_src(b, arm=arm, **p)) for p in strike[1:]],
              "flinch": [sample(b.pm, *bend_src(b, arm=arm, **p)) for p in flinch]}
    foot = b.contact(-f)
    x_push = int(flinch[0]["push"])
    dfr = [frames["flinch"][0]]
    for k in range(1, 8):
        p = death[k]
        out = sample(b.pm, *rigid_src(p["lean"], foot, x_push, 0, 1.0, p["sy"]))
        out, _ = rest_on_ground(out, G)
        a = out[..., 3] >= 128
        if a.any() and p["centre"]:
            cols = np.nonzero(a.any(0))[0]
            mid = (cols.min() + cols.max()) / 2.0
            out = shift_pm(out, int(round((ANCHOR[0] - mid) * p["centre"])), 0)
        if k == 6:
            out = shift_pm(out, 0, -1)
        if k == 7:
            out = settle(out, G)
        dfr.append(out)
    frames["death"] = dfr
    death[0] = dict(flinch[0], note="= flinch frame 0")
    return frames, P


# ---------------------------------------------------------------- output (the recipe's format, recipe.write)
def write_set(b, frames, params, plan, opts, base):
    """final_<key>_nofx/ and final_<key>/ (the latter with the recipe's white hit flash on flinch and death frame 0),
    each with <anim>/frame_NN.png, <anim>/manifest.json, sheet_<anim>.png and unit_poses.json. `frames` are the
    finished images ({anim: [PIL]}, any canvas size), `base` the static sprite on that canvas."""
    dirs = []
    for suffix, fx in (("_nofx", False), ("", True)):
        d = ROOT / f"final_{b.key}{suffix}"
        for anim in ("idle", "strike", "flinch", "death"):
            fr = list(frames[anim])
            if anim in ("idle", "strike"):
                fr[0] = base.copy()
            if fx and anim in ("flinch", "death"):
                fr[0] = flash(fr[0], 0.6)
            write_anim(d, anim, fr, b, plan)
        (d / "unit_poses.json").write_text(json.dumps({
            "unit": b.key, "facing": b.facing, "pose_set": f"procedural-{plan}", "mirrored": False, "legacy": False,
            "method": "lib/procedural.py: rigid/field warps of the sprite's own pixels, no image model",
            "sprite": str(b.sprite_path.relative_to(ROOT.parent)).replace("\\", "/"),
            "canvas": list(base.size), "anchor": [base.size[0] // 2, base.size[1] - 2],
            "options": opts, "anims": params}, default=_jsonable))
        dirs.append(d)
    return dirs


def _jsonable(o):
    if isinstance(o, (np.floating, np.integer)):
        return o.item()
    if isinstance(o, tuple):
        return list(o)
    return str(o)


def flash(im, amount=0.6, colour=(255, 255, 255)):
    """finish.flash: the recipe's hit flash."""
    a = np.asarray(im.convert("RGBA")).astype(np.float32)
    a[..., :3] = a[..., :3] * (1 - amount) + np.array(colour, np.float32) * amount
    return Image.fromarray(a.astype(np.uint8), "RGBA")


def write_anim(d, anim, frames, b, plan):
    """Exactly what recipe.write (via animkit.write_anim) writes, straight into `d`; the canvas and anchor come from
    the frames (W x H, anchor (W/2, H-2)), so oversize strips say their own size."""
    n = beats.COUNTS[anim]
    assert len(frames) == n, (anim, len(frames))
    ms, events = beats.ms(anim), beats.events(anim)
    warns = []
    fw, fh = frames[0].size
    for i, f in enumerate(frames):
        assert f.size == (fw, fh)
        bb = ak.bbox(f, 40)
        if fw == ak.W and bb and (bb[0] < ak.SAFE_X[0] or bb[2] > ak.SAFE_X[1]):
            warns.append(f"frame {i} leaves the safe region x {ak.SAFE_X}: bbox {bb}")
    ad = d / anim
    ad.mkdir(parents=True, exist_ok=True)
    for old in ad.glob("frame_*.png"):
        old.unlink()
    man = {"unit": b.key, "facing": b.facing, "arm": d.name, "anim": anim, "canvas": [fw, fh], "anchor": [fw // 2, fh - 2],
           "loop": beats.LOOP[anim], "hold_last": beats.HOLD_LAST[anim], "frames": []}
    for i, f in enumerate(frames):
        fn = f"frame_{i:02d}.png"
        f.save(ad / fn)
        man["frames"].append({"file": fn, "ms": int(ms[i]), "events": list(events[i])})
    man["note"] = f"procedural ({plan}): lib/procedural.py"
    if warns:
        man["warnings"] = warns
    (ad / "manifest.json").write_text(json.dumps(man, indent=1))
    sheet = Image.new("RGBA", (fw * n, fh), (0, 0, 0, 0))
    for i, f in enumerate(frames):
        sheet.alpha_composite(f, (i * fw, 0))
    sheet.save(d / f"sheet_{anim}.png")


# ---------------------------------------------------------------- driver
RECIPES = {"ship": ship_anims, "siege_or_vehicle": siege_anims, "elemental_or_amorphous": elemental_anims,
           "bird": bird_anims, "wheel": wheel_anims, "serpent": serpent_anims, "beast": beast_anims,
           "giant": giant_anims}
# groups of units: the first three are whole body plans of roster_classified.json, the rest named units
GROUPS = {
    "ship": "ship", "siege_or_vehicle": "siege_or_vehicle", "elemental_or_amorphous": "elemental_or_amorphous",
    "flying": ["unit_hawk", "unit_ophanim"],
    "serpent": ["unit_leviathan", "unit_sea_serpent"],
    "oversize_beast": ["unit_abashi", "unit_acheron", "unit_drifa", "unit_eurabatres", "unit_beast_of_agares",
                       "unit_elephant", "unit_giant_tortoise", "unit_margalard", "unit_kraken"],
    "oversize_biped": ["unit_auric_ascended", "unit_flagbearer", "unit_hill_giant", "unit_mithril_golem",
                       "unit_wilboman", "unit_wrath", "unit_gurid"],
}
DEFAULT_GROUPS = list(GROUPS)


def roster(groups=DEFAULT_GROUPS, names=None):
    r = json.loads(ROSTER.read_text())
    out = []
    for g in groups:
        sel = GROUPS[g]
        for u in r:
            if (u["body_plan"] == sel if isinstance(sel, str) else u["name"] in sel) and (not names or u["name"] in names):
                if u not in out:
                    out.append(dict(u, group=g))
    return out


def recipe_of(entry, opts):
    return opts.get("recipe") or entry["body_plan"]


def fit_width(frames, w0, h0, ax):
    """The narrowest canvas of 160, 176 or 192 px (centred on the anchor) that holds every pixel of every frame."""
    hw = 0
    for fr in frames:
        cols = np.nonzero((fr[..., 3] > 0).any(0))[0]
        if len(cols):
            hw = max(hw, ax - cols.min(), cols.max() + 1 - ax)
    for w in (160, 176, 192):
        if w // 2 >= hw:
            return w
    return w0


def animate(entry, write=True):
    """Animate one unit. Returns (body, {anim: [PIL frames]}, base) on the unit's final canvas."""
    opts = dict(UNIT.get(entry["name"], {}))
    facing = opts.pop("facing", entry["facing"])
    if facing not in ("lower-left", "lower-right"):
        raise ValueError(f"{entry['name']}: facing {facing!r}; add a lower-left/lower-right override to UNIT")
    tall = opts.get("canvas") == "tall"
    b = Body(entry["name"], facing, TALL if tall else (ak.W, ak.H))
    plan = recipe_of(entry, opts)
    frames, params = RECIPES[plan](b, opts)
    base = b.base
    if tall:  # crop the working canvas to the narrowest of 160/176/192 that holds everything, centred on the anchor
        fw = fit_width([p for fr in frames.values() for p in fr], W, H, ANCHOR[0])
        x0 = ANCHOR[0] - fw // 2
        box = (x0, 0, x0 + fw, H)
        pil = {a: [frame(p).crop(box) for p in fr] for a, fr in frames.items()}
        base = b.base.crop(box)
    else:
        pil = {a: [frame(p) for p in fr] for a, fr in frames.items()}
    if write:
        write_set(b, pil, params, plan, {k: v for k, v in opts.items() if k != "arm"}, base)
    return b, pil, base


# ---------------------------------------------------------------- review
WATER, GRASS = (58, 104, 140, 255), (126, 158, 92, 255)


def ground_colour(entry, opts):
    return WATER if recipe_of(entry, opts) in ("ship", "serpent") and not opts.get("death") == "collapse" else GRASS


def review(units, out, scale=2):
    """work/procedural/<group>_review.png: per unit, each animation's frames at `scale`x side by side (frame 0 first),
    on water (ships, serpents) or grass, with the ground row marked and per-frame ms; on the right the same frames at
    game scale (1x). Units on a taller or wider canvas get bigger cells."""
    rows = []
    lab_h = 26
    for entry in units:
        b, fr, base = animate(entry, write=False)
        bg = ground_colour(entry, dict(UNIT.get(entry["name"], {})))
        fw, fh = base.size
        ground = int(np.nonzero((np.asarray(base)[..., 3] >= 128).any(1))[0].max())
        for anim in ("idle", "strike", "flinch", "death"):
            row = Image.new("RGBA", (8 * fw * scale + 4 * fw + 12, max(fh * scale, 2 * fh) + lab_h), (235, 236, 228, 255))
            d = ImageDraw.Draw(row)
            ms, ev = beats.ms(anim), beats.events(anim)
            d.text((4, 1), f"{b.key} ({b.facing}, {fw}x{fh})  {anim}", fill=(120, 20, 20, 255))
            d.text((8 * fw * scale + 12, 1), "game scale (1x)", fill=(120, 20, 20, 255))
            for i, f in enumerate(fr[anim]):
                cell = Image.new("RGBA", (fw, fh), bg)
                ImageDraw.Draw(cell).line([(0, ground + 1), (fw, ground + 1)], fill=tuple(int(c * 0.8) for c in bg[:3]) + (255,))
                cell.alpha_composite(f)
                row.alpha_composite(cell.resize((fw * scale, fh * scale), Image.NEAREST), (i * fw * scale, lab_h))
                d.text((i * fw * scale + 4, 13), f"frame {i}: {ms[i]} ms{'  HIT' if 'hit' in ev[i] else ''}",
                       fill=(30, 30, 30, 255))
                small = Image.new("RGBA", (fw, fh), bg)
                small.alpha_composite(f)
                row.alpha_composite(small, (8 * fw * scale + 12 + (i % 4) * fw, lab_h + (i // 4) * fh))
            rows.append(row)
    sheet = Image.new("RGBA", (max(r.width for r in rows), sum(r.height + 4 for r in rows)), (255, 255, 255, 255))
    y = 0
    for r in rows:
        sheet.alpha_composite(r, (0, y))
        y += r.height + 4
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.convert("RGB").save(out)
    return out


def overview(units, anim, out):
    """work/procedural/ov_<group>_<anim>.png: every unit of a group, one row each, the animation's frames at 1x."""
    rows = []
    for entry in units:
        b, fr, base = animate(entry, write=False)
        bg = ground_colour(entry, dict(UNIT.get(entry["name"], {})))
        fw, fh = base.size
        row = Image.new("RGBA", (8 * (fw + 2), fh + 12), (235, 236, 228, 255))
        ImageDraw.Draw(row).text((2, 0), f"{b.key} ({b.facing})", fill=(150, 0, 0, 255))
        for i, f in enumerate(fr[anim]):
            c = Image.new("RGBA", (fw, fh), bg)
            c.alpha_composite(f)
            row.alpha_composite(c, (i * (fw + 2), 12))
        rows.append(row)
    sheet = Image.new("RGBA", (max(r.width for r in rows), sum(r.height + 2 for r in rows)), (255, 255, 255, 255))
    y = 0
    for r in rows:
        sheet.alpha_composite(r, (0, y))
        y += r.height + 2
    sheet.convert("RGB").save(out)
    return out


REVIEW_UNITS = {"ship": ["unit_frigate", "unit_galley", "unit_workboat", "unit_airship"],
                "siege_or_vehicle": ["unit_cannon", "unit_battering_ram", "unit_dwarven_cannon"],
                "elemental_or_amorphous": ["unit_fire_elemental", "unit_water_elemental", "unit_wraith",
                                           "unit_earth_elemental"]}


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--groups", "--plans", nargs="*", default=DEFAULT_GROUPS, help=f"of {list(GROUPS)}")
    ap.add_argument("--units", nargs="*")
    ap.add_argument("--review", action="store_true", help="also write work/procedural/<group>_review.png and "
                    "ov_<group>_<anim>.png")
    ap.add_argument("--review-units", nargs="*", help="units for the review image (default: a few per group)")
    ap.add_argument("--no-write", action="store_true")
    a = ap.parse_args()
    todo = roster(a.groups, a.units)
    if not a.no_write:
        for e in todo:
            b, fr, base = animate(e)
            print(f"{e['group']:22} {e['name']:28} {b.facing:11} {base.size[0]}x{base.size[1]} -> final_{b.key}_nofx/, "
                  f"final_{b.key}/")
    if a.review:
        for g in a.groups:
            us = roster([g])
            pick = a.review_units or REVIEW_UNITS.get(g)
            ru = [u for u in us if u["name"] in pick] if pick else us
            if ru:
                print(review(ru, ROOT / "work/procedural" / f"{g}_review.png"))
            if us:
                for anim in ("strike", "death"):
                    print(overview(us, anim, ROOT / "work/procedural" / f"ov_{g}_{anim}.png"))
