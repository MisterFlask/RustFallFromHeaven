"""Impact cards and key poses for the battle screen's ranged snap (ANIMATING_UNITS.md, "Impact cards and key poses").

An impact card is one still image of what a ranged blow does where it lands (the catapult's rubble), drawn in two
layers: `<name>_back.png` (behind the target: dust, smoke) and `<name>_front.png` (in front: the stones). The battle
screen (crates/ffh-play/assets/app/battle_snap.js) makes it arrive fast, brake hard, hold and fade.

A key pose is one 160x128 frame of a unit (the feet at 80, 126), held for the whole snap: `<unit>_attack.png` (the
shooter's) and `<unit>_hit.png` (the target's), for units without the full anim-sprint strips.

    python lib/impact_cards.py rubble ../crates/ffh-play/assets/app/fx
    python lib/impact_cards.py pose final_catapult/strike/frame_04.png ../crates/ffh-play/assets/app/pose/catapult_attack.png --empty-bucket
    python lib/impact_cards.py pose final_catapult/flinch/frame_01.png ../crates/ffh-play/assets/app/pose/catapult_hit.png
    python lib/impact_cards.py hawk final_hawk_nofx/strike/frame_04.png ../crates/ffh-play/assets/app/fx
    python lib/impact_cards.py pose ../crates/ffh-play/assets/app/iso/unit_ranger.png ../crates/ffh-play/assets/app/pose/ranger_attack.png --release-hawk

(`final_catapult` and `final_hawk_nofx` are anim-sprint sets on the `anim-all` branch.) `pose` also takes an M7 sprite
(its feet at the bottom centre, 2 px up), placed on the 160x128 canvas.

Pixel art at the sprites' pixel size: flat faces lit from the upper left, a 1 px dark outline, dust in four tones with an
ordered-dither edge. Deterministic (fixed seeds)."""
import argparse
import math
import random
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

BAYER4 = (np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]], float) + 0.5) / 16
OUTLINE = (24, 20, 22, 255)
STONE = [(206, 214, 214), (160, 170, 172), (116, 125, 130), (78, 84, 92)]  # light, mid, shade, deep
DUST = [(236, 206, 140), (205, 166, 101), (160, 122, 72), (112, 84, 52)]


def _poly(cx, cy, r, n, rng, squash):
    a0 = rng.uniform(0, math.tau)
    pts = []
    for i in range(n):
        a = a0 + i * math.tau / n + rng.uniform(-0.35, 0.35)
        rr = r * rng.uniform(0.72, 1.12)
        pts.append((cx + math.cos(a) * rr, cy + math.sin(a) * rr * squash))
    return pts


def stone(img, cx, cy, r, rng):
    """An angular chunk lit from the upper left: wedge faces round an off-centre point, a flat top, an outline, a crack."""
    n = rng.randint(5, 7)
    pts = _poly(cx, cy, r, n, rng, rng.uniform(0.75, 0.95))
    mask = Image.new("L", img.size, 0)
    ImageDraw.Draw(mask).polygon(pts, fill=255)
    m = np.array(mask) > 0
    yy, xx = np.mgrid[0:img.size[1], 0:img.size[0]]
    ccx, ccy = cx + r * 0.2, cy + r * 0.15
    lit = np.cos(np.arctan2(yy - ccy, xx - ccx) - math.radians(-135))
    lvl = np.where(np.hypot(xx - ccx, yy - ccy) < r * 0.38, 2, np.digitize(lit, [-0.45, 0.25, 0.7]))
    a = np.array(img)
    a[m, :3] = np.array(STONE[::-1], np.uint8)[lvl[m]]
    a[m, 3] = 255
    out = Image.fromarray(a)
    d = ImageDraw.Draw(out)
    d.polygon(pts, outline=OUTLINE)
    d.line([pts[rng.randrange(n)], (ccx + rng.uniform(-2, 2), ccy + rng.uniform(-2, 2))], fill=OUTLINE, width=1)
    if r > 9:
        q = (ccx + rng.uniform(-r * .3, r * .3), ccy + rng.uniform(-r * .3, r * .3))
        d.line([q, (q[0] + rng.uniform(3, 6), q[1] + rng.uniform(-3, 3))], fill=STONE[3] + (255,), width=1)
    return out


def dust(w, h, blobs, k):
    """Ochre dust: elliptical blobs (`sy` stretches one upward into a plume), roughened by value noise, posterized to
    four tones with a dithered edge."""
    yy, xx = np.mgrid[0:h, 0:w]
    f = np.zeros((h, w))
    for bx, by, br, sy in blobs:
        f = np.maximum(f, 1 - np.hypot(xx - bx * k, (yy - by * k) / sy) / (br * k))
    coarse = np.random.default_rng(11).random((h // 6 + 2, w // 6 + 2))
    n = np.array(Image.fromarray((coarse * 255).astype(np.uint8)).resize((w + 12, h + 12), Image.BILINEAR))[:h, :w] / 255
    f = np.clip(f - 0.22 * n, 0, 1)
    thr = BAYER4[yy % 4, xx % 4]
    a = np.zeros((h, w, 4), np.uint8)
    on = f > thr * 0.35
    tone = np.digitize(f + (thr - 0.5) * 0.12, [0.18, 0.42, 0.7])
    a[on, :3] = np.array(DUST[::-1], np.uint8)[tone[on]]
    a[on, 3] = 235
    return Image.fromarray(a)


def rubble(out: Path):
    """The catapult's stone landing: a split boulder and thrown chunks (front) in a dust plume (back). Both layers
    share one canvas; the card's foot (where it sits on the target's feet) is its bottom centre."""
    W, H = 100, 92
    back = dust(W, H, [(50, 76, 40, 0.42), (34, 46, 13, 2.6), (64, 40, 12, 2.9), (50, 52, 11, 2.2), (22, 62, 12, 1.4),
                       (80, 60, 12, 1.5), (40, 18, 7, 1.6), (68, 12, 6, 1.6)], 1.0)
    # the stones at 90% of the dust's geometry, standing on the same bottom centre
    k = 0.9
    rng = random.Random(7)
    front = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    sx = lambda x: W / 2 + (x - 50) * k
    sy = lambda y: H + (y - H) * k
    for cx, cy, r in [(40, 62, 17), (62, 60, 16), (51, 76, 13), (30, 78, 10), (73, 78, 10),
                      (52, 42, 10), (36, 38, 6), (68, 36, 6), (24, 56, 5), (82, 52, 5)]:
        front = stone(front, sx(cx), sy(cy), r * k, rng)
    d = ImageDraw.Draw(front)
    for x, y, s in [(44, 24, 2), (60, 20, 3), (28, 30, 2), (78, 30, 2), (18, 44, 2), (88, 42, 2), (54, 12, 2)]:
        x, y = round(sx(x)), round(sy(y))
        d.polygon([(x, y), (x + s, y - 1), (x + s + 1, y + s), (x, y + s)], fill=OUTLINE)
    out.mkdir(parents=True, exist_ok=True)
    back.save(out / "rubble_back.png")
    front.save(out / "rubble_front.png")


def empty_bucket(img):
    """The catapult's attack pose: the shot has left, so the grey stone in its bucket becomes the bucket's dark inside."""
    a = np.array(img).astype(int)
    rgb = a[..., :3]
    sat = rgb.max(-1) - rgb.min(-1)
    reg = np.zeros(sat.shape, bool)
    reg[28:60, 26:70] = True
    grey = reg & (a[..., 3] > 0) & (sat < 40) & (rgb.max(-1) > 70)
    a[grey, :3] = (74, 40, 18)
    return Image.fromarray(a.astype(np.uint8))


def release_hawk(img):
    """The ranger's attack pose (unit_ranger.png, 96x120): his hawk has left his raised fist. Erases the bird (its body
    and wings right of the hood, its tail right of the glove, its talons on the glove's top), drops the isolated pixels
    left behind, and closes the cut edges with the outline colour. Coordinates are the sprite's own."""
    a = np.array(img).astype(int)
    h, w = a.shape[:2]
    assert (w, h) == (96, 120), f"release_hawk expects unit_ranger.png (96x120), not {w}x{h}"
    yy, xx = np.mgrid[0:h, 0:w]
    erased = ((xx >= 50) & (yy < 58)) | ((xx >= 66) & (yy < 72)) | ((xx >= 50) & (xx < 67) & (yy <= 61))
    a[erased, 3] = 0

    def neighbours(op):
        c = np.zeros(op.shape, int)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                if dy or dx:
                    c += np.roll(np.roll(op, dy, 0), dx, 1)
        return c

    op = a[..., 3] > 0
    a[op & (neighbours(op) <= 1) & (xx >= 46) & (yy < 74), 3] = 0
    # the bird's head outline, left on the hood's edge
    for x, y in [(49, 43), (49, 44), (50, 44), (50, 54), (49, 54)]:
        a[y, x, 3] = 0
    op = a[..., 3] > 0
    light = op & (a[..., :3].sum(-1) >= 140)
    edge = np.zeros_like(op)
    for dy, dx in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
        edge |= np.roll(np.roll(light, dy, 0), dx, 1)
    fix = ~op & edge & erased
    a[fix, :3] = (30, 18, 30)
    a[fix, 3] = 255
    return Image.fromarray(a.astype(np.uint8))


def on_canvas(im):
    """An M7 sprite (the feet at its bottom centre, 2 px up) on the 160x128 canvas, the feet at (80, 126)."""
    if im.size == (160, 128):
        return im
    c = Image.new("RGBA", (160, 128), (0, 0, 0, 0))
    c.alpha_composite(im, (80 - im.width // 2, 126 - (im.height - 2)))
    return c


def pose(src: Path, dst: Path, bucket: bool, hawk: bool = False):
    im = Image.open(src).convert("RGBA")
    if hawk:
        im = release_hawk(im)
    im = on_canvas(im)
    if bucket:
        im = empty_bucket(im)
    dst.parent.mkdir(parents=True, exist_ok=True)
    im.save(dst)


def feather(d, x, y, ang, n, dark, light):
    """A loose feather `n` px long along `ang`: a pointed leaf shape, light with a dark vane down one side, outlined."""
    ca, sa = math.cos(ang), math.sin(ang)
    at = lambda u, v: (x + ca * u - sa * v, y + sa * u + ca * v)
    pts = [at(0, 0), at(n * 0.35, 2.2), at(n, 0), at(n * 0.35, -2.2)]
    d.polygon(pts, fill=light, outline=OUTLINE)
    d.line([at(1, 0), at(n - 1, 0)], fill=dark, width=1)


def hawk(src: Path, out: Path):
    """The ranger's hawk striking, talons first: the hawk's strike frame (it faces right and dives on its talons, at
    (90, 118) in the frame) placed so the talons land 44 px above the card's foot (the target's chest), three claw
    rakes under them (front), and loose feathers (back). Faces right; the battle screen mirrors it for a blow toward
    the left."""
    W, H = 110, 112
    tx, ty = W // 2, H - 44
    bird = Image.open(src).convert("RGBA")
    front = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(front)
    # the rakes: three slashes down and forward of the talons, a white core in a dark-red rim and an outline
    for k in range(3):
        x0, y0 = tx - 6 + 6 * k, ty - 4 + 2 * k
        x1, y1 = x0 + 14, y0 + 18
        d.line([(x0, y0), (x1, y1)], fill=OUTLINE, width=4)
        d.line([(x0, y0), (x1, y1)], fill=(170, 30, 36, 255), width=2)
        d.line([(x0, y0 - 1), (x1, y1 - 1)], fill=(255, 248, 236, 255), width=1)
    front.alpha_composite(bird, (tx - 90, ty - 118))
    back = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(back)
    rng = random.Random(5)
    for x, y in [(18, 44), (30, 22), (84, 30), (96, 58), (22, 76), (70, 14)]:
        feather(d, x, y, rng.uniform(0, math.tau), rng.randint(8, 11), (120, 70, 34, 255), (226, 200, 160, 255))
    out.mkdir(parents=True, exist_ok=True)
    back.save(out / "hawk_back.png")
    front.save(out / "hawk_front.png")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("rubble", help="write rubble_back.png and rubble_front.png into OUT")
    r.add_argument("out", type=Path)
    p = sub.add_parser("pose", help="write one frame as a key pose")
    p.add_argument("src", type=Path)
    p.add_argument("dst", type=Path)
    p.add_argument("--empty-bucket", action="store_true", help="the catapult's attack: its stone has left")
    p.add_argument("--release-hawk", action="store_true", help="the ranger's attack: his hawk has left his fist")
    k = sub.add_parser("hawk", help="write hawk_back.png and hawk_front.png into OUT from the hawk's strike frame SRC")
    k.add_argument("src", type=Path)
    k.add_argument("out", type=Path)
    a = ap.parse_args()
    if a.cmd == "rubble":
        rubble(a.out)
    elif a.cmd == "hawk":
        hawk(a.src, a.out)
    else:
        pose(a.src, a.dst, a.empty_bucket, a.release_hawk)


if __name__ == "__main__":
    main()
