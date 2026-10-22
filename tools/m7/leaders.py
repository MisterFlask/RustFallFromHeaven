#!/usr/bin/env python3
"""Leader portraits for the client (docs/ISO.md, "The interface"): one bust per leader, painted by
the M7 image model in the style of the reference's unit portrait.

  python3 leaders.py gen    --keys sabathiel,capria [--n 3] [--round r1] [--quality medium] [--jobs 6] [--extra "..."] [--dry-run]
  python3 leaders.py cut    --keys sabathiel,capria [--round r1] [--art 80]
  python3 leaders.py review --keys sabathiel,capria [--round r1] [--out work/leaders/review.png]
  python3 leaders.py accept --key sabathiel --variant 2 [--round r1]
  then: python3 m7.py export

Each leader's description is prompts/leaders/<key>.txt (written from its Civilopedia entry), after
prompts/style_leader.txt; the anchors are anchors/anchor_ui.png (the reference's panels) and
anchors/anchor_portrait.png (its worker portrait, enlarged 3x). One call makes N variants at
1024x1024. The cut keeps the whole square, reduces it to ART x ART art px with a box filter and
re-grids it to PIXEL px blocks (the interface's 2 px grid). `accept` stores
roster/leaders/leader_<key>.png and the exact prompt in prompts/assets/leader_<key>.txt; the
export copies it into the client. `--keys all` is every prompts/leaders/*.txt. The budget tag is
`ui/leaders`.
"""
import argparse, json, shutil, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent
ANCHORS = [ROOT / "anchors" / "anchor_ui.png", ROOT / "anchors" / "anchor_portrait.png"]
MODEL = "gpt-image-2.5-sunburst"
TAG = "ui/leaders"
ART = 80      # art px a side: a Fire Emblem-sized bust
PIXEL = 2     # CSS px per art px (the interface's grid)


def keys_of(arg):
    if arg == "all":
        return sorted(p.stem for p in (ROOT / "prompts" / "leaders").glob("*.txt"))
    return [k.strip() for k in arg.split(",") if k.strip()]


def wdir(key):
    d = ROOT / "work" / "leaders" / key
    for s in ("raw", "cut"):
        (d / s).mkdir(parents=True, exist_ok=True)
    return d


def prompt_of(key, extra=None):
    style = (ROOT / "prompts" / "style_leader.txt").read_text(encoding="utf-8").strip()
    desc = (ROOT / "prompts" / "leaders" / f"{key}.txt").read_text(encoding="utf-8").strip()
    return "\n".join(x for x in (style, desc, extra) if x) + "\n"


def rel(p):
    return str(Path(p).relative_to(ROOT)).replace("\\", "/")


def gen_one(key, a):
    d = wdir(key)
    pf = d / f"prompt_{a.round}.txt"
    pf.write_text(prompt_of(key, a.extra), encoding="utf-8")
    images = list(ANCHORS)
    if a.revise:
        rnd, v = a.revise.split(":")
        images.append(d / "raw" / f"{rnd}_{v}.png")
    out = d / "raw" / f"{a.round}.png"
    cmd = [sys.executable, str(ROOT / "lib" / "imggen.py"), "edit", "--tag", TAG, "--model", a.model, "--quality", a.quality,
           "--size", "1024x1024", "--n", str(a.n), "--prompt-file", rel(pf), "--out", rel(out)]
    for im in images:
        cmd += ["--image", rel(im)]
    if a.dry_run:
        return {"key": key, "cmd": " ".join(cmd)}
    p = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT)
    try:
        res = json.loads(p.stdout.strip().splitlines()[-1])
    except Exception:
        res = {"ok": False, "error": (p.stdout + p.stderr)[-600:]}
    if a.n == 1 and out.exists():
        out.rename(d / "raw" / f"{a.round}_1.png")
    with open(d / "calls.jsonl", "a", encoding="utf-8") as f:
        f.write(json.dumps({"round": a.round, "images": [rel(i) for i in images], **res}) + "\n")
    return {"key": key, "ok": res.get("ok"), "usd": res.get("usd"), "error": res.get("error")}


def gen(a):
    keys = keys_of(a.keys)
    with ThreadPoolExecutor(max(1, min(a.jobs, len(keys)))) as ex:
        for r in ex.map(lambda k: gen_one(k, a), keys):
            print(json.dumps(r))


def portrait(raw, art=ART):
    """The raw painting as a portrait: the whole square reduced to art x art art px (a box filter
    averages each block, so the model's own pixel grid need not line up), then PIXEL px blocks."""
    im = Image.open(raw).convert("RGB")
    s = min(im.size)
    im = im.crop(((im.width - s) // 2, (im.height - s) // 2, (im.width + s) // 2, (im.height + s) // 2))
    return im.resize((art, art), Image.BOX).resize((art * PIXEL, art * PIXEL), Image.NEAREST)


def variants(key, rnd):
    return sorted((wdir(key) / "raw").glob(f"{rnd}_*.png"), key=lambda p: int(p.stem.split("_")[-1]))


def cut(a):
    for key in keys_of(a.keys):
        for raw in variants(key, a.round):
            portrait(raw, a.art).save(wdir(key) / "cut" / f"{raw.stem}.png")
    if a.review:
        review(a)


def review(a):
    """Each leader a row: its variants at 1x (as the trade screen shows them) and at the diplomacy
    row's size, on the interface navy, the variant number under each."""
    keys = keys_of(a.keys)
    navy = (3, 26, 72)
    rows = []
    for key in keys:
        cuts = [(p.stem.split("_")[-1], Image.open(p).convert("RGB")) for p in sorted((wdir(key) / "cut").glob(f"{a.round}_*.png"))]
        rows.append((key, cuts))
    size = max((im.width for _, c in rows for _, im in c), default=ART * PIXEL)
    cell = size + 56 + 24
    nv = max((len(c) for _, c in rows), default=1)
    out = Image.new("RGB", (110 + nv * cell, len(rows) * (size + 28)), navy)
    dr = ImageDraw.Draw(out)
    for i, (key, cuts) in enumerate(rows):
        y = i * (size + 28) + 6
        dr.text((6, y + size // 2), key, fill=(246, 198, 80))
        for j, (v, im) in enumerate(cuts):
            x = 110 + j * cell
            out.paste(im, (x, y))
            out.paste(im.resize((56, 56), Image.LANCZOS), (x + size + 8, y + size - 56))
            dr.text((x + 2, y + size + 4), f"{a.round}_{v}", fill=(251, 242, 214))
    p = Path(a.out) if a.out else ROOT / "work" / "leaders" / f"review_{a.round}_{keys[0]}.png"
    out.save(p)
    print(json.dumps({"review": str(p)}))


def accept(a):
    d = wdir(a.key)
    src = d / "cut" / f"{a.round}_{a.variant}.png"
    if not src.exists():
        raise SystemExit(f"{src}: no such cut (run `cut` first)")
    dst = ROOT / "roster" / "leaders"
    dst.mkdir(parents=True, exist_ok=True)
    key = f"leader_{a.key}"
    shutil.copy(src, dst / f"{key}.png")
    shutil.copy(d / f"prompt_{a.round}.txt", ROOT / "prompts" / "assets" / f"{key}.txt")
    man_p = ROOT / "roster" / "manifest.json"
    man = json.loads(man_p.read_text())
    calls = [json.loads(l) for l in (d / "calls.jsonl").read_text(encoding="utf-8").splitlines() if l.strip()]
    anchors = next((c["images"] for c in reversed(calls) if c.get("round") == a.round and c.get("ok")), [rel(p) for p in ANCHORS])
    im = Image.open(src)
    man[key] = {"kind": "leader_portrait", "file": f"leaders/{key}.png", "size": [im.width, im.height], "pixel": PIXEL,
                "art": im.width // PIXEL, "round": a.round, "variant": a.variant, "gen_size": "1024x1024",
                "prompt": f"prompts/assets/{key}.txt", "anchors": anchors}
    man_p.write_text(json.dumps(man, indent=1) + "\n")
    print(json.dumps({"accepted": key}))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("gen")
    p.add_argument("--keys", required=True); p.add_argument("--n", type=int, default=3)
    p.add_argument("--quality", default="medium"); p.add_argument("--round", default="r1")
    p.add_argument("--jobs", type=int, default=6); p.add_argument("--extra", help="a sentence added after the description")
    p.add_argument("--revise", help="ROUND:V raw appended as the last image")
    p.add_argument("--model", default=MODEL); p.add_argument("--dry-run", action="store_true")
    p = sub.add_parser("cut")
    p.add_argument("--keys", required=True); p.add_argument("--round", default="r1")
    p.add_argument("--art", type=int, default=ART); p.add_argument("--review", action="store_true"); p.add_argument("--out")
    p = sub.add_parser("review")
    p.add_argument("--keys", required=True); p.add_argument("--round", default="r1"); p.add_argument("--out")
    p = sub.add_parser("accept")
    p.add_argument("--key", required=True); p.add_argument("--variant", required=True); p.add_argument("--round", default="r1")
    a = ap.parse_args()
    {"gen": gen, "cut": cut, "review": review, "accept": accept}[a.cmd](a)


if __name__ == "__main__":
    main()
