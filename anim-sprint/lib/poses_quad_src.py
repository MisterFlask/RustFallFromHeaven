"""Source of poses_quad.json: the quadruped pose set, a four-legged animal (a wolf or lion build) looking LOWER-LEFT.

Same canvas as poses.json (160x128, anchor (80,126)), same frame counts and timings (lib/beats.py): idle 4, strike 8
(hit on frame 4), flinch 4, death 8 (the last frame held). Joints as lib/quad.py documents them (head, snout, jaw,
neck, withers, spine, hips, tail_base, tail_mid, tail_tip, and fn/ff/hn/hf _top/_knee/_paw; wings wn/wf _root/_wrist/
_tip, used only by units that have wings). The near side (fn, hn, wn) faces the viewer: the animal's left side, lower
on the screen and drawn in front.

  python lib/poses_quad_src.py      # writes poses_quad.json
  python lib/quad.py --poses poses_quad.json --out work/quad/src_quad.png

Per-frame keys besides the joints: planted = legs whose paws stay on the ground (the retarget solves them by IK onto
the unit's own paw positions, moved as the source paw moves); abs = {group: weight} blends the unit's bones toward
this frame's absolute angles (the lying frames, so an animal drawn sitting still ends lying flat); ground = "body"
keeps the frame's height above the ground for a lying or falling body; eyes = open | shut.

The beats (lib/beats_quad.py has the texts; the attacker stands at the lower-left, the target tile at (-96, +48)):
  idle    breathing: the barrel and the head rise 1 px and settle, the tail sways; paws planted.
  strike  0 ready; 1 crouch: the weight shifts back onto the haunches, the head drops to aim; 2 coil (wind-up
          extreme): body drawn far back and low over the bent hind legs, head low and aimed at the lower-left, tail
          stiff; 3 launch (50 ms): the hind legs drive, the body springs up and forward, forelegs reaching, jaws opening;
          4 contact (hit): fully stretched toward the lower-left, the head the farthest point with the jaws wide open,
          the forelegs reaching out at the target with the claws spread, the hind paws still planted; 5 follow-through:
          the forepaws land ahead and the head bites down; 6 recover: steps back halfway; 7 settle (a warp).
  flinch  0 impact: hit from the lower-left, the head tossed up and back, eyes shut, jaws open, the body jolts back to
          the upper-right and the forepaws leave the ground; 1 recoil extreme: half-rearing back, tail tucked;
          2 recovering; 3 settle (a warp).
  death   0 = flinch 0; 1 stagger back: thrown up and back onto the hind legs, forelegs dangling; 2 legs buckle: the
          forepaws land and the haunches drop, head drooping; 3 collapse onto the belly, head sinking to the ground;
          4 rolls onto its side away from the viewer, the legs swinging out; 5 lying on its side, legs stretched out
          stiff along the ground (forelegs forward, hind legs back), head flat; 6 bounce: head and legs lift a little;
          7 corpse: still, eyes shut.
Readability: the coil (2) and the contact (4) have opposite outlines (short, tall at the rump and low at the head vs
long and low toward the lower-left); nothing reaches toward the attacker in the flinch and death.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

REST = dict(
    head=(42, 79), snout=(29, 85), jaw=(32, 88), neck=(54, 84),
    withers=(64, 90), spine=(82, 89), hips=(100, 87),
    tail_base=(111, 82), tail_mid=(119, 89), tail_tip=(123, 101),
    fn_top=(63, 98), fn_knee=(60, 111), fn_paw=(58, 123),
    ff_top=(58, 96), ff_knee=(55, 108), ff_paw=(53, 119),
    hn_top=(100, 94), hn_knee=(106, 108), hn_paw=(102, 123),
    hf_top=(95, 92), hf_knee=(100, 105), hf_paw=(96, 119),
    # wings (only units with wing joints use them): raised and swept back toward the upper-right
    wn_root=(70, 81), wn_wrist=(84, 56), wn_tip=(110, 50),
    wf_root=(64, 80), wf_wrist=(72, 52), wf_tip=(96, 42),
)
ALL = ("ff", "hf", "fn", "hn")
HIND = ("hf", "hn")
LYING_ORDER = ["wf", "tail", "ff", "hf", "body", "wn", "hn", "fn", "neck", "head"]
REACH_ORDER = ["wf", "hf", "tail", "body", "wn", "hn", "neck", "ff", "head", "fn"]  # forelegs out in front


def Q(planted=ALL, **kw):
    p = dict(REST)
    p.update(kw)
    p["planted"] = list(planted)
    return p


def shift(p, dx, dy, keys=None):
    for k in keys or [k for k in p if isinstance(p[k], tuple)]:
        p[k] = (p[k][0] + dx, p[k][1] + dy)
    return p


BODY_KEYS = ["head", "snout", "jaw", "neck", "withers", "spine", "hips", "tail_base", "tail_mid", "tail_tip",
             "fn_top", "ff_top", "hn_top", "hf_top", "wn_root", "wn_wrist", "wn_tip", "wf_root", "wf_wrist", "wf_tip"]

IDLE = [
    Q(),
    shift(Q(tail_tip=(124, 100)), 0, -1, ["head", "snout", "jaw", "neck", "withers", "spine", "wn_root", "wn_wrist",
                                          "wn_tip", "wf_root", "wf_wrist", "wf_tip"]),
    shift(Q(tail_mid=(120, 89), tail_tip=(125, 100)), 0, -1, ["head", "snout", "jaw", "neck", "withers", "spine",
                                                              "hips", "wn_root", "wn_wrist", "wn_tip", "wf_root",
                                                              "wf_wrist", "wf_tip"]),
    Q(tail_tip=(124, 101), jaw=(32, 89)),
]

STRIKE = [
    Q(),
    # 1 crouch: weight back onto the haunches, body lower, head dropped to aim; paws planted
    Q(head=(47, 86), snout=(34, 91), jaw=(36, 94), neck=(58, 89),
      withers=(68, 95), spine=(85, 93), hips=(102, 91),
      tail_base=(113, 86), tail_mid=(122, 89), tail_tip=(129, 96),
      fn_top=(67, 103), fn_knee=(61, 113), ff_top=(62, 101), ff_knee=(56, 110),
      hn_top=(102, 98), hn_knee=(110, 110), hf_top=(97, 96), hf_knee=(104, 107),
      wn_root=(73, 86), wn_wrist=(90, 62), wn_tip=(116, 58), wf_root=(67, 85), wf_wrist=(78, 58), wf_tip=(102, 50)),
    # 2 coil (wind-up extreme): drawn far back and low over the folded hind legs, head low and aimed, tail stiff;
    #   wings raised high
    Q(abs={"neck": 0.4}, head=(53, 95), snout=(40, 98), jaw=(42, 101), neck=(63, 96),
      withers=(73, 101), spine=(90, 98), hips=(106, 94),
      tail_base=(117, 89), tail_mid=(127, 86), tail_tip=(136, 82),
      fn_top=(72, 109), fn_knee=(63, 116), ff_top=(67, 107), ff_knee=(58, 112),
      hn_top=(106, 101), hn_knee=(116, 113), hf_top=(101, 99), hf_knee=(110, 110),
      wn_root=(78, 91), wn_wrist=(88, 60), wn_tip=(104, 36), wf_root=(72, 90), wf_wrist=(78, 58), wf_tip=(90, 32)),
    # 3 launch (50 ms): hind legs drive, the body springs up and forward, forelegs reaching, jaws opening
    Q(planted=HIND, abs={"neck": 0.5, "head": 0.3, "body": 0.3}, q_order=REACH_ORDER, head=(33, 77), snout=(20, 82), jaw=(22, 87), neck=(45, 81),
      withers=(55, 85), spine=(73, 87), hips=(91, 90),
      tail_base=(102, 86), tail_mid=(112, 85), tail_tip=(122, 83),
      fn_top=(53, 93), fn_knee=(43, 97), fn_paw=(34, 103), ff_top=(49, 91), ff_knee=(39, 93), ff_paw=(30, 98),
      hn_top=(92, 97), hn_knee=(102, 109), hf_top=(87, 95), hf_knee=(97, 106),
      wn_root=(62, 76), wn_wrist=(82, 58), wn_tip=(108, 60), wf_root=(56, 75), wf_wrist=(72, 54), wf_tip=(98, 54)),
    # 4 contact (hit): stretched long and low toward the lower-left, the head the farthest point, jaws wide open,
    #   forelegs reaching out at the target with the claws spread; hind paws still planted
    Q(planted=HIND, abs={"neck": 0.6, "head": 0.5, "body": 0.4}, q_order=REACH_ORDER, head=(30, 92), snout=(18, 98), jaw=(22, 105), neck=(40, 91),
      withers=(50, 95), spine=(67, 94), hips=(85, 93),
      tail_base=(96, 88), tail_mid=(107, 86), tail_tip=(118, 86),
      fn_top=(48, 103), fn_knee=(37, 109), fn_paw=(27, 114), ff_top=(44, 100), ff_knee=(33, 103), ff_paw=(23, 107),
      hn_top=(86, 100), hn_knee=(97, 110), hf_top=(81, 98), hf_knee=(92, 107),
      wn_root=(57, 86), wn_wrist=(76, 72), wn_tip=(104, 76), wf_root=(51, 85), wf_wrist=(68, 68), wf_tip=(94, 70)),
    # 5 follow-through: the forepaws land ahead at the lower-left, the head bites down low, hind legs drawn up
    Q(abs={"neck": 0.5, "head": 0.4, "body": 0.3}, head=(33, 103), snout=(21, 110), jaw=(24, 114), neck=(43, 98),
      withers=(53, 100), spine=(70, 96), hips=(88, 92),
      tail_base=(99, 86), tail_mid=(108, 88), tail_tip=(116, 96),
      fn_top=(52, 108), fn_knee=(42, 115), fn_paw=(33, 124), ff_top=(47, 106), ff_knee=(37, 112), ff_paw=(28, 120),
      hn_top=(88, 99), hn_knee=(96, 110), hn_paw=(98, 123), hf_top=(84, 97), hf_knee=(92, 107), hf_paw=(92, 119),
      wn_root=(60, 90), wn_wrist=(78, 76), wn_tip=(104, 78), wf_root=(54, 89), wf_wrist=(70, 72), wf_tip=(96, 72)),
    # 6 recover: halfway back, head up, forepaws stepping back
    Q(abs={"neck": 0.2}, head=(39, 82), snout=(26, 88), jaw=(29, 91), neck=(50, 87),
      withers=(59, 93), spine=(77, 91), hips=(96, 89),
      tail_base=(107, 84), tail_mid=(116, 90), tail_tip=(121, 100),
      fn_top=(58, 101), fn_knee=(53, 112), fn_paw=(48, 124), ff_top=(53, 99), ff_knee=(48, 109), ff_paw=(43, 120),
      hn_top=(96, 96), hn_knee=(103, 109), hn_paw=(100, 123), hf_top=(91, 94), hf_knee=(97, 106), hf_paw=(94, 119),
      wn_root=(66, 83), wn_wrist=(82, 58), wn_tip=(108, 53), wf_root=(60, 82), wf_wrist=(70, 54), wf_tip=(94, 45)),
    # 7 settle (a warp of the sprite): almost the rest pose
    shift(Q(head=(42, 80), snout=(29, 86), jaw=(32, 89)), 1, 0, BODY_KEYS),
]

FLINCH = [
    # 0 impact: hit from the lower-left; head tossed up and back, eyes shut, jaws open; body jolts back, forepaws up
    Q(planted=HIND, abs={"neck": 0.5, "head": 0.5}, head=(49, 71), snout=(42, 60), jaw=(38, 67), neck=(58, 80),
      withers=(68, 87), spine=(86, 87), hips=(103, 87),
      tail_base=(114, 84), tail_mid=(119, 93), tail_tip=(117, 104),
      fn_top=(67, 95), fn_knee=(65, 107), fn_paw=(62, 118), ff_top=(62, 93), ff_knee=(59, 104), ff_paw=(56, 114),
      hn_top=(103, 94), hn_knee=(110, 107), hf_top=(98, 92), hf_knee=(104, 105),
      wn_root=(74, 78), wn_wrist=(92, 58), wn_tip=(118, 56), wf_root=(68, 77), wf_wrist=(80, 52), wf_tip=(104, 46),
      eyes="shut"),
    # 1 recoil extreme: half-rearing back toward the upper-right, head still back, tail tucked
    Q(planted=HIND, abs={"neck": 0.5, "head": 0.5}, head=(54, 67), snout=(48, 56), jaw=(43, 63), neck=(63, 76),
      withers=(72, 83), spine=(89, 86), hips=(105, 89),
      tail_base=(116, 88), tail_mid=(120, 97), tail_tip=(117, 107),
      fn_top=(71, 91), fn_knee=(68, 102), fn_paw=(65, 112), ff_top=(66, 89), ff_knee=(62, 99), ff_paw=(59, 108),
      hn_top=(105, 96), hn_knee=(113, 108), hf_top=(100, 94), hf_knee=(107, 106),
      wn_root=(78, 75), wn_wrist=(96, 56), wn_tip=(122, 58), wf_root=(72, 74), wf_wrist=(84, 50), wf_tip=(108, 48),
      eyes="shut"),
    # 2 recovering: halfway back down, head coming forward
    Q(head=(45, 77), snout=(33, 81), jaw=(35, 86), neck=(57, 83),
      withers=(67, 90), spine=(85, 89), hips=(102, 88),
      tail_base=(113, 83), tail_mid=(120, 91), tail_tip=(123, 102),
      fn_top=(66, 98), fn_knee=(63, 110), fn_paw=(60, 123), ff_top=(61, 96), ff_knee=(58, 107), ff_paw=(55, 119),
      hn_top=(102, 94), hn_knee=(108, 108), hf_top=(97, 92), hf_knee=(102, 105),
      wn_root=(72, 81), wn_wrist=(87, 56), wn_tip=(113, 51), wf_root=(66, 80), wf_wrist=(75, 52), wf_tip=(99, 43)),
    # 3 settle (a warp of the sprite): nearly the rest pose
    shift(Q(), 1, 0, BODY_KEYS),
]

DEATH = [
    dict(FLINCH[0]),
    # 1 stagger back: thrown up and back onto the hind legs, forelegs dangling, head thrown back
    Q(planted=HIND, abs={"neck": 0.5, "head": 0.5}, head=(57, 64), snout=(53, 53), jaw=(48, 60), neck=(66, 73),
      withers=(75, 81), spine=(91, 85), hips=(107, 90),
      tail_base=(118, 89), tail_mid=(122, 99), tail_tip=(119, 109),
      fn_top=(74, 89), fn_knee=(73, 100), fn_paw=(70, 109), ff_top=(69, 87), ff_knee=(67, 97), ff_paw=(64, 105),
      hn_top=(107, 97), hn_knee=(114, 109), hf_top=(102, 95), hf_knee=(109, 107),
      wn_root=(80, 73), wn_wrist=(100, 60), wn_tip=(124, 66), wf_root=(74, 72), wf_wrist=(88, 52), wf_tip=(112, 54),
      eyes="shut"),
    # 2 legs buckle: the forepaws come down and fold, the haunches drop, head drooping
    Q(abs={"neck": 0.4}, head=(52, 99), snout=(40, 105), jaw=(43, 108), neck=(62, 95),
      withers=(72, 98), spine=(89, 100), hips=(106, 101),
      tail_base=(117, 96), tail_mid=(124, 104), tail_tip=(128, 113),
      fn_top=(71, 106), fn_knee=(64, 116), fn_paw=(62, 123), ff_top=(66, 104), ff_knee=(59, 112), ff_paw=(57, 119),
      hn_top=(106, 107), hn_knee=(114, 116), hn_paw=(104, 123), hf_top=(101, 105), hf_knee=(109, 113),
      hf_paw=(99, 119),
      wn_root=(78, 90), wn_wrist=(96, 74), wn_tip=(122, 80), wf_root=(72, 89), wf_wrist=(86, 68), wf_tip=(110, 70),
      eyes="shut"),
    # 3 collapse onto the belly, legs folded, head sinking toward the ground
    Q(planted=(), abs={"leg": 0.7, "neck": 0.7, "head": 0.5, "body": 0.5}, ground="body", head=(51, 108), snout=(39, 114), jaw=(42, 117), neck=(62, 104),
      withers=(72, 107), spine=(89, 108), hips=(106, 107),
      tail_base=(117, 102), tail_mid=(126, 107), tail_tip=(133, 114),
      fn_top=(70, 114), fn_knee=(61, 119), fn_paw=(52, 122), ff_top=(66, 112), ff_knee=(57, 116), ff_paw=(48, 119),
      hn_top=(106, 113), hn_knee=(116, 119), hn_paw=(104, 123), hf_top=(101, 111), hf_knee=(111, 116),
      hf_paw=(99, 120),
      wn_root=(78, 99), wn_wrist=(98, 88), wn_tip=(124, 96), wf_root=(72, 98), wf_wrist=(88, 82), wf_tip=(112, 86),
      eyes="shut"),
    # 4 rolls onto its side, away from the viewer: the body sinks flat, the legs straighten and swing out stiff
    Q(planted=(), abs=0.7, ground="body", q_order=LYING_ORDER, ear="flat",
      head=(51, 109), snout=(39, 114), jaw=(42, 118), neck=(62, 106),
      withers=(75, 104), spine=(93, 103), hips=(111, 101),
      tail_base=(121, 98), tail_mid=(130, 102), tail_tip=(138, 108),
      fn_top=(72, 111), fn_knee=(64, 117), fn_paw=(56, 122), ff_top=(77, 112), ff_knee=(71, 118), ff_paw=(65, 123),
      hn_top=(110, 108), hn_knee=(119, 113), hn_paw=(127, 119), hf_top=(105, 109), hf_knee=(112, 115),
      hf_paw=(118, 122),
      wn_root=(82, 96), wn_wrist=(102, 90), wn_tip=(128, 98), wf_root=(76, 95), wf_wrist=(92, 82), wf_tip=(116, 84),
      eyes="shut"),
    # 5 hits the ground: lying flat on its side, the four legs stretched out stiff and straight (forelegs forward,
    #   hind legs back), the head flat on the ground with the ear laid back and the jaw slack, the tail flat behind
    Q(planted=(), abs=1.0, ground="body", q_order=LYING_ORDER, ear="flat",
      head=(50, 113), snout=(37, 117), jaw=(41, 121), neck=(63, 109),
      withers=(76, 108), spine=(94, 106), hips=(112, 104),
      tail_base=(123, 101), tail_mid=(132, 104), tail_tip=(141, 108),
      fn_top=(72, 114), fn_knee=(61, 118), fn_paw=(50, 122), ff_top=(78, 115), ff_knee=(70, 120), ff_paw=(62, 125),
      hn_top=(112, 110), hn_knee=(123, 114), hn_paw=(134, 118), hf_top=(106, 112), hf_knee=(115, 118),
      hf_paw=(124, 124),
      wn_root=(82, 100), wn_wrist=(104, 96), wn_tip=(132, 104), wf_root=(76, 99), wf_wrist=(94, 88), wf_tip=(120, 90),
      eyes="shut"),
    # 6 bounce: the head and the stiff legs lift a little off the ground
    Q(planted=(), abs=1.0, ground="body", q_order=LYING_ORDER, ear="flat",
      head=(50, 110), snout=(37, 113), jaw=(40, 118), neck=(63, 107),
      withers=(76, 107), spine=(94, 105), hips=(112, 103),
      tail_base=(123, 100), tail_mid=(132, 102), tail_tip=(141, 105),
      fn_top=(72, 113), fn_knee=(61, 116), fn_paw=(50, 119), ff_top=(78, 114), ff_knee=(70, 119), ff_paw=(62, 123),
      hn_top=(112, 109), hn_knee=(123, 112), hn_paw=(134, 115), hf_top=(106, 111), hf_knee=(115, 116),
      hf_paw=(124, 122),
      wn_root=(82, 99), wn_wrist=(104, 94), wn_tip=(132, 100), wf_root=(76, 98), wf_wrist=(94, 86), wf_tip=(120, 87),
      eyes="shut"),
]
for _p in DEATH[3:]:  # the fall carries the body 8 px back from where it stood, not further (canvas room)
    shift(_p, -4, 0)
DEATH.append(dict(DEATH[5]))  # 7 corpse: still, as it landed

POSES = {"idle": IDLE, "strike": STRIKE, "flinch": FLINCH, "death": DEATH}


def as_json():
    out = {"canvas": [160, 128], "anchor": [80, 126], "body_plan": "quadruped", "facing": "lower-left", "anims": {}}
    for anim, frames in POSES.items():
        out["anims"][anim] = [{k: (list(v) if isinstance(v, tuple) else v) for k, v in p.items()} for p in frames]
    return out


if __name__ == "__main__":
    for anim, fr in POSES.items():
        for i, p in enumerate(fr):
            assert set(REST) <= set(p), (anim, i)
    assert [len(POSES[a]) for a in ("idle", "strike", "flinch", "death")] == [4, 8, 4, 8]
    (ROOT / "poses_quad.json").write_text(json.dumps(as_json(), indent=1))
    print("wrote", ROOT / "poses_quad.json")
