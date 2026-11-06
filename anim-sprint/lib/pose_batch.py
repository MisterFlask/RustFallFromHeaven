"""Attack and hit key poses for every unit, by prompt alone (lib/prompt_pose.py; ANIMATING_UNITS.md, "Impact cards and
key poses"). The owner's request (5 November 2026): a strike and a hit frame for each unit, with unconventional
attacks wherever the unit and what it carries allow.

    python lib/pose_batch.py run [--only NAME,...] [--limit N]    # needs ComfyUI and KLEIN_LIB (see prompt_pose.py)
    python lib/pose_batch.py sheet [--from K] [--count N]         # review sheets of the picks
    python lib/pose_batch.py pick NAME attack|hit 1|2             # override a pick by eye
    python lib/pose_batch.py install [--from DIR]                 # copy the picks into the client, write key_poses.js

Inputs in pose_prompts/ (RULES.md says how they were written): `roster.json` (the units, from anim-all's roster_classified.json: facing, weapon,
description) and `chunk_*.prompts.json` ({name, who, attack, hit, unconventional} per unit). Per unit, each pose is one
call for two variants (`<unit>/attack_1.png`, `_2`), each finished with an automatic scale (`k_attack_1.png`), and the
pick written to `out/<short>_attack.png`: a variant clipped at the canvas edge loses, then the one that changed more
from the standing sprite wins (an overlap above 0.85 means it fell back to standing: flagged). A pose already in `out/`
is skipped, so a run resumes. `log.jsonl` has one line per unit.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).resolve().parent))
import prompt_pose as pp  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / "anim-sprint" / "work" / "poses_all"
PROMPTS = ROOT / "anim-sprint" / "pose_prompts"
OUT = WORK / "out"
ISO = ROOT / "crates" / "ffh-play" / "assets" / "app" / "iso"
POSE = ROOT / "crates" / "ffh-play" / "assets" / "app" / "pose"


def units():
    roster = json.loads((PROMPTS / "roster.json").read_text(encoding="utf-8"))
    prompts = {}
    for f in sorted(PROMPTS.glob("chunk_*.prompts.json")):
        for p in json.loads(f.read_text(encoding="utf-8")):
            prompts[p["name"]] = p
    return [dict(u, **prompts[u["name"]]) for u in roster if u["name"] in prompts]


def standing(sprite):
    W, H = pp.canvas_for(sprite)
    s = Image.open(sprite).convert("RGBA")
    c = Image.new("RGBA", (W, H))
    c.alpha_composite(s, (W // 2 - s.width // 2, H - 2 - (s.height - 2)))
    return c


def judge(k_png, stand):
    a = np.array(Image.open(k_png))[..., 3] > 0
    b = np.array(stand)[..., 3] > 0
    clipped = bool(a[0].any() or a[-1].any() or a[:, 0].any() or a[:, -1].any())
    iou = float((a & b).sum() / max(1, (a | b).sum()))
    return {"clipped": clipped, "iou": round(iou, 3)}


def pick(cands):
    """cands: [(k, judge)]; returns (k, flag)."""
    ok = [c for c in cands if not c[1]["clipped"]] or cands
    k, j = min(ok, key=lambda c: c[1]["iou"])
    flag = "clipped" if j["clipped"] else "no-pose" if j["iou"] > 0.85 else ""
    return k, flag


def run(only=None, limit=None):
    OUT.mkdir(parents=True, exist_ok=True)
    done = 0
    for u in units():
        short = u["name"].removeprefix("unit_")
        if only and u["name"] not in only and short not in only:
            continue
        sprite = ISO / f"{u['name']}.png"
        if not sprite.exists():
            continue
        todo = [p for p in ("attack", "hit") if not (OUT / f"{short}_{p}.png").exists()]
        if not todo:
            continue
        d = WORK / short
        stand = standing(sprite)
        rec = {"unit": short, "t": time.strftime("%H:%M:%S")}
        t0 = time.time()
        for p in todo:
            r = None
            for attempt in range(2):
                r = pp.gen(sprite, d, p, u[p], u["who"])
                if r and r.get("ok"):
                    break
            if not r or not r.get("ok"):
                rec[p] = {"error": (r or {}).get("error")}
                continue
            cands = []
            for k in (1, 2):
                src = d / f"{p}_{k}.png"
                if not src.exists():
                    continue
                kp = d / f"k_{p}_{k}.png"
                try:
                    sc = pp.finish(src, "auto", kp, sprite)
                except Exception as e:  # noqa: BLE001
                    rec[f"{p}_{k}"] = f"finish failed: {e}"
                    continue
                j = judge(kp, stand)
                j["scale"] = round(sc, 3)
                cands.append((k, j))
            if not cands:
                continue
            k, flag = pick(cands)
            Image.open(d / f"k_{p}_{k}.png").save(OUT / f"{short}_{p}.png")
            rec[p] = {"pick": k, "flag": flag, "cands": {c[0]: c[1] for c in cands}}
        rec["seconds"] = round(time.time() - t0, 1)
        with open(WORK / "log.jsonl", "a", encoding="utf-8") as f:
            f.write(json.dumps(rec) + "\n")
        print(json.dumps(rec), flush=True)
        done += 1
        if limit and done >= limit:
            break


def set_pick(name, pose, k):
    short = name.removeprefix("unit_")
    Image.open(WORK / short / f"k_{pose}_{k}.png").save(OUT / f"{short}_{pose}.png")


def sheet(start=0, count=12):
    """Rows of: standing | attack 1 | attack 2 | hit 1 | hit 2, the picks framed in gold, 2x."""
    us = [u for u in units() if (WORK / u["name"].removeprefix("unit_")).exists()][start:start + count]
    cw, ch = 220, 150
    img = Image.new("RGBA", (cw * 5 * 2, ch * len(us) * 2), (60, 70, 52, 255))
    d = ImageDraw.Draw(img)
    for r, u in enumerate(us):
        short = u["name"].removeprefix("unit_")
        sprite = ISO / f"{u['name']}.png"
        tiles = [standing(sprite)]
        for p in ("attack", "hit"):
            for k in (1, 2):
                f = WORK / short / f"k_{p}_{k}.png"
                tiles.append(Image.open(f) if f.exists() else Image.new("RGBA", (10, 10)))
        for c, t in enumerate(tiles):
            t = t.resize((t.width * 2, t.height * 2), Image.NEAREST)
            x, y = c * cw * 2, r * ch * 2
            img.alpha_composite(t.crop((0, 0, min(t.width, cw * 2), min(t.height, ch * 2))), (x, y))
            if c:
                p, k = ("attack", "hit")[(c - 1) // 2], (c - 1) % 2 + 1
                o = OUT / f"{short}_{p}.png"
                kf = WORK / short / f"k_{p}_{k}.png"
                if o.exists() and kf.exists() and np.array_equal(np.array(Image.open(o)), np.array(Image.open(kf))):
                    d.rectangle((x + 1, y + 1, x + cw * 2 - 2, y + ch * 2 - 2), outline=(246, 198, 80, 255), width=3)
        d.text((6, r * ch * 2 + 4), f"{start + r}: {short}", fill=(255, 255, 255, 255))
    path = WORK / f"sheet_{start:03d}.png"
    img.save(path)
    print(path)


# Poses made by hand for the owner, never overwritten by a batch: the ranger's launch (lib/ranger_launch.py), the
# scout's lunge and hit (picked from prompt_pose.py's tries), the catapult's (cut from its anim-sprint set).
KEEP = {"ranger_attack", "scout_attack", "scout_hit", "catapult_attack", "catapult_hit"}


def install(src=None):
    """The picks (from `src`: work/poses_all/out, or the sheet batch's work/pose_sheets/out) into the client (app/pose),
    and the table of who has which (app/key_poses.js)."""
    have = {}
    for f in sorted(Path(src or OUT).glob("*.png")):
        if f.stem in KEEP:
            continue
        Image.open(f).save(POSE / f.name)
    for f in sorted(POSE.glob("*.png")):
        short, pose = f.stem.rsplit("_", 1)
        have.setdefault(short, []).append(pose)
    lines = ",\n".join(f'  {json.dumps(k)}: {json.dumps(sorted(v))}' for k, v in sorted(have.items()))
    (POSE.parent / "key_poses.js").write_text(
        "// The key poses the client ships (app/pose/<unit>_<attack|hit>.png), by the short name of the M7 sprite.\n"
        "// Written by anim-sprint/lib/pose_batch.py install; ANIMATING_UNITS.md, \"Impact cards and key poses\".\n"
        f"export const KEY_POSES = {{\n{lines},\n}};\n", encoding="utf-8", newline="\n")
    print(len(have), "units with key poses")


if __name__ == "__main__":
    a = sys.argv[1:]
    opt = lambda k, d=None: a[a.index(k) + 1] if k in a else d
    if a[0] == "run":
        run(set(opt("--only").split(",")) if opt("--only") else None, int(opt("--limit")) if opt("--limit") else None)
    elif a[0] == "sheet":
        sheet(int(opt("--from", 0)), int(opt("--count", 12)))
    elif a[0] == "pick":
        set_pick(a[1], a[2], int(a[3]))
    elif a[0] == "install":
        install(opt("--from"))
    else:
        sys.exit(__doc__)
