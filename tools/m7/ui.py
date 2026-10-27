#!/usr/bin/env python3
"""Interface art for the isometric client's UI (docs/ISO.md, "The interface"): icon sheets drawn by
the M7 image model in the style of the reference screenshot's panels, cut into icons.

  python3 ui.py sheet  --name stats [--n 2] [--quality medium] [--round r1] [--dry-run]
  python3 ui.py cut    --name stats [--round r1]           # every variant: work/ui_<name>/cut/<v>/<key>.png
  python3 ui.py accept --name stats --variant 1 [--round r1] [--only k1,k2] [--skip k3]
  python3 ui.py backdrop --raw work/ui_backdrop/raw/r1_2.png   # the portrait and city-view backdrop
  then: python3 m7.py export

A sheet is 16 icons in a 4 x 4 grid, listed in prompts/ui/<name>.txt (`key: description`, row by
row). Each call gets prompts/style_ui_icons.txt, the list, and anchors/anchor_ui.png (the reference's
panels). The cut keys out the magenta, keeps each cell's pixels, trims, fits the icon in a
square canvas of each size of CUTS (centred) and re-grids it to PIXEL px blocks, the reference's
chunky pixels. `accept` stores roster/ui/ui_<key>.png (the export copies it
into the client as `ui_<key>`) and the exact prompt in prompts/assets/ui_<name>.txt.
"""
import argparse, json, shutil, sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / "lib"))
import cutout as co  # noqa: E402
from m7 import MODEL, call_api, log_call  # noqa: E402

GRID = 4
# The cuts of each icon: suffix -> (canvas, largest side inside it), art px, drawn at 1 CSS px per
# art px. Both are re-grid to PIXEL px blocks, the reference's chunky pixels: `ui_<key>` is 24
# blocks (action tiles), `ui_<key>_s` 16 (the top bar, list rows; at half size, inline in text).
CUTS = {"": (48, 44), "_s": (32, 30)}
SIZE = 48     # the review sheet's cell
PIXEL = 2
ANCHOR = ROOT / "anchors" / "anchor_ui.png"
BACKDROP = (384, 256)


def items(name):
    out = []
    for line in (ROOT / "prompts" / "ui" / f"{name}.txt").read_text(encoding="utf-8").splitlines():
        if line.strip():
            k, d = line.split(":", 1)
            out.append((k.strip(), d.strip()))
    if len(out) != GRID * GRID:
        raise SystemExit(f"{name}: {len(out)} icons listed, a sheet holds {GRID * GRID}")
    return out


def prompt_of(name):
    style = (ROOT / "prompts" / "style_ui_icons.txt").read_text(encoding="utf-8").strip()
    lines = [f"{i + 1}. {d}" for i, (_, d) in enumerate(items(name))]
    return style + "\n" + "\n".join(lines) + "\n"


def wdir(name):
    d = ROOT / "work" / f"ui_{name}"
    for s in ("raw", "cut"):
        (d / s).mkdir(parents=True, exist_ok=True)
    return d


def sheet(a):
    d = wdir(a.name)
    prompt = prompt_of(a.name)
    pf = d / f"prompt_{a.round}.txt"
    pf.write_text(prompt, encoding="utf-8")
    if a.dry_run:
        print(prompt)
        print(json.dumps({"images": [str(ANCHOR)]}))
        return
    job = {"tag": "ui/icons", "model": a.model, "size": "1024x1024"}
    from concurrent.futures import ThreadPoolExecutor
    raws = [d / "raw" / f"{a.round}_{k + 1}.png" for k in range(a.n)]
    with ThreadPoolExecutor(a.n) as ex:
        outs = list(ex.map(lambda r: call_api(job, 0, a.quality, pf, [ANCHOR], r), raws))
    for r, o in zip(raws, outs):
        log_call(d, {"round": a.round, "raw": str(r), **o})
        print(json.dumps({"raw": str(r), "ok": o.get("ok"), "usd": o.get("usd"), "error": o.get("error")}))


def cells(im):
    """The grid's cells as images with a transparent background: the magenta keyed out of the whole
    sheet, then each cell's pixels (an icon that strays a little over a cell line is cut there)."""
    keyed = co.remove_bg(im.convert("RGBA"), "#ff00ff", 48, 30, {})
    a = np.asarray(keyed).copy()
    h, w = a.shape[:2]
    out = []
    for r in range(GRID):
        for c in range(GRID):
            y0, y1 = r * h // GRID, (r + 1) * h // GRID
            x0, x1 = c * w // GRID, (c + 1) * w // GRID
            out.append(Image.fromarray(a[y0:y1, x0:x1], "RGBA"))
    return out


def icon_of(cell, size, fit):
    im = co.trim(co.clean_specks(cell, 60))
    if im.width < 4 or im.height < 4:
        return None
    s = fit / max(im.width, im.height)
    im = co.scale_to(im, w=max(1, round(im.width * s)), h=max(1, round(im.height * s)))
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    # Whole blocks from the canvas edge, so the re-grid keeps the icon centred.
    x, y = (size - im.width) // 2, (size - im.height) // 2
    canvas.alpha_composite(im, (x - x % PIXEL, y - y % PIXEL))
    return co.pixelate(canvas, PIXEL)


def review(d, rnd, cuts):
    """A sheet per variant: each icon at 1x and 2x on the interface navy, its key under it."""
    from PIL import ImageDraw
    navy = (17, 27, 52, 255)
    cw, ch = SIZE * 3 + 16, SIZE * 2 + 30
    sheets = []
    for v, icons in cuts:
        img = Image.new("RGBA", (cw * GRID, ch * GRID), navy)
        dr = ImageDraw.Draw(img)
        for i, (k, ic) in enumerate(icons):
            x, y = (i % GRID) * cw, (i // GRID) * ch
            if ic is not None:
                img.alpha_composite(ic, (x + 4, y + 4 + SIZE // 2))
                img.alpha_composite(ic.resize((SIZE * 2, SIZE * 2), Image.NEAREST), (x + SIZE + 12, y + 4))
            dr.text((x + 4, y + ch - 20), f"{v}:{k}", fill=(240, 220, 150, 255))
        sheets.append(img)
    out = Image.new("RGBA", (sum(s.width for s in sheets) + 8 * (len(sheets) - 1), sheets[0].height), (0, 0, 0, 255))
    x = 0
    for s in sheets:
        out.alpha_composite(s, (x, 0))
        x += s.width + 8
    p = d / f"review_{rnd}.png"
    out.convert("RGB").save(p)
    return p


def cut(a):
    d = wdir(a.name)
    keys = [k for k, _ in items(a.name)]
    cuts = []
    for raw in sorted((d / "raw").glob(f"{a.round}_*.png")):
        v = raw.stem.split("_")[-1]
        od = d / "cut" / f"{a.round}_{v}"
        od.mkdir(parents=True, exist_ok=True)
        icons = []
        for k, cell in zip(keys, cells(Image.open(raw))):
            for suffix, (size, fit) in CUTS.items():
                ic = icon_of(cell, size, fit)
                if ic is not None:
                    ic.save(od / f"{k}{suffix}.png")
                if suffix == "":
                    icons.append((k, ic))
        cuts.append((v, icons))
    if not cuts:
        raise SystemExit(f"no raw images for round {a.round} in {d / 'raw'}")
    print(json.dumps({"review": str(review(d, a.round, cuts))}))


def accept(a):
    d = wdir(a.name)
    src = d / "cut" / f"{a.round}_{a.variant}"
    only = set(a.only.split(",")) if a.only else None
    skip = set(a.skip.split(",")) if a.skip else set()
    dst = ROOT / "roster" / "ui"
    dst.mkdir(parents=True, exist_ok=True)
    man_p = ROOT / "roster" / "manifest.json"
    man = json.loads(man_p.read_text())
    done = []
    for k, _ in items(a.name):
        if (only and k not in only) or k in skip or not (src / f"{k}.png").exists():
            continue
        for suffix, (size, fit) in CUTS.items():
            key = f"ui_{k}{suffix}"
            shutil.copy(src / f"{k}{suffix}.png", dst / f"{key}.png")
            man[key] = {"kind": "ui_icon", "file": f"ui/{key}.png", "size": [size, size], "sheet": a.name,
                        "round": a.round, "variant": a.variant, "pixel": PIXEL, "fit": fit,
                        "prompt": f"prompts/assets/ui_{a.name}.txt", "anchors": [str(ANCHOR.relative_to(ROOT))]}
            done.append(key)
    shutil.copy(d / f"prompt_{a.round}.txt", ROOT / "prompts" / "assets" / f"ui_{a.name}.txt")
    man_p.write_text(json.dumps(man, indent=1) + "\n")
    print(json.dumps({"accepted": done}))


def backdrop(a):
    """The landscape behind the unit portrait and the city view (prompts/assets/ui_backdrop.txt,
    made with lib/imggen.py at 1536x1024 on anchors/anchor_ui.png): BACKDROP px, re-grid to PIXEL px
    blocks. The client shows crops of it at 1 CSS px per px."""
    w, h = BACKDROP
    im = Image.open(a.raw).convert("RGB").resize((w // PIXEL, h // PIXEL), Image.BOX).resize((w, h), Image.NEAREST)
    dst = ROOT / "roster" / "ui"
    dst.mkdir(parents=True, exist_ok=True)
    im.save(dst / "ui_backdrop.png", optimize=True)
    prompt = Path(a.raw).parent.parent / "prompt_r1.txt"
    if prompt.exists():
        shutil.copy(prompt, ROOT / "prompts" / "assets" / "ui_backdrop.txt")
    man_p = ROOT / "roster" / "manifest.json"
    man = json.loads(man_p.read_text())
    man["ui_backdrop"] = {"kind": "ui_backdrop", "file": "ui/ui_backdrop.png", "size": [w, h], "pixel": PIXEL,
                          "raw": Path(a.raw).name, "gen_size": "1536x1024", "prompt": "prompts/assets/ui_backdrop.txt",
                          "anchors": [str(ANCHOR.relative_to(ROOT))]}
    man_p.write_text(json.dumps(man, indent=1) + "\n")
    print(json.dumps({"backdrop": str(dst / "ui_backdrop.png"), "size": [w, h]}))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("sheet")
    p.add_argument("--name", required=True); p.add_argument("--n", type=int, default=2)
    p.add_argument("--quality", default="medium"); p.add_argument("--round", default="r1")
    p.add_argument("--model", default=MODEL); p.add_argument("--dry-run", action="store_true")
    p = sub.add_parser("cut")
    p.add_argument("--name", required=True); p.add_argument("--round", default="r1")
    p = sub.add_parser("accept")
    p.add_argument("--name", required=True); p.add_argument("--variant", required=True)
    p.add_argument("--round", default="r1"); p.add_argument("--only"); p.add_argument("--skip")
    p = sub.add_parser("backdrop")
    p.add_argument("--raw", required=True)
    a = ap.parse_args()
    {"sheet": sheet, "cut": cut, "accept": accept, "backdrop": backdrop}[a.cmd](a)


if __name__ == "__main__":
    main()
