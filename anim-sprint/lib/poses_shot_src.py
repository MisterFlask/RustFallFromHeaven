"""Shot pose set (the top of three designs judged blind, 2026-11-01): the far set's idle, flinch and death with a RANGED SHOT as the strike.

For shooters that look LOWER-LEFT with the bow (or crossbow, or gun) in the far, screen-left hand: the archer, the
longbowman, the crossbowman, the marksman, the hunter. Same schema, canvas (160x128, anchor (80,126)), joints, frame
counts and timings as lib/poses_src.py (the "far" set); only the strike differs. The engine plays the impact on the
defender at the strike's "hit" event (frame 4), so the release frame is the extreme of the move: the longest, lowest
outline of the set, aimed along the attack line.

  python lib/poses_shot_src.py     # writes poses_shot.json (a unit file points at it with "poses_from")

Joints as in lib/poses_src.py: r_* = the bow arm and the leg on that side (FAR side, screen-left), l_* = the draw arm
and leg (NEAR side, screen-right). r_ha = the bow GRIP, tip = the tip of the bow's UPPER limb (the guide draws the
upper limb as a line r_ha -> tip; the lower limb and the string are not drawn). No shield in the strike. head_tilt + =
tipped back toward the upper-right.

The shot, built like a fighting-game special for anticipation and snap (the target tile is at (-96, +48), 27 degrees
below level to the left):
  0 ready      REST exactly.
  1 nock       (120 ms) a small dip: knees give, head bows to look at the nock; the bow comes up in front of the chest,
               canted forward, and the draw hand meets the string at the grip. The compact frame before the big open.
  2 full draw  (160 ms, the wind-up extreme) the archer coils back toward the upper-right and sinks: weight on the
               bent rear leg, front leg reaching out with its foot light; the bow arm pushed out to the left and a
               little UP (aimed above the target), the bow upright and highest; the draw hand anchored at the jaw, the
               draw elbow cocked far back and high. Held longest, for the tension.
  3 drop       (50 ms) the snap: the body drives forward, the front foot stamps down to the lower-left and the bow
               swings DOWN onto the attack line, still at full draw.
  4 release    (hit) the extreme: a deep lunge to the lower-left; the bow arm locked straight along the attack line, the
               bow's top already rolling forward; the draw hand has snapped back past the ear and flown out to the
               upper-right, fingers open: one long diagonal from the bow grip to the draw hand.
  5 recoil     (overshoot) the bow keeps rolling forward and dips (its top tips toward the target), the draw hand
               overshoots up and back, the body rebounds up and back a touch; the feet stay planted.
  6 recover    body rises, the front foot pulls back halfway, the bow returns to its carry angle at the side, the draw
               hand drops to the hip.
  7 settle     almost REST (warped from the real sprite, so a small move): as the far set's settle.

Readability rules: the two key frames (2 and 4) have opposite outlines (tall, coiled back to the upper-right, aim high
vs low and long to the lower-left, aim on the target); the bow never crosses the head; the draw arm only crosses the
body where an archer's must (the hand at the jaw in 2-3, at the grip in 1). The bow's upper limb sweeps one way the
whole move (canted forward 1, upright 2, then rolling forward 3-5) so the weapon line reads as one arc. Both feet
stay planted from frame 3 to 5 (the front foot lands on 3, the rear foot never slides).

Retarget note: recipe.retarget turns each unit bone by the source bone's turn from REST, whose "weapon" leans 23
degrees forward of upright and whose draw arm hangs. The bow lines here turn less than 30 degrees from it, so a unit's
own bow (or crossbow, or gun barrel) keeps roughly its own angle, tipping back on 2 and forward on 4-5. The roster's
bow sprites stand at full draw already (bow arm out, draw hand at the face); the draw arm is not in WEAPON_CHAIN, so
for them check the draw arm in --check-guides first.
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
O_AIM = ["r_leg", "torso", "l_leg", "r_arm", "sword", "head", "l_arm"]      # bow arm out in front, draw arm nearest


def C(**kw):
    p = dict(NO_SHIELD)
    p.update(kw)
    return p


STRIKE = [
    # 0 ready: REST exactly (no shield)
    C(order=O_REST),
    # 1 nock: small dip, head bowed to the nock, bow up in front of the chest canted forward, draw hand at the grip
    C(head=(81, 56), neck=(82, 68), chest=(82, 76), pelvis=(83, 92),
      r_sh=(70, 71), r_el=(64, 80), r_ha=(70, 73), tip=(58, 39),
      l_sh=(94, 71), l_el=(88, 81), l_ha=(76, 74),
      r_hip=(78, 96), r_kn=(74, 107), r_ft=(67, 121),
      l_hip=(89, 96), l_kn=(93, 108), l_ft=(98, 123),
      head_tilt=-6, order=O_AIM),
    # 2 full draw (wind-up extreme): coiled back to the upper-right on the bent rear leg, front foot light, bow pushed
    #   out to the left and a little up, draw hand at the jaw, draw elbow far back and high
    C(head=(97, 61), neck=(96, 73), chest=(92, 81), pelvis=(89, 97),
      r_sh=(84, 76), r_el=(73, 70), r_ha=(62, 64), tip=(61, 29),
      l_sh=(106, 73), l_el=(115, 63), l_ha=(101, 69),
      r_hip=(84, 100), r_kn=(75, 109), r_ft=(65, 121),
      l_hip=(95, 100), l_kn=(92, 111), l_ft=(101, 123),
      head_tilt=8, order=O_AIM),
    # 3 drop (50 ms, the snap): body drives forward, front foot stamps to the lower-left, bow swings down onto the line
    C(head=(78, 61), neck=(79, 73), chest=(78, 81), pelvis=(81, 97),
      r_sh=(67, 77), r_el=(58, 80), r_ha=(48, 82), tip=(42, 47),
      l_sh=(90, 74), l_el=(94, 65), l_ha=(82, 69),
      r_hip=(76, 100), r_kn=(66, 111), r_ft=(56, 123),
      l_hip=(87, 100), l_kn=(92, 111), l_ft=(101, 123),
      head_tilt=-4, order=O_AIM),
    # 4 release (hit): deep lunge, bow arm locked on the attack line, bow top rolling forward, draw hand flung back
    #   past the ear to the upper-right
    C(head=(69, 65), neck=(72, 77), chest=(73, 85), pelvis=(78, 100),
      r_sh=(61, 80), r_el=(50, 86), r_ha=(40, 91), tip=(27, 58),
      l_sh=(84, 77), l_el=(93, 73), l_ha=(103, 67),
      r_hip=(72, 103), r_kn=(61, 111), r_ft=(53, 123),
      l_hip=(84, 103), l_kn=(92, 113), l_ft=(101, 123),
      head_tilt=-8, order=O_AIM),
    # 5 recoil (overshoot): bow rolls on forward and dips, draw hand overshoots up and back, body rebounds a touch
    C(head=(72, 63), neck=(74, 75), chest=(75, 83), pelvis=(79, 99),
      r_sh=(63, 78), r_el=(54, 86), r_ha=(44, 93), tip=(22, 65),
      l_sh=(86, 75), l_el=(96, 68), l_ha=(106, 60),
      r_hip=(73, 102), r_kn=(62, 111), r_ft=(53, 123),
      l_hip=(85, 102), l_kn=(93, 112), l_ft=(101, 123),
      head_tilt=-2, order=O_AIM),
    # 6 recover: rising, front foot back halfway, bow back to its carry angle at the side, draw hand at the hip
    C(head=(79, 57), neck=(79, 69), chest=(78, 77), pelvis=(81, 93),
      r_sh=(67, 72), r_el=(62, 80), r_ha=(56, 86), tip=(42, 53),
      l_sh=(92, 72), l_el=(98, 80), l_ha=(102, 86),
      r_hip=(76, 97), r_kn=(69, 108), r_ft=(61, 122),
      l_hip=(87, 97), l_kn=(92, 108), l_ft=(99, 123),
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
    (ROOT / "poses_shot.json").write_text(json.dumps(as_json(), indent=1))
    print("wrote", ROOT / "poses_shot.json")
