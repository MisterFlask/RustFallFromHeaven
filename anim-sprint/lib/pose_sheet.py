"""Attack and hit key poses for many units in ONE call to the OpenAI image model (the experiment of 5 November 2026:
is a sprite sheet of many units cheaper per pose than the local per-pose calls, at the same quality?).

    python lib/pose_sheet.py batch [--from G] [--limit N]       # every unit, 8 a sheet, 4 calls at once -> work/pose_sheets/out
    python lib/pose_sheet.py make NAME unit_a,unit_b,...        # up to 8 units: template + prompt in work/pose_sheets/NAME/
    python lib/pose_sheet.py call NAME [--model M] [--quality Q]
    python lib/pose_sheet.py cut NAME                           # slice the answer, finish each pose like prompt_pose.py
    python lib/pose_sheet.py review g000,g001 OUT.png           # standing | attack | hit, for review by eye

The template is 1536x1024: 4 rows of 6 cells (256x256), two units a row, each as [standing | attack | hit]. The
standing cell holds the unit's real sprite at 2x (less for a big one, so it fits) on magenta, feet 14 px above the
cell's bottom; the other cells are
empty magenta for the model to fill, so each unit's reference sits beside its own cells and sets their scale. The
prompts per unit are pose_prompts/chunk_*.prompts.json (pose_prompts/RULES.md).
"""
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
import prompt_pose as pp  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
ISO = ROOT / "crates" / "ffh-play" / "assets" / "app" / "iso"
WORK = ROOT / "anim-sprint" / "work" / "pose_sheets"
PROMPTS = ROOT / "anim-sprint" / "pose_prompts"
CELL, COLS, ROWS, K = 256, 6, 4, 2
MAG = (255, 0, 255, 255)


def prompts():
    out = {}
    for f in sorted(PROMPTS.glob("chunk_*.prompts.json")):
        for p in json.loads(f.read_text(encoding="utf-8")):
            out[p["name"]] = p
    roster = {u["name"]: u for u in json.loads((PROMPTS / "roster.json").read_text(encoding="utf-8"))}
    return out, roster


def cell_xy(i, which):
    """Unit i (0-7), which 0 standing / 1 attack / 2 hit: the cell's top-left."""
    r, half = i // 2, i % 2
    return (half * 3 + which) * CELL, r * CELL


def scale_for(sprite):
    """The template's scale for a unit: 2x, or less for a big sprite so it fits its 256 cell with a margin."""
    w, h = Image.open(sprite).size
    return min(2.0, (CELL - 20) / h, (CELL - 20) / w)


def make(name, units):
    P, R = prompts()
    d = WORK / name
    d.mkdir(parents=True, exist_ok=True)
    t = Image.new("RGBA", (COLS * CELL, ROWS * CELL), MAG)
    lines, meta = [], []
    for i, u in enumerate(units):
        s = Image.open(ISO / f"{u}.png").convert("RGBA")
        k = scale_for(ISO / f"{u}.png")
        s2 = s.resize((round(s.width * k), round(s.height * k)), Image.NEAREST)
        x, y = cell_xy(i, 0)
        t.alpha_composite(s2, (x + CELL // 2 - s2.width // 2, y + CELL - 14 - s2.height))
        p, f = P[u], R[u]["facing"]
        side = "lower left" if f == "lower-left" else "lower right"
        row, col0 = i // 2 + 1, (i % 2) * 3 + 1
        lines.append(f"Row {row}, cells {col0}-{col0 + 2}: {p['who']}, facing the {side}. "
                     f"Cell {col0 + 1} (attack): {p['attack']}. Cell {col0 + 2} (hit): {p['hit']}.")
        meta.append({"name": u, "k": k})
    t.convert("RGB").save(d / "template.png")
    prompt = (
        "The attached image is a sprite-sheet template from my isometric fantasy strategy game: a grid of 6 columns by "
        "4 rows of equal square cells on a flat magenta background (no visible grid lines). Each row holds two units, "
        "three cells each: in the first cell of each group of three is the finished unit standing at rest, drawn in "
        "bright saturated HD pixel art with soft chunky pixels, dark coloured outlines and simple flat shading lit from "
        "the upper left. Fill the two empty cells to the right of each standing unit with that same unit: the next cell "
        "shows it at the instant of its attack, the cell after it at the instant it is struck, as described below. Every "
        "figure must be the SAME character as the standing one in its group: same size, same face, same outfit, colours "
        "and equipment, same pixel size and outline style, feet on the same ground line as the standing figure, the whole "
        "figure inside its own cell. Leave the standing figures exactly as they are. "
        "An attack is frozen at the instant of contact, never before it: for a melee attack (a weapon, a fist, claws, "
        "a bite, horns, a charge) the unit is caught mid-lunge, its body thrown forward toward the target with the front "
        "foot planted, the weapon or limb fully extended at the end of its swing or thrust, the blow landing; never with "
        "the weapon raised or drawn back about to strike, even where a description below sounds like a wind-up. A shot or "
        "a spell is frozen as it is loosed or cast, the arm or staff thrust toward the target. Columns are numbered 1-6 "
        "from the "
        "left, rows 1-4 from the top.\n" + "\n".join(lines) + "\n"
        "Each cell holds exactly one figure. No grid lines, frames, numbers, text, ground, shadows, motion lines or "
        "effects outside the figures. Background: flat solid pure magenta #FF00FF filling the whole image.")
    (d / "prompt.txt").write_text(prompt, encoding="utf-8")
    (d / "units.json").write_text(json.dumps(meta), encoding="utf-8")
    return d


def call(name, model="gpt-image-2.6-sunburst", quality="medium"):
    import gen
    d = WORK / name
    r = gen.edit(tag="anim/sheets", images=[d / "template.png"], prompt=(d / "prompt.txt").read_text(encoding="utf-8"),
                 out=d / "answer.png", model=model, quality=quality, size="1536x1024", n=1)
    out = {k: r.get(k) for k in ("ok", "usd", "latency", "error")}
    print(name, json.dumps(out), flush=True)
    return out


def drop_fragments(rgb):
    """A cell's crop may catch the edge of a neighbour's figure, or a seam the model drew between cells: drop the pieces
    that reach within 12 px of the crop's edge and are small next to the largest piece (under 30% of it), and specks
    under 0.4% of it anywhere; everything else (sparks, a thrown item) stays."""
    from scipy import ndimage
    a = np.array(rgb).astype(int)
    fg = ~((a[..., 0] > 150) & (a[..., 2] > 150) & (a[..., 1] < 120))
    lab, n = ndimage.label(fg, structure=np.ones((3, 3)))
    if n <= 1:
        return rgb
    sizes = ndimage.sum(fg, lab, range(1, n + 1))
    big = sizes.max()
    E = 12  # the cell's rim: a neighbour's sliver or a seam the model drew lies within it
    rim = np.zeros(fg.shape, bool)
    rim[:E], rim[-E:], rim[:, :E], rim[:, -E:] = True, True, True, True
    out = np.array(rgb)
    for j in range(1, n + 1):
        piece = lab == j
        near_rim = piece[rim].any()
        if (near_rim and sizes[j - 1] < 0.3 * big) or sizes[j - 1] < 0.004 * big:
            out[piece] = (255, 0, 255)
    return Image.fromarray(out)


def cut(name, dest=None):
    d = WORK / name
    meta = json.loads((d / "units.json").read_text(encoding="utf-8"))
    ans = Image.open(d / "answer.png")
    if ans.mode == "RGBA":
        # the model may answer with real transparency (and a haze of faint magenta): below half alpha is background
        a = np.array(ans)
        rgb = np.where(a[..., 3:4] >= 128, a[..., :3], np.array([255, 0, 255], np.uint8))
        ans = Image.fromarray(rgb.astype(np.uint8), "RGB")
    ans = ans.convert("RGB")
    sx, sy = ans.width / (COLS * CELL), ans.height / (ROWS * CELL)
    for i, m in enumerate(meta):
        u, k = m["name"], m["k"]
        short = u.removeprefix("unit_")
        for w, pose in ((1, "attack"), (2, "hit")):
            x, y = cell_xy(i, w)
            c = drop_fragments(ans.crop((round(x * sx), round(y * sy), round((x + CELL) * sx), round((y + CELL) * sy))))
            # the template drew the unit at k x; finish expects a 4x render (its scale comes from the figure's area)
            raw = d / f"{short}_{pose}_raw.png"
            c.resize((round(c.width * 4 / k), round(c.height * 4 / k)), Image.NEAREST).save(raw)
            dst = d / f"{short}_{pose}.png"
            pp.finish(raw, "auto", dst, ISO / f"{u}.png")
            if dest:
                Path(dest).mkdir(parents=True, exist_ok=True)
                Image.open(dst).save(Path(dest) / f"{short}_{pose}.png")
    return len(meta)


def review(names, path):
    """standing | attack | hit for every unit of the sheets `names`, 2x, two units a row."""
    from PIL import ImageDraw
    rows = []
    for n in names:
        for m in json.loads((WORK / n / "units.json").read_text(encoding="utf-8")):
            rows.append((n, m["name"]))
    cw, ch = 200, 150
    per = 2
    img = Image.new("RGBA", (cw * 3 * 2 * per, ch * 2 * ((len(rows) + per - 1) // per)), (60, 70, 52, 255))
    dr = ImageDraw.Draw(img)
    for j, (n, u) in enumerate(rows):
        short = u.removeprefix("unit_")
        s = Image.open(ISO / f"{u}.png").convert("RGBA")
        W, H = pp.canvas_for(ISO / f"{u}.png")
        st = Image.new("RGBA", (W, H))
        st.alpha_composite(s, (W // 2 - s.width // 2, H - 2 - (s.height - 2)))
        tiles = [st] + [Image.open(WORK / n / f"{short}_{p}.png") for p in ("attack", "hit")]
        x0, y0 = (j % per) * cw * 3 * 2, (j // per) * ch * 2
        for c, t in enumerate(tiles):
            t = t.resize((t.width * 2, t.height * 2), Image.NEAREST)
            img.alpha_composite(t.crop((0, 0, min(t.width, cw * 2), min(t.height, ch * 2))), (x0 + c * cw * 2, y0))
        dr.text((x0 + 4, y0 + 4), f"{n}: {short}", fill=(255, 255, 255, 255))
    img.save(path)
    return path


def batch(start=0, limit=None, workers=4):
    """Every unit with prompts, in roster order, 8 a sheet: make, call (in parallel), cut into work/pose_sheets/out."""
    from concurrent.futures import ThreadPoolExecutor
    P, R = prompts()
    names = [u for u in R if u in P and (ISO / f"{u}.png").exists()]
    groups = [names[i:i + 8] for i in range(0, len(names), 8)][start:]
    if limit:
        groups = groups[:limit]
    todo = []
    for gi, g in enumerate(groups, start):
        name = f"g{gi:03d}"
        if not (WORK / name / "answer.png").exists():
            make(name, g)
            todo.append(name)
    with ThreadPoolExecutor(workers) as ex:
        res = list(ex.map(call, todo))
    for gi in range(start, start + len(groups)):
        name = f"g{gi:03d}"
        if (WORK / name / "answer.png").exists():
            cut(name, WORK / "out")
    spent = sum(r["usd"] or 0 for r in res)
    print(f"{len(todo)} calls, ${spent:.2f}; {sum(1 for r in res if not r['ok'])} failed", flush=True)


if __name__ == "__main__":
    a = sys.argv[1:]
    opt = lambda k, dflt=None: a[a.index(k) + 1] if k in a else dflt
    if a[0] == "make":
        print(make(a[1], a[2].split(",")))
    elif a[0] == "call":
        call(a[1], opt("--model", "gpt-image-2.6-sunburst"), opt("--quality", "medium"))
    elif a[0] == "cut":
        print("cut", cut(a[1]), "units")
    elif a[0] == "batch":
        batch(int(opt("--from", 0)), int(opt("--limit")) if opt("--limit") else None)
    elif a[0] == "review":
        print(review(a[1].split(","), a[2]))
    else:
        sys.exit(__doc__)
