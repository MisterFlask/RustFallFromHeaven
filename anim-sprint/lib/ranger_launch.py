"""The ranger's launch pose (crates/ffh-play/assets/app/pose/ranger_attack.png): his arm raised and flung outward,
the hand open, as though he has just launched his hawk. A redrawn limb on the real sprite, in three steps
(ANIMATING_UNITS.md, "Impact cards and key poses"):

1. `guide OUT`: on the hawk-free sprite (`impact_cards.py pose ... --release-hawk`), the old bent forearm is cut away and
   a rough outstretched arm painted in (green sleeve, leather forearm, a hand); written at 4x on magenta as
   `OUT/init.png`, with the original sprite the same way as `OUT/ref.png`.
2. The local model (lib/klein.py on the anim-all branch: FLUX 3 Image Klein with the pose LoRA, ComfyUI running) redraws
   it image-to-image: `klein.edit(images=["ref.png"], init="init.png", denoise=0.65, n=2, prompt=...)`, the prompt
   "the character from the first image, the ranger, redrawn with his left arm raised high and stretched straight out
   toward the upper right, hand open with the fingers pointing outward, as though he has just launched a hawk from his
   fist; no bird anywhere. Same face, hair, hood, outfit, colours, size and dark outlines as the first image. Full body
   game unit sprite, 16-bit pixel art, on a flat magenta background." Pick a variant by eye (the shipped one was the
   first of seed 2 at 0.65; 0.5 kept the arm shorter).
3. `finish RENDER DST`: shrinks the render by its 4x4 blocks' most common colour, keys the magenta, snaps to the
   sprite's own colours, takes only the arm's region from it (the rest stays the sprite's pixels), turns the magenta's
   purple fringe into the outline and closes the outline round the arm.
"""
import sys
from collections import Counter
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
MAG = (255, 0, 255, 255)
sys.path.insert(0, str(Path(__file__).resolve().parent))
from impact_cards import on_canvas, release_hawk  # noqa: E402


def freed_sprite():
    """The ranger with his hawk gone from his fist, on the 160x128 canvas."""
    return on_canvas(release_hawk(Image.open(ROOT / "crates/ffh-play/assets/app/iso/unit_ranger.png").convert("RGBA")))


def guide(OUT):
    OUT = Path(OUT)
    OUT.mkdir(parents=True, exist_ok=True)
    orig = Image.open(ROOT / "crates/ffh-play/assets/app/iso/unit_ranger.png").convert("RGBA")
    freed = freed_sprite().crop((32, 8, 128, 128))
    glove = (170, 90, 24)
    sleeve = (61, 94, 36)
    # take the old bent forearm and fist away (right of the collar, below the shoulder)
    a = freed.copy()
    px = a.load()
    for y in range(56, 76):
        for x in range(53, 70):
            px[x, y] = (0, 0, 0, 0)
    d = ImageDraw.Draw(a)
    OUTL = (30, 18, 30, 255)
    # upper arm from the shoulder up and out to the elbow, the forearm on to the hand, fingers pointing out
    pts = [(51, 64), (61, 57), (71, 47)]
    for w, c in [(7, OUTL), (5, sleeve + (255,))]:
        d.line(pts[:2], fill=c, width=w)
    for w, c in [(7, OUTL), (5, glove + (255,))]:
        d.line(pts[1:], fill=c, width=w)
    d.ellipse((69, 42, 77, 50), fill=glove + (255,), outline=OUTL)
    d.line([(75, 44), (81, 39)], fill=OUTL, width=3)
    d.line([(75, 44), (80, 40)], fill=glove + (255,), width=1)


    def mag4(im):
        c = Image.new("RGBA", (160, 128), MAG)
        c.alpha_composite(im, (32, 8))
        return c.convert("RGB").resize((640, 512), Image.NEAREST)


    mag4(orig).save(OUT / "ref.png")
    mag4(a).save(OUT / "init.png")
    a.save(OUT / "guide_1x.png")


def finish(src, dst):
    gen = np.array(Image.open(src).convert("RGB")).astype(int)
    H, W = 128, 160
    small = np.zeros((H, W, 4), np.uint8)
    for y in range(H):
        for x in range(W):
            blk = gen[y * 4:(y + 1) * 4, x * 4:(x + 1) * 4].reshape(-1, 3)
            # magenta-ish: red and blue high, green low
            mag = (blk[:, 0] > 170) & (blk[:, 2] > 170) & (blk[:, 1] < 110)
            if mag.sum() >= 8:
                continue
            keep = blk[~mag]
            c = Counter(map(tuple, (keep // 8)))
            q = np.array(c.most_common(1)[0][0])
            near = keep[np.all(keep // 8 == q, axis=1)].mean(0)
            small[y, x, :3] = near.round().astype(np.uint8)
            small[y, x, 3] = 255
    orig = Image.open(ROOT / "crates/ffh-play/assets/app/iso/unit_ranger.png").convert("RGBA")
    pal = np.array(sorted({p[:3] for p in orig.get_flattened_data() if p[3] > 0}), int)
    op = small[..., 3] > 0
    cols = small[op, :3].astype(int)
    d = ((cols[:, None, :] - pal[None, :, :]) ** 2).sum(-1)
    small[op, :3] = pal[d.argmin(1)].astype(np.uint8)
    base = np.array(freed_sprite())
    # the arm's region on the 160x128 canvas (the sprite sits at 32, 8): right of the hood and above the belt
    yy, xx = np.mgrid[0:H, 0:W]
    region = (xx >= 32 + 52) & (yy >= 8 + 14) & (yy < 8 + 76)
    out = base.copy()
    out[region] = small[region]
    # the magenta's fringe: purple pixels on the arm's edge become the outline, then close the outline round the arm
    OUTL = (32, 11, 25)
    rgb = out[..., :3].astype(int)
    opq = out[..., 3] > 0
    purple = region & opq & (rgb[..., 0] > 60) & (rgb[..., 2] > 60) & (rgb[..., 1] < 50)
    out[purple, :3] = OUTL
    opq = out[..., 3] > 0
    light = opq & (out[..., :3].astype(int).sum(-1) >= 150)
    ring = np.zeros_like(opq)
    for dy, dx in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
        ring |= np.roll(np.roll(light, dy, 0), dx, 1)
    ring &= region & ~opq
    out[ring, :3] = OUTL
    out[ring, 3] = 255
    Image.fromarray(out).save(dst)
    prev = Image.new("RGBA", (W, H), (70, 80, 60, 255))
    prev.alpha_composite(Image.fromarray(out))
    prev.resize((W * 4, H * 4), Image.NEAREST).save(Path(dst).with_name("preview_x4.png"))


if __name__ == "__main__":
    if sys.argv[1] == "guide":
        guide(sys.argv[2])
    elif sys.argv[1] == "finish":
        finish(sys.argv[2], sys.argv[3])
    else:
        sys.exit(__doc__)
