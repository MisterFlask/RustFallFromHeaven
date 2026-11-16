"""Import the original painterly terrain sample (see docs/HD2D-TERRAIN-PREVIEW.md).

  python3 tools/hd2d/painterly.py TREES.png MEADOW.png SCATTER.png

The transparent sheet contains three broadleaf trees above three conifers. Keep
the source images: the temporary cuts can always be recreated. Requires the
existing art pipeline's Pillow and numpy dependencies.
"""
import sys
import tempfile
from pathlib import Path

from PIL import Image

import art


def cards(source, families, height):
    with tempfile.TemporaryDirectory(prefix="ffh-canopies-") as tmp:
        sheet = Path(tmp) / "trees.png"
        sheet.write_bytes(Path(source).read_bytes())
        art.split(sheet, "tree", grow=2, preserve_rgba=True)
        cuts = list(Path(tmp).glob("tree_*.png"))
        if len(cuts) != 6:
            raise ValueError(f"expected six separate objects, found {len(cuts)}")
        for n in range(6):
            im = Image.open(Path(tmp) / f"tree_{n + 1}.png").convert("RGBA")
            im = im.crop(im.getbbox())
            # Fine painted leaves without the pixel-art pipeline's palette cut or
            # heavy outline. A transparent texel around the card protects its mips.
            im = im.resize((round(im.width * height / im.height), height), Image.Resampling.LANCZOS)
            card = Image.new("RGBA", (im.width + 2, im.height + 2))
            card.paste(im, (1, 1))
            family = families[0 if n < 3 else 1]
            card.save(art.ART / f"{family}_{n % 3 + 1}.png", optimize=True)


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    cards(sys.argv[1], ("tree_broadleaf", "tree_fir"), 144)
    cards(sys.argv[3], ("grass_meadow", "flowers_meadow"), 48)
    # Native 256-texel detail; retain the existing seamless edge treatment and
    # height-in-alpha material contract, with more tones than the pixel-art turf.
    art.ground(sys.argv[2], "gnd_grass", art=256, band=64, colors=128)
    art.index()
