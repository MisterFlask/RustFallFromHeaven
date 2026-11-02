"""Per-frame generation on the local model (FLUX 3 Image Klein + the pose-trained pixel-art LoRA, lib/klein.py), feeding
the recipe's unchanged build.

The local model is an edit model trained per frame (tools/pixel-lora/build_pose_dataset.py): reference 1 is the unit's
static sprite, reference 2 the grey pose mannequin, the answer the unit in that pose, all on the 160x128 canvas at 4x
on magenta. So each frame the recipe's sheets ask for is one call (`n` variants), with the same caption the LoRA was
trained on. Each variant is scored against its guide (does it take the pose, or did it fall back to the standing
sprite, is it the sprite's size), and the variants are then composed into the recipe's own sheet layout
(`work/<out>/raw_<sheet>_<k>.png`: variant rank k of every frame, cell 1 the true sprite), so `recipe.build` slices,
de-fringes, snaps, warps and writes them exactly as it does the OpenAI sheets, and `variants_<sheet>.png` shows them
for the pick by eye.

  import perframe
  perframe.generate(u, "final_<unit>", ["idle", "strike", "flinch", "death"], style)     # recipe.generate calls it
"""
import json, os, sys, time
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).resolve().parent))
import animkit as ak  # noqa: E402

S = 4  # the canvas at 4x is what the LoRA was trained on (640x512)
# A frame is re-rolled (RETRIES more calls at most) until its best variant scores GOOD with a guide overlap of
# GOOD_GUIDE: on the warrior the frames that took their pose scored 0.6-0.8 (guide 0.64-0.83), the ones that fell
# back to the standing sprite or drifted 0.36-0.47 (guide 0.45-0.56).
RETRIES = 2
GOOD = 0.5
GOOD_GUIDE = 0.55
# Frames not drawn but taken from another frame's variants: death 6, the corpse's small bounce, came back as the
# STANDING unit for the warrior and the lizardman in every try (6 variants each); death 5 (lying) holds instead.
REUSE = {("death", 6): ("death", 5)}
MAGENTA = (255, 0, 255)
LABEL = {"strike": "a strike", "flinch": "a hit reaction", "death": "a death", "walk": "a walk cycle"}


def caption(anim, i, beat):
    """The edit instruction the pose LoRA was trained with (build_pose_dataset.py imports it from here)."""
    what = "standing at rest." if (anim, i) == ("strike", 0) else f"frame {i} of {LABEL[anim]} animation: {beat.rstrip('.')}."
    return ("pxlhd2d, the character from the first image redrawn in exactly the pose and position of the grey mannequin "
            f"in the second image, {what} Same face, hair, outfit, colours, weapon, size and dark outlines as the first "
            "image; the mannequin is only a pose guide. Full body game unit sprite, 16-bit pixel art, on a flat magenta "
            "background.")


def on_magenta(im, scale=S):
    c = Image.new("RGB", im.size, MAGENTA)
    c.paste(im, (0, 0), im)
    return c.resize((im.width * scale, im.height * scale), Image.NEAREST)


def keyed_canvas(raw):
    """A 4x answer keyed and brought back to the 160x128 canvas (for scoring only; the slicer does the real cut)."""
    im = np.asarray(Image.open(raw).convert("RGB")).astype(np.int32)
    d = np.abs(im - np.array(MAGENTA)).sum(axis=2)
    alpha = np.where(d > 120, 255, 0).astype(np.uint8)
    a = np.dstack([im.astype(np.uint8), alpha])
    small = Image.fromarray(a, "RGBA").resize((ak.W, ak.H), Image.BOX)
    return small


def mask(im, t=128):
    return np.asarray(im.getchannel("A")) >= t


def iou(a, b):
    return float((a & b).sum()) / max(1, float((a | b).sum()))


def score(frame, guide, base, standing):
    """Higher is better: the overlap with the (dilated) guide, less how much more it overlaps the static sprite than the
    guide itself does (a figure redrawn standing still), less a size error for frames that stand."""
    M = mask(frame)
    if M.sum() < 200:
        return -9.0, {"empty": True}
    G = ndimage.binary_dilation(mask(guide, 60), iterations=3)
    B = mask(base)
    g, b, gb = iou(M, G), iou(M, B), iou(G, B)
    s = g - 0.8 * max(0.0, b - gb - 0.05)
    info = {"guide": round(g, 3), "sprite": round(b, 3), "guide_sprite": round(gb, 3)}
    if standing:
        rows = np.nonzero(M.any(axis=1))[0]
        rows0 = np.nonzero(B.any(axis=1))[0]
        h, h0 = rows.max() - rows.min() + 1, rows0.max() - rows0.min() + 1
        info["height"] = round(h / h0, 3)
        s -= 1.5 * max(0.0, abs(h / h0 - 1) - 0.08)
    return round(s, 3), info


def generate(u, out, anims, style, n=2, reroll=None):
    """Draws every frame the sheets need, or with `reroll` (a set of (anim, frame)) only those frames: their earlier
    variants move to work/<out>/rejected/ and new ones are drawn, while every other frame keeps (and re-scores) the
    variants already on disk. Then the raw sheets are composed again."""
    import guides, klein, recipe, sheet  # noqa: E401 (recipe imports this module)
    root = recipe.ROOT
    wdir = root / "work" / out
    wdir.mkdir(parents=True, exist_ok=True)
    poses = recipe.unit_poses(u)
    base = recipe.base_canvas(u)
    sprite_path = wdir / "pf_sprite.png"
    on_magenta(base).save(sprite_path)
    need = [s for s in recipe.SHEETS if (s in anims) or (s == "flinch" and "death" in anims)]
    log = {"calls": [], "frames": {}}
    t0 = time.time()
    for name in need:
        geom, _ = recipe.make_sheet(u, poses, name, wdir, style)  # the template + geometry the slicer needs
        ranked = []
        for an, i in recipe.SHEETS[name]:
            if (an, i) in REUSE:  # drawn from another frame's variants (see REUSE)
                ranked.append(ranked[recipe.SHEETS[name].index(REUSE[(an, i)])])
                continue
            guide = recipe.render_guide(u, poses[an][i], style)
            gpath = wdir / f"pf_guide_{an}{i}.png"
            on_magenta(guide).save(gpath)
            scored = []
            old = sorted(wdir.glob(f"pf_{an}{i}_*.png")) + sorted(wdir.glob(f"pf_{an}{i}.png"))
            if reroll is not None and (an, i) not in reroll and old:
                for f in old:  # keep this frame's earlier variants
                    sc, info = score(keyed_canvas(f), guide, base, (an, i) in recipe.standing(u))
                    scored.append((sc, str(f), info))
                scored.sort(key=lambda t: -t[0])
                log["frames"][f"{an} {i}"] = [{"file": Path(f).name, "score": sc, **info} for sc, f, info in scored]
                ranked.append(scored)
                continue
            tag = ""
            if reroll is not None and old:
                rej = wdir / "rejected"
                rej.mkdir(exist_ok=True)
                for f in old:
                    os.replace(f, rej / f.name)
                tag = f"_x{len(list(rej.glob(f'pf_{an}{i}_*')))}"
            # A frame whose best variant barely takes its pose (or falls back to the standing sprite) gets more
            # variants with fresh seeds, up to RETRIES more calls.
            for attempt in range(1 + RETRIES):
                res = klein.edit(tag=u.get("tag"), images=[sprite_path, gpath],
                                 prompt=caption(an, i, recipe.beat_text(u, an, i)),
                                 out=wdir / (f"pf_{an}{i}{tag}.png" if not attempt else f"pf_{an}{i}{tag}_r{attempt}.png"), n=n,
                                 ref_mp=(ak.W * S) * (ak.H * S) / 1e6)
                log["calls"].append({"frame": f"{an} {i}", "attempt": attempt,
                                     **{k: res.get(k) for k in ("ok", "seconds", "error")}})
                if not res.get("ok"):
                    print(f"perframe: {an} {i} failed: {res.get('error')}", file=sys.stderr)
                for f in res.get("files", []):
                    sc, info = score(keyed_canvas(f), guide, base, (an, i) in recipe.standing(u))
                    scored.append((sc, f, info))
                scored.sort(key=lambda t: -t[0])
                if scored and scored[0][0] >= GOOD and scored[0][2].get("guide", 0) >= GOOD_GUIDE:
                    break
            log["frames"][f"{an} {i}"] = [{"file": Path(f).name, "score": sc, **info} for sc, f, info in scored]
            ranked.append(scored)
        # compose rank k of every frame into raw_<name>_<k>.png in the template's layout
        x0, y0, x1, y1 = geom["region"]
        s = geom["scale"]
        for k in range(n):
            sh = Image.open(wdir / f"template_{name}.png").convert("RGB")
            for cell, scored in enumerate(ranked, 1):
                c = geom["cells"][cell]
                ox, oy = c["x"] + geom["inner"][0], c["y"] + geom["inner"][1]
                sh.paste(MAGENTA, (ox, oy, ox + (x1 - x0) * s, oy + (y1 - y0) * s))
                if not scored:
                    continue
                _, f, _ = scored[min(k, len(scored) - 1)]
                im = Image.open(f).convert("RGB").crop((x0 * S, y0 * S, x1 * S, y1 * S))
                sh.paste(im.resize(((x1 - x0) * s, (y1 - y0) * s), Image.LANCZOS), (ox, oy))
            sh.save(wdir / f"raw_{name}_{k + 1}.png")
    log["seconds"] = round(time.time() - t0, 1)
    (wdir / "gen_log.json").write_text(json.dumps(log, indent=1))
    return {"seconds": log["seconds"], "calls": len(log["calls"]),
            "failed": [c["frame"] for c in log["calls"] if not c["ok"]]}
