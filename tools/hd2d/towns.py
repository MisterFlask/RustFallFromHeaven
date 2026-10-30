"""The town kit's sheets (tools/hd2d/towns.json, docs/HD2D.md §5c) into cards: cut a sheet made by
gen.py apart (art.py split) and card each object at its piece's height (art.py prop), in the order
the sheet asked for them.

  python3 tools/hd2d/towns.py SHEET_PNG SHEET [k=PIECE ...] [--grow N]

SHEET is the gen.py sheet name (`town:human`, `landmark:3`); its pieces, in reading order, are
carded as crates/ffh-play/assets/app/r3d/hd2d/art/<piece>.png. `k=PIECE` overrides the order (the
k-th object cut from the sheet is PIECE; `k=-` skips it) when the model laid a sheet out otherwise;
`--grow N` joins parts of one object up to N pixels apart (a floating crystal, a flag) into it.
Then run `art.py index`. Needs Pillow and numpy.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import art  # noqa: E402
import gen  # noqa: E402

# A town piece's palette: more colours than a prop's 32, so the lit windows, the stone's shades
# and the owner-colour band (iso.js ownerBand) each keep theirs.
COLORS = 48


def heights():
    kit = json.loads(gen.TOWNS.read_text(encoding="utf-8"))
    out = {}
    for fam, f in kit["families"].items():
        for p in f["pieces"]:
            out[f"town_{fam}_{p['name']}"] = p["height"]
    for lm in kit["landmarks"].values():
        out[lm["name"]] = lm["height"]
    return out


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    grow = 0
    if "--grow" in sys.argv:
        i = sys.argv.index("--grow")
        grow = int(sys.argv[i + 1])
        del sys.argv[i:i + 2]
    png, sheet = Path(sys.argv[1]), sys.argv[2]
    names = gen.town_sheets()[sheet][2]
    order = dict(enumerate(names, 1))
    for a in sys.argv[3:]:
        k, piece = a.split("=", 1)
        order[int(k)] = None if piece == "-" else piece
    prefix = png.stem + "_cut"
    art.split(str(png), prefix, grow=grow)
    h = heights()
    for k, piece in sorted(order.items()):
        src = png.parent / f"{prefix}_{k}.png"
        if piece is None:
            continue
        if not src.exists():
            sys.exit(f"{sheet}: object {k} ({piece}) was not cut from the sheet")
        art.prop(str(src), piece, h[piece], colors=COLORS)


if __name__ == "__main__":
    main()
