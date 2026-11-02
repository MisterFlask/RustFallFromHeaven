"""Cast pose set (the winner of three designs judged blind, 2026-11-01): the far set's idle, flinch and death with a SPELL CAST as the strike.

For casters that look LOWER-LEFT with a staff (or wand, orb, dagger) in the far, screen-left hand: the archmage, the
adepts, the priest, the druid, the lich, the disciple. Same schema, canvas (160x128, anchor (80,126)), joints, frame
counts and timings as lib/poses_src.py (the "far" set); only the strike differs. The engine draws the spell itself at
the strike's "hit" event (frame 4), so the release frame puts the staff head out at the far lower-left end of one long
diagonal through the body, with nothing else out there.

  python lib/poses_cast_src.py     # writes poses_cast.json (a unit file points at it with "poses_from")

Joints as in lib/poses_src.py: r_* = the staff arm and the leg on that side (FAR side, screen-left), l_* = the free
arm and leg (NEAR side, screen-right). tip = the staff HEAD (the guide draws the staff from r_ha to tip; the butt end
below the hand is not drawn). No shield in the strike. head_tilt + = tipped back toward the upper-right.

The cast, as a javelin throw with a staff (the target tile is at (-96, +48), 27 degrees below level to the left):
  0 ready      REST exactly.
  1 gather     weight back onto the rear leg, the staff lifted upright in front of the chest, the free hand opening
               out at the side.
  2 charge     (anticipation key) arched back and crouched on the rear leg, the staff held high overhead at arm's
               length and tilted back, its head up behind the head; the free arm raised out the other way, palm open:
               a wide, tall Y leaning to the upper-right.
  3 swing      (50 ms) the front foot steps to the lower-left and the torso whips forward; the staff comes over the
               top, level, its head out to the left at head height; the free arm sweeps down.
  4 release    (hit) deep lunge, body low and leaning in; staff arm fully extended to the lower-left with the staff
               along the attack line, its head the farthest point of the figure; the free arm flung back behind the
               hip, open palm: one diagonal from the free hand to the staff head.
  5 hold       the staff stays pointed at the lower-left, its head dipping a little; body sinks a touch.
  6 recover    body rises, the front foot pulls back halfway, the staff comes back up toward its ready angle.
  7 settle     almost REST (warped from the real sprite, so a small move): as the far set's settle.

Readability rules: the two key frames (2 and 4) have opposite outlines (tall and leaning back vs low and long toward
the lower-left); the staff never crosses the head or the torso; the free arm never crosses the torso (it stays on the
screen-right side, where a near arm in front of the chest would vanish at 1x).
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from poses_src import DEATH, FLINCH, HEAD_R, IDLE, REST, DEFAULT_ORDER, up  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
NO_SHIELD = {k: v for k, v in REST.items() if k != "shield"}

# back to front, no shield
O_REST = ["sword", "r_arm", "r_leg", "torso", "l_leg", "head", "l_arm"]
O_RAISED = ["sword", "r_leg", "torso", "l_leg", "r_arm", "head", "l_arm"]       # far arm up past the face
O_FORWARD = ["r_leg", "torso", "l_leg", "head", "r_arm", "sword", "l_arm"]      # far arm thrust out in front


def C(**kw):
    p = dict(NO_SHIELD)
    p.update(kw)
    return p


STRIKE = [
    # 0 ready: REST exactly (no shield)
    C(order=O_REST),
    # 1 gather: weight back, staff lifted upright in front of the chest, free hand opening out at the side
    C(head=(87, 54), neck=(86, 67), chest=(85, 75), pelvis=(85, 91),
      r_sh=(73, 70), r_el=(67, 77), r_ha=(65, 67), tip=(70, 31),
      l_sh=(98, 70), l_el=(104, 78), l_ha=(110, 75),
      r_hip=(80, 95), r_kn=(74, 107), r_ft=(67, 121),
      l_hip=(91, 95), l_kn=(95, 107), l_ft=(98, 123),
      head_tilt=6, order=O_RAISED),
    # 2 charge (anticipation key): arched back, crouched on the rear leg, staff high overhead tilted back,
    #   free arm raised out the other way
    C(head=(96, 60), neck=(93, 72), chest=(91, 80), pelvis=(87, 95),
      r_sh=(80, 75), r_el=(76, 62), r_ha=(78, 48), tip=(100, 17),
      l_sh=(103, 75), l_el=(112, 68), l_ha=(119, 59),
      r_hip=(82, 99), r_kn=(74, 109), r_ft=(66, 121),
      l_hip=(93, 99), l_kn=(100, 109), l_ft=(99, 123),
      head_tilt=14, order=O_RAISED),
    # 3 swing (50 ms): front foot steps to the lower-left, torso whips forward, staff over the top and level
    C(head=(74, 57), neck=(76, 69), chest=(76, 77), pelvis=(80, 94),
      r_sh=(67, 72), r_el=(59, 66), r_ha=(51, 62), tip=(18, 55),
      l_sh=(89, 72), l_el=(97, 79), l_ha=(104, 85),
      r_hip=(75, 98), r_kn=(66, 109), r_ft=(58, 122),
      l_hip=(86, 97), l_kn=(93, 109), l_ft=(100, 123),
      head_tilt=-4, order=O_FORWARD),
    # 4 release (hit): deep lunge, staff thrust along the attack line, free arm flung back behind the hip
    C(head=(67, 62), neck=(70, 74), chest=(72, 81), pelvis=(78, 97),
      r_sh=(63, 76), r_el=(55, 81), r_ha=(47, 86), tip=(18, 102),
      l_sh=(86, 76), l_el=(95, 82), l_ha=(105, 87),
      r_hip=(73, 100), r_kn=(62, 110), r_ft=(55, 122),
      l_hip=(84, 100), l_kn=(94, 111), l_ft=(104, 123),
      head_tilt=-10, order=O_FORWARD),
    # 5 hold: the staff stays on the target, head dipping a little; body sinks a touch
    C(head=(68, 63), neck=(71, 75), chest=(73, 82), pelvis=(78, 98),
      r_sh=(64, 77), r_el=(56, 84), r_ha=(48, 89), tip=(21, 109),
      l_sh=(86, 77), l_el=(94, 84), l_ha=(102, 90),
      r_hip=(73, 101), r_kn=(62, 110), r_ft=(55, 122),
      l_hip=(84, 101), l_kn=(94, 111), l_ft=(104, 123),
      head_tilt=-6, order=O_FORWARD),
    # 6 recover: rising, front foot back halfway, staff back up toward its ready angle
    C(head=(78, 56), neck=(79, 68), chest=(78, 76), pelvis=(80, 92),
      r_sh=(67, 71), r_el=(62, 80), r_ha=(57, 85), tip=(41, 53),
      l_sh=(93, 71), l_el=(97, 80), l_ha=(102, 86),
      r_hip=(76, 96), r_kn=(69, 107), r_ft=(62, 122),
      l_hip=(87, 96), l_kn=(92, 108), l_ft=(98, 123),
      head_tilt=-2, order=O_REST),
    # 7 settle: almost REST (the far set's settle)
    up(C(r_ha=(61, 85), tip=(46, 53), order=O_REST),
       ["head", "neck", "chest", "pelvis", "r_sh", "l_sh", "r_hip", "l_hip"], 1),
]

POSES = {"idle": IDLE, "strike": STRIKE, "flinch": FLINCH, "death": DEATH}
MS = [100, 120, 160, 50, 120, 120, 100, 100]  # the recipe's strike timing (lib/beats.py); "hit" on frame 4
assert len(STRIKE) == len(MS) == 8 and all("shield" not in p for p in STRIKE)
assert {k: v for k, v in STRIKE[0].items() if k != "order"} == NO_SHIELD


def as_json():
    out = {"canvas": [160, 128], "anchor": [80, 126], "sprite_offset": [32, 8], "head_r": HEAD_R,
           "default_order": DEFAULT_ORDER, "anims": {}}
    for anim, frames in POSES.items():
        fl = []
        for p in frames:
            q = {k: (list(v) if isinstance(v, tuple) else v) for k, v in p.items()}
            if "sword" in q:
                q["sword"] = [list(q["sword"][0]), list(q["sword"][1])]
                q.pop("tip", None)
            q.setdefault("order", DEFAULT_ORDER)
            fl.append(q)
        out["anims"][anim] = fl
    return out


if __name__ == "__main__":
    (ROOT / "poses_cast.json").write_text(json.dumps(as_json(), indent=1))
    print("wrote", ROOT / "poses_cast.json")
