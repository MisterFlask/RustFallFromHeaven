"""GIF and MP4 previews for units that so far have only an idle (no strike/flinch/death for lib/preview.py's sequence).

  python3 lib/preview_idle.py warrior skeleton phalanx lizardman paladin goblin

Per unit: final_<unit>/idle_3x.{gif,mp4}, the idle on a grass tile with its shadow and team banner, 3x NEAREST, as in
lib/preview.py's sequence_3x. Together: previews/idle_roster.{gif,mp4}, every unit side by side at 2x with its name.
The GIFs hold one loop (4 frames x 250 ms) and loop forever; the MP4s repeat it LOOPS times so a player shows it for a
while. Free: no image calls.
"""
import json, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path[:0] = [str(HERE), str(ROOT / "m7" / "lib")]

from PIL import Image, ImageDraw  # noqa: E402
import preview as pv  # noqa: E402

LOOPS = 8


def idle_frames(unit):
    d = ROOT / f"final_{unit}"
    man = json.loads((d / "idle" / "manifest.json").read_text())
    return [(Image.open(d / "idle" / f["file"]).convert("RGBA"), f["ms"]) for f in man["frames"]], d


def scene(frame, label=None):
    g = pv.grass()
    sc = Image.new("RGBA", (232, 200), (54, 92, 150, 255))
    sc.alpha_composite(g, (20, 64))
    pv.unit_on(sc, frame, 20 + 96, 64 + 54)
    if label:
        ImageDraw.Draw(sc).text((6, 4), label, fill=(240, 244, 255, 255), font=pv.font(12))
    return sc


def main(units):
    out_all = ROOT / "previews"
    out_all.mkdir(exist_ok=True)
    per_unit = {}
    for u in units:
        frames, d = idle_frames(u)
        pv.set_unit(d)  # the banner goes left of this unit's idle frame 0
        clip = [(scene(f, u).resize((232 * 3, 200 * 3), Image.NEAREST), ms) for f, ms in frames]
        pv.save_gif(clip, d / "idle_3x.gif")
        pv.save_mp4(clip * LOOPS, d / "idle_3x.mp4")
        # the same scenes at 2x for the roster (banner position is per unit, so draw them now)
        per_unit[u] = [(scene(f, u).resize((232 * 2, 200 * 2), Image.NEAREST), ms) for f, ms in frames]
        print(u, len(frames), "frames ->", d / "idle_3x.gif")
    cols = 3
    rows = (len(units) + cols - 1) // cols
    cw, ch = 232 * 2, 200 * 2
    n = min(len(v) for v in per_unit.values())
    roster = []
    for i in range(n):
        sheet = Image.new("RGBA", (cols * cw, rows * ch), (54, 92, 150, 255))
        for k, u in enumerate(units):
            sheet.alpha_composite(per_unit[u][i][0], ((k % cols) * cw, (k // cols) * ch))
        roster.append((sheet, per_unit[units[0]][i][1]))
    pv.save_gif(roster, out_all / "idle_roster.gif")
    pv.save_mp4(roster * LOOPS, out_all / "idle_roster.mp4")
    print("roster ->", out_all / "idle_roster.mp4")


if __name__ == "__main__":
    main(sys.argv[1:] or ["warrior", "skeleton", "phalanx", "lizardman", "paladin", "goblin"])
