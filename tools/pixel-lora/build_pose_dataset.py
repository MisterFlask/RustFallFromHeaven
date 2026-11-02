"""The pose dataset for the pixel-art LoRA v2 (docs/art/PIXEL_LORA.md, "v2: poses"): edit triples from the finished
anim-sprint sets, so the LoRA learns to redraw a unit in a given pose (the v1 LoRA only ever saw standing sprites, and
with it Klein drew every requested pose back as the standing sprite).

For each unit set (`anim-sprint/final_<unit>_nofx/`, frames drawn by gpt-image from pose guides) and each frame the
image model drew (strike 1-6, flinch 0-2, death 1-7, walk 0-5) plus the rest pose (strike 0, the sprite itself):
  target/<id>.png       the frame, 160x128 at 4x (640x512), nearest-neighbour, on flat magenta
  ref_sprite/<id>.png   the unit's static sprite on the same canvas (reference 1)
  ref_guide/<id>.png    the grey pose mannequin the frame was drawn from (reference 2: anim-sprint/lib/guides.py)
  target/<id>.txt       the caption: the trigger word, the edit instruction and the frame's beat text
and the same triple mirrored (left-right swapped in the text). Units in --holdout are left out to test on.

  C:/ComfyUI/python_embeded/python.exe tools/pixel-lora/build_pose_dataset.py --out C:/ai-lora/pixel_hd2d/dataset_pose
      [--holdout warrior,lizardman]
"""
import argparse, json, sys
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
ANIM = ROOT / "anim-sprint"
sys.path.insert(0, str(ANIM / "lib"))
import beats, guides, recipe  # noqa: E402
from perframe import caption  # noqa: E402  (the caption the recipe sends at inference)

S = 4
MAGENTA = (255, 0, 255)
UNITS = ["swordsman", "ogre", "hyborem", "warrior", "skeleton", "phalanx", "lizardman", "paladin", "goblin", "axeman",
         "champion", "donal", "losha", "maros", "guybrush"]
DRAWN = {"strike": range(0, 7), "flinch": range(0, 3), "death": range(1, 8), "walk": range(0, 6)}


def on_magenta(im):
    c = Image.new("RGB", im.size, MAGENTA)
    c.paste(im, (0, 0), im)
    return c.resize((im.width * S, im.height * S), Image.NEAREST)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--holdout", default="warrior,lizardman")
    a = ap.parse_args()
    out = Path(a.out)
    for d in ("target", "ref_sprite", "ref_guide"):
        (out / d).mkdir(parents=True, exist_ok=True)
    hold = set(filter(None, a.holdout.split(",")))
    n = 0
    for unit in UNITS:
        if unit in hold:
            continue
        u = recipe.load_unit(str(ANIM / f"units/{unit}.json"))
        base = recipe.base_canvas(u)
        poses = json.loads((ANIM / f"final_{unit}_nofx/unit_poses.json").read_text())["anims"]
        walk_poses = None
        wp = ANIM / f"final_{unit}/walk/unit_poses.json"
        for anim, frames in DRAWN.items():
            src = ANIM / (f"final_{unit}/walk" if anim == "walk" else f"final_{unit}_nofx/{anim}")
            if not src.exists():
                continue
            if anim == "walk":
                walk_poses = walk_poses or (json.loads(wp.read_text())["anims"]["walk"] if wp.exists() else None)
                if walk_poses is None:
                    continue
            for i in frames:
                f = src / f"frame_{i:02d}.png"
                if not f.exists():
                    continue
                pose = walk_poses[i] if anim == "walk" else poses[anim][i]
                beat = "" if anim == "walk" else recipe.beat_text(u, anim, i)
                if anim == "walk":
                    beat = f"walking toward the {u['facing']}, mid-stride"
                frame = Image.open(f).convert("RGBA")
                guide = guides.render_guide(pose, "grey")
                for mirror in (False, True):
                    tid = f"{unit}_{anim}{i}{'_m' if mirror else ''}"
                    ims = [frame, base, guide]
                    if mirror:
                        ims = [im.transpose(Image.FLIP_LEFT_RIGHT) for im in ims]
                    on_magenta(ims[0]).save(out / "target" / f"{tid}.png")
                    on_magenta(ims[1]).save(out / "ref_sprite" / f"{tid}.png")
                    on_magenta(ims[2]).save(out / "ref_guide" / f"{tid}.png")
                    text = beats.swap_sides(beat) if mirror else beat
                    (out / "target" / f"{tid}.txt").write_text(caption(anim, i, text), encoding="utf-8")
                    n += 1
    print(json.dumps({"triples": n, "out": str(out), "holdout": sorted(hold)}))


if __name__ == "__main__":
    main()
