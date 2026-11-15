"""Import a 3 x 3 sheet of bespoke HD-2D map improvements.

  python tools/hd2d/landmarks.py SHEET.png SHEET_NAME

The prompts, order and art-pixel heights live in landmarks.json. Cut by grid cell,
not connected component: a statue's loose stones and seven separate pines belong
to the same sprite. Reuses the town/prop pipeline's hard alpha, palette and rim.
Run art.py index after importing all sheets.
"""
import json
import sys
import tempfile
from pathlib import Path

from PIL import Image
import numpy as np

import art


def remove_specks(im):
    """Drop detached generation specks or a neighbouring sprite's tiny overhang.
    Keep substantial disconnected parts (seven trees, a statue's plinths).
    """
    rgba = np.array(im)
    mask = rgba[..., 3] >= 128
    seen = np.zeros(mask.shape, dtype=bool)
    parts = []
    h, w = mask.shape
    for y, x in zip(*np.nonzero(mask)):
        if seen[y, x]:
            continue
        stack, part = [(y, x)], []
        seen[y, x] = True
        while stack:
            yy, xx = stack.pop()
            part.append((yy, xx))
            for ny, nx in ((yy-1, xx), (yy+1, xx), (yy, xx-1), (yy, xx+1)):
                if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not seen[ny, nx]:
                    seen[ny, nx] = True
                    stack.append((ny, nx))
        parts.append(part)
    threshold = max(32, max(map(len, parts), default=0) // 100)
    for part in parts:
        if len(part) < threshold:
            ys, xs = zip(*part)
            rgba[ys, xs, 3] = 0
    return Image.fromarray(rgba)


def main():
    src, name = sys.argv[1:]
    manifest_path = Path(__file__).with_name('resources.json') if name.startswith('resources_') else Path(__file__).with_suffix('.json')
    manifest = json.loads(manifest_path.read_text())
    sheet = next(s for s in manifest['sheets'] if s['name'] == name)
    im = Image.open(src).convert('RGBA')
    with tempfile.TemporaryDirectory() as tmp:
        for i, piece in enumerate(sheet['pieces']):
            col, row = i % 3, i // 3
            box = (round(col * im.width / 3), round(row * im.height / 3),
                   round((col + 1) * im.width / 3), round((row + 1) * im.height / 3))
            crop = remove_specks(im.crop(box))
            # Transparent padding keeps the outline even at a sheet's outer edge.
            cell = Image.new('RGBA', (crop.width + 16, crop.height + 16))
            cell.paste(crop, (8, 8))
            path = Path(tmp) / 'cell.png'
            cell.save(path)
            art.prop(path, piece['key'], piece['height'], colors=48)


if __name__ == '__main__':
    main()
