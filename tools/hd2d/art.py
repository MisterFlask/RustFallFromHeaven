"""The HD-2D mode's own art (docs/HD2D.md §10): pixel-art ground layers and prop cards drawn by a
local image model (FLUX 3 Image Klein in ComfyUI on the owner's PC) and brought to one contract
here, the way tools/m7 does it for the isometric sprites:

  ground SRC NAME   a top-down texture -> a seamless 256 x 256 ground layer (128 art px a side, each
                    two texels, its palette cut), its height (from its value) in alpha:
                    crates/ffh-play/assets/app/r3d/hd2d/art/NAME.png
  prop SRC NAME H [DEEPEN]
                    an object on a plain white background -> a card H art px tall (cut out, palette
                    cut, a dark one-pixel outline), its foot at the bottom centre: art/NAME.png;
                    DEEPEN (a gamma, e.g. 1.3) darkens its middle values (the model paints foliage
                    a bright lime; the look target's woods are deep greens under bright tips)
  index             art/index.js: every card's size and opaque box, every ground layer

Needs Pillow and numpy (ComfyUI's bundled Python has both):
  C:/ComfyUI/python_embeded/python.exe tools/hd2d/art.py ground gen/gnd_grass.png gnd_grass
"""
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "crates" / "ffh-play" / "assets" / "app" / "r3d" / "hd2d" / "art"
LAYER = 256  # texels a side of a ground layer (r3d/materials.js MAT_SIZE)


def smooth_rank(shape, sigma, seed):
    """Smooth noise ranked to a uniform 0..1 (clustered choices, not salt and pepper)."""
    r = np.random.default_rng(seed).random(shape)
    # A separable box blur, a few times: close enough to a gaussian, no scipy needed.
    k = max(1, int(sigma))
    for _ in range(3):
        for ax in (0, 1):
            r = sum(np.roll(r, s, ax) for s in range(-k, k + 1)) / (2 * k + 1)
    return (r.argsort(axis=None).argsort().reshape(shape) + 0.5) / r.size


def quantize(rgb, colors):
    """Cut an RGB array's palette to `colors` (median cut, no dithering)."""
    im = Image.fromarray(rgb.astype(np.uint8), "RGB").quantize(colors, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    return np.asarray(im.convert("RGB"))


def ground(src, name, art=128, band=32, colors=40, seed=7):
    im = Image.open(src).convert("RGB")
    s = min(im.size)
    im = im.crop(((im.width - s) // 2, (im.height - s) // 2, (im.width + s) // 2, (im.height + s) // 2))
    n = art + band
    small = np.asarray(im.resize((n, n), Image.Resampling.BOX)).astype(np.float32)
    small = quantize(small, colors)
    # Seamless: near the tile's west (north) edge a texel is taken, more often the nearer the edge,
    # from one tile's width further east (south) in the source -- what lies just past the opposite
    # edge -- so the tile's last column meets its first as the source's neighbours meet. Clustered
    # choices (a dithered cross-fade): no ghosting, no seam.
    ys, xs = np.mgrid[0:art, 0:art]
    wx = np.clip(1.0 - xs / band, 0.0, 1.0)
    wy = np.clip(1.0 - ys / band, 0.0, 1.0)
    bx = smooth_rank((art, art), 1.5, seed) < wx
    by = smooth_rank((art, art), 1.5, seed + 1) < wy
    sx = xs + np.where(bx, art, 0)
    sy = ys + np.where(by, art, 0)
    tile = small[sy, sx]
    # The height: the texel's value, softened a little, kept off zero (the shader reads it as relief
    # and as which of two layers stands taller).
    lum = tile @ np.array([0.2126, 0.7152, 0.0722])
    soft = lum.copy()
    for ax in (0, 1):
        soft = (np.roll(soft, 1, ax) + soft * 2 + np.roll(soft, -1, ax)) / 4
    h = (soft - soft.min()) / max(1e-6, soft.max() - soft.min())
    alpha = (0.3 + 0.7 * h) * 255
    rgba = np.dstack([tile, alpha]).astype(np.uint8)
    k = LAYER // art
    rgba = np.repeat(np.repeat(rgba, k, 0), k, 1)
    ART.mkdir(parents=True, exist_ok=True)
    Image.fromarray(rgba, "RGBA").save(ART / f"{name}.png", optimize=True)
    print(f"{name}: ground {LAYER}x{LAYER}, {art} art px a side, {colors} colours")


def cutout(rgb, thresh=232):
    """The object on a plain light background: the background is the near-white, low-saturation
    region connected to the image's border (a flood fill, so white inside the object stays)."""
    h, w = rgb.shape[:2]
    mx = rgb.max(2).astype(int)
    mn = rgb.min(2).astype(int)
    bgish = (mn >= thresh) | ((mn >= thresh - 30) & (mx - mn < 10))
    bg = np.zeros((h, w), bool)
    stack = [(y, x) for y in (0, h - 1) for x in range(w)] + [(y, x) for x in (0, w - 1) for y in range(h)]
    while stack:
        y, x = stack.pop()
        if bg[y, x] or not bgish[y, x]:
            continue
        bg[y, x] = True
        if y > 0:
            stack.append((y - 1, x))
        if y < h - 1:
            stack.append((y + 1, x))
        if x > 0:
            stack.append((y, x - 1))
        if x < w - 1:
            stack.append((y, x + 1))
    return ~bg


def prop(src, name, height, colors=32, outline=True, deepen=1.0):
    rgb = np.asarray(Image.open(src).convert("RGB"))
    if deepen != 1.0:
        # Only the object's colours (the white background must stay white for the cut-out).
        bg = rgb.min(2) >= 232
        deep = (255 * (rgb / 255.0) ** deepen).astype(np.uint8)
        rgb = np.where(bg[..., None], rgb, deep)
    # Work at half size: the flood fill is pure Python.
    if max(rgb.shape[:2]) > 640:
        rgb = np.asarray(Image.fromarray(rgb).resize((rgb.shape[1] // 2, rgb.shape[0] // 2), Image.Resampling.BOX))
    mask = cutout(rgb)
    ys, xs = np.nonzero(mask)
    if not len(ys):
        sys.exit(f"{name}: no object found")
    y0, y1, x0, x1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
    rgb, mask = rgb[y0:y1, x0:x1], mask[y0:y1, x0:x1]
    k = height / rgb.shape[0]
    W = max(1, round(rgb.shape[1] * k))
    H = height
    # Downsample colour and coverage together (premultiplied box filter), then a hard alpha.
    a = mask.astype(np.float32)
    pm = Image.fromarray((rgb * a[..., None]).astype(np.uint8))
    cov = np.asarray(Image.fromarray((a * 255).astype(np.uint8)).resize((W, H), Image.Resampling.BOX)).astype(np.float32) / 255
    col = np.asarray(pm.resize((W, H), Image.Resampling.BOX)).astype(np.float32) / np.maximum(cov, 1e-3)[..., None]
    solid = cov >= 0.5
    col = np.clip(col, 0, 255)
    pal = quantize(np.where(solid[..., None], col, 0), colors)
    out = np.zeros((H + 2, W + 2, 4), np.uint8)
    out[1:-1, 1:-1, :3] = pal
    out[1:-1, 1:-1, 3] = np.where(solid, 255, 0)
    if outline:
        # A dark one-pixel outline round the silhouette (the isometric sprites have one: it keeps a
        # card readable on any ground), in a deep version of the colour inside.
        op = out[..., 3] > 0
        ring = np.zeros_like(op)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            ring |= np.roll(np.roll(op, dy, 0), dx, 1)
        ring &= ~op
        near = np.zeros(out.shape[:2] + (3,), np.float32)
        cnt = np.zeros(out.shape[:2], np.float32)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            m = np.roll(np.roll(op, dy, 0), dx, 1)
            near += np.roll(np.roll(out[..., :3].astype(np.float32), dy, 0), dx, 1) * m[..., None]
            cnt += m
        dark = near / np.maximum(cnt, 1)[..., None] * 0.28 + 6
        out[ring, :3] = dark[ring].astype(np.uint8)
        out[ring, 3] = 255
    ART.mkdir(parents=True, exist_ok=True)
    Image.fromarray(out, "RGBA").save(ART / f"{name}.png", optimize=True)
    print(f"{name}: card {out.shape[1]}x{out.shape[0]}, {colors} colours")


def index():
    cards, grounds = {}, []
    for p in sorted(ART.glob("*.png")):
        im = Image.open(p)
        if p.stem.startswith("gnd_"):
            grounds.append(p.stem)
            continue
        a = np.asarray(im.convert("RGBA"))[..., 3]
        ys, xs = np.nonzero(a)
        cards[p.stem] = {"w": im.width, "h": im.height, "box": [int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1]}
    lines = [
        "// The HD-2D mode's own art (docs/HD2D.md): ground layers and prop cards drawn by a local image",
        "// model and brought to one contract by tools/hd2d/art.py, which writes this index; do not edit.",
        f"export const GROUNDS = {json.dumps(grounds)};",
        "export const CARDS = {",
    ]
    for k, v in cards.items():
        lines.append(f"  {k}: {{ w: {v['w']}, h: {v['h']}, box: {json.dumps(v['box']).replace(',', ', ')} }},")
    lines.append("};")
    (ART / "index.js").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    print(f"index: {len(grounds)} ground layers, {len(cards)} cards")


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "ground":
        ground(sys.argv[2], sys.argv[3])
    elif cmd == "prop":
        prop(sys.argv[2], sys.argv[3], int(sys.argv[4]), deepen=float(sys.argv[5]) if len(sys.argv) > 5 else 1.0)
    elif cmd == "index":
        index()
    else:
        sys.exit(__doc__)
