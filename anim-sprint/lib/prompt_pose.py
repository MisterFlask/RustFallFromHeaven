"""A key pose from the local model by prompt alone (ANIMATING_UNITS.md, "Impact cards and key poses"): the unit's
sprite is the only reference, the prompt names the pose, and the render is turned into a pixel-exact 160x128 frame.

    python lib/prompt_pose.py gen <sprite.png> <out_dir> <name> "<pose>"     # 2 variants: <out_dir>/<name>_1.png, _2.png
    python lib/prompt_pose.py finish <render.png> <scale> <dst.png> <sprite.png>

`gen` needs ComfyUI running and lib/klein.py (the anim-all branch: FLUX 3 Image Klein with the pose LoRA); put that
lib on PYTHONPATH. The prompt: "the character from the first image, <who>, redrawn <pose>. Same face, hair, hood,
cloak, outfit, colours, size and dark outlines as the first image. Full body game unit sprite, 16-bit pixel art, on a
flat magenta background." Write the pose toward the side the sprite looks (rule 1).

`finish` keys the magenta, shrinks the 4x render by `scale` of the sprite's size (premultiplied box filter), snaps to
the sprite's own colours, turns the magenta's purple fringe into the outline, closes the outline, and puts the lowest
row on y 126 with the boots' centre on x 80. The model draws a dynamic pose larger than the sprite and floats its feet
up: the scout's lunges needed scale 0.85-0.9 (judge it by the head and hood next to the standing sprite).

The scout's attack (pose/scout_attack.png) is `gen ... scout "lunging forward toward the lower right, front knee bent,
thrusting his short sword straight out ahead of him at arm's length, the spyglass tucked into his belt"`, first
variant, `finish ... 0.88`.
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image

TAIL = ("Same face, hair, hood, cloak, outfit, colours, size and dark outlines as the first image. Full body game unit "
        "sprite, 16-bit pixel art, on a flat magenta background.")


def gen(sprite, out_dir, name, pose, who="the unit"):
    import klein  # anim-all's lib/klein.py

    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    s = Image.open(sprite).convert("RGBA")
    c = Image.new("RGBA", (160, 128), (255, 0, 255, 255))
    c.alpha_composite(s, (80 - s.width // 2, 126 - (s.height - 2)))
    ref = out_dir / "ref.png"
    c.convert("RGB").resize((640, 512), Image.NEAREST).save(ref)
    r = klein.edit(images=[str(ref)], prompt=f"the character from the first image, {who}, redrawn {pose}. {TAIL}",
                   out=str(out_dir / f"{name}.png"), n=2)
    print(r)


def finish(src, scale, dst, sprite):
    OUTL = (32, 11, 25)
    a = np.array(Image.open(src).convert("RGB")).astype(int)
    mag = (a[..., 0] > 150) & (a[..., 2] > 150) & (a[..., 1] < 120)
    rgba = np.dstack([a, np.where(mag, 0, 255)]).astype(np.uint8)
    im = Image.fromarray(rgba, "RGBA")
    # premultiplied box shrink to 1x at the given scale (the model's figure is 4x, `scale` of the sprite's size)
    w, h = round(640 * scale / 4), round(512 * scale / 4)
    pm = np.array(im).astype(float)
    pm[..., :3] *= pm[..., 3:4] / 255
    small = np.array(Image.fromarray(pm.astype(np.uint8), "RGBA").resize((w, h), Image.BOX)).astype(float)
    al = small[..., 3]
    rgb = np.where(al[..., None] > 0, small[..., :3] * 255 / np.maximum(al[..., None], 1), 0)
    op = al >= 128
    # the sprite's own colours
    pal = np.array(sorted({p[:3] for p in Image.open(sprite).convert("RGBA").get_flattened_data() if p[3] > 0}), int)
    cols = rgb[op].astype(int)
    idx = np.argmin(((cols[:, None, :] - pal[None]) ** 2).sum(-1), 1)
    out = np.zeros((h, w, 4), np.uint8)
    out[op, :3] = pal[idx]
    out[op, 3] = 255
    # the magenta's fringe: purple edge pixels become the outline; close the outline round light pixels
    r, g, b = (out[..., k].astype(int) for k in range(3))
    purple = op & (r > 60) & (b > 60) & (g < 50) & (b > g * 2)
    out[purple, :3] = OUTL
    light = op & (out[..., :3].astype(int).sum(-1) >= 150)
    ring = np.zeros_like(op)
    for dy, dx in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
        ring |= np.roll(np.roll(light, dy, 0), dx, 1)
    ring &= ~op
    out[ring, :3] = OUTL
    out[ring, 3] = 255
    # onto the canvas: the lowest opaque row on y 126, the boots' centre (the bottom 8 rows) on x 80
    ys, xs = np.where(out[..., 3] > 0)
    bot = ys.max()
    boots = xs[ys >= bot - 8]
    cx = (boots.min() + boots.max()) / 2
    canvas = Image.new("RGBA", (160, 128), (0, 0, 0, 0))
    canvas.alpha_composite(Image.fromarray(out, "RGBA"), (round(80 - cx), 126 - bot))
    canvas.save(dst)


if __name__ == "__main__":
    if sys.argv[1] == "gen":
        gen(*sys.argv[2:6])
    elif sys.argv[1] == "finish":
        finish(sys.argv[2], float(sys.argv[3]), sys.argv[4], sys.argv[5])
    else:
        sys.exit(__doc__)
