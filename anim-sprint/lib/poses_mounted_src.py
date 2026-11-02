"""Source of poses_mounted.json: the mounted pose set, a rider on a horse (the roster's front three-quarter view), both
looking LOWER-LEFT.

Same canvas, frame counts and timings as poses.json. Joints as lib/quad.py documents them: the mount's quadruped joints
with the prefix m_ (m_head, m_snout, m_jaw, m_neck, m_withers, m_spine, m_hips, m_tail_*, m_fn/ff/hn/hf _top/_knee/_paw)
and the rider's biped upper body (head, neck, chest, pelvis, r_sh r_el r_ha tip = the weapon arm, FAR side and
screen-left; l_sh l_el l_ha = the rein or shield arm, NEAR side; shield; leg_kn leg_ft = his near leg on the flank).

  python lib/poses_mounted_src.py     # writes poses_mounted.json
  python lib/quad.py --poses poses_mounted.json --out work/quad/src_mounted.png

The strike is a pair of tracks: the mount's (rear | surge | stand) and the rider's (swing | thrust | shoot). The rider's
track is written in the saddle's frame (as if the mount stood still) and carried by the mount's barrel (it follows the
barrel's position and turn), so any pair composes: "strikes" holds all nine as "<rider>+<mount>"; "anims"."strike" is
swing+rear. A unit file picks with "rider_action" (and "mount_action", which defaults to rear for swing, surge for
thrust, stand for shoot).

  mount rear   1 gathers on the haunches, head up; 2 rears (wind-up extreme), forelegs folded high, head up;
               3 plunges forward, forelegs reaching to the lower-left; 4 (hit) forehooves land ahead at the lower-left,
               the forehand low, the head stretched forward; 5 weight on the forehand; 6 steps back halfway.
  mount surge  1 sinks back on the haunches; 2 coiled back; 3 springs forward, forelegs reaching; 4 (hit) lunged far
               forward and low, neck stretched out at the lower-left; 5 holds; 6 steps back halfway.
  mount stand  small shifts only: back a touch on 2 (the draw), forward a touch on 3-4.
  rider swing  1 the weapon lifted over the head; 2 cocked behind the head, leaning back; 3 sweeps over the top, level;
               4 (hit) arm extended to the lower-left, the weapon chopping down past the mount's head; 5 follows on
               down; 6 comes back up.
  rider thrust 1 the weapon lowered level, couched; 2 drawn back, leaning in; 3 driving forward; 4 (hit) arm and
               weapon thrust out along the attack line to the lower-left, leaning far forward over the neck; 5 holds,
               the point dipping; 6 rises back.
  rider shoot  as poses_shot: 1 nock; 2 full draw aimed high; 3 the aim drops onto the target; 4 (hit) release, the
               bow arm locked toward the lower-left, the draw hand flung back; 5 recoil; 6 recover. r_ha = the bow grip,
               tip = the top of the bow's upper limb, l_ = the draw arm.
  flinch       0 impact from the lower-left: the mount throws its head up and shies back to the upper-right, forehooves
               off the ground; the rider jolts back, arms flung out, eyes shut; 1 shies further, half-rearing; 2
               recovering; 3 settle (a warp).
  death        0 = flinch 0; 1 the mount rears back, the rider thrown back in the saddle; 2 the mount's forelegs fold and
               it drops onto its knees, the rider pitching back in the saddle; 3 the mount sinks onto its chest, the
               rider sags, arms dropping; 4 down on its belly with the legs folded under, the neck lowering forward;
               the rider slumps forward over the neck, the weapon slipping from his hand; 5 lying collapsed on its
               belly, its head on the ground toward the lower-left (the facing side); the rider slumped forward over
               the neck, arms hanging, the weapon on the ground beside the head; 6 a tiny settle (the rider's shoulders
               and the mount's head lift a hair); 7 = 5, held.
               The first version rolled the mount onto its side and laid the rider on the ground in front of it; the
               local model (FLUX Klein + the pose LoRA) redrew that as the standing horseman in every try, so the
               mount now stays on its belly and the rider in the saddle. A fused rider (a centaur) slumps the same way.
The rider stays in the saddle (carried) in every frame of every animation ("rider_free", which places a thrown rider
by his own displacement, is supported by lib/quad.py but no longer used here). On strike frames that aim (AIM), the
weapon's final angle is set after the lean and the mount's tilt are composed.
"""
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

M = dict(  # the mount at rest: a horse in front three-quarter view, head at the lower-left
    head=(52, 67), snout=(43, 83), jaw=(47, 85), neck=(63, 75),
    withers=(68, 88), spine=(83, 89), hips=(102, 86),
    tail_base=(109, 81), tail_mid=(113, 91), tail_tip=(114, 104),
    fn_top=(68, 97), fn_knee=(68, 109), fn_paw=(66, 121),
    ff_top=(62, 95), ff_knee=(60, 106), ff_paw=(60, 116),
    hn_top=(103, 94), hn_knee=(107, 106), hn_paw=(103, 120),
    hf_top=(93, 93), hf_knee=(93, 104), hf_paw=(89, 115),
)
R = dict(  # the rider at rest, in the saddle
    head=(80, 46), neck=(80, 55), chest=(80, 62), pelvis=(80, 75),
    r_sh=(73, 58), r_el=(69, 66), r_ha=(66, 70), tip=(58, 34),
    l_sh=(88, 58), l_el=(91, 66), l_ha=(87, 73), shield=(93, 68, 7, 9),
    leg_kn=(88, 85), leg_ft=(88, 97),
)
ALL = ("ff", "hf", "fn", "hn")
HIND = ("hf", "hn")
FRONT_KEYS = ("head", "snout", "jaw", "neck", "withers", "fn_top", "ff_top")


def rot_pts(p, keys, piv, deg):
    """Turn joints about piv; deg > 0 lifts the front (the screen-left end) of a lower-left mount."""
    t = math.radians(deg)
    c, s = math.cos(t), math.sin(t)
    for k in keys:
        x, y = p[k][0] - piv[0], p[k][1] - piv[1]
        p[k] = (round(piv[0] + c * x - s * y, 1), round(piv[1] + s * x + c * y, 1))
    return p


def mv(p, dx, dy, keys=None):
    for k in keys or [k for k in p if isinstance(p[k], tuple) and len(p[k]) == 2]:
        p[k] = (p[k][0] + dx, p[k][1] + dy)
    return p


def leg(p, name, knee_off, paw_off):
    """Place a leg relative to its (already placed) top."""
    t = p[f"{name}_top"]
    p[f"{name}_knee"] = (t[0] + knee_off[0], t[1] + knee_off[1])
    k = p[f"{name}_knee"]
    p[f"{name}_paw"] = (k[0] + paw_off[0], k[1] + paw_off[1])
    return p


def m(**kw):
    p = dict(M)
    p.update(kw)
    return p


def r(**kw):
    p = dict(R)
    p.update(kw)
    return p


BODY = ("head", "snout", "jaw", "neck", "withers", "spine", "hips", "tail_base", "tail_mid", "tail_tip",
        "fn_top", "ff_top", "hn_top", "hf_top")
UPPER = ("head", "neck", "chest", "r_sh", "r_el", "r_ha", "tip", "l_sh", "l_el", "l_ha", "shield")


def rider_lean(p, deg, piv=None):
    """Lean the rider's upper body about the pelvis (deg > 0 leans back toward the upper-right)."""
    piv = piv or p["pelvis"]
    keys = [k for k in UPPER if k in p and k != "shield"]
    rot_pts(p, keys, piv, deg)
    if "shield" in p:
        cx, cy, rx, ry = p["shield"]
        q = rot_pts({"c": (cx, cy)}, ["c"], piv, deg)["c"]
        p["shield"] = (q[0], q[1], rx, ry)
    return p


# ---------------------------------------------------------------- mount tracks (absolute coordinates)
def gather():
    p = mv(m(), 2, 0, BODY)
    rot_pts(p, ("head", "snout", "jaw"), p["neck"], 10)
    p = mv(p, 0, 3, ("hips", "hn_top", "hf_top", "tail_base", "tail_mid", "tail_tip"))
    leg(p, "hn", (6, 10), (-6, 13)); leg(p, "hf", (3, 10), (-6, 12))
    leg(p, "fn", (0, 12), (-2, 12)); leg(p, "ff", (-2, 11), (0, 10))
    return p


def rear(deg=24):
    p = gather()
    piv = p["hips"]
    rot_pts(p, ("head", "snout", "jaw", "neck", "withers", "spine", "fn_top", "ff_top"), piv, deg)
    rot_pts(p, ("head", "snout", "jaw"), p["neck"], 8)
    leg(p, "fn", (-7, 6), (4, 9)); leg(p, "ff", (-8, 5), (3, 9))   # forelegs folded up, hooves tucked
    leg(p, "hn", (6, 9), (-6, 13)); leg(p, "hf", (4, 9), (-6, 12))
    mv(p, 0, 0)
    return p


def plunge():
    p = mv(m(), -6, -2, BODY)
    rot_pts(p, ("head", "snout", "jaw", "neck", "withers", "spine", "fn_top", "ff_top"), p["hips"], 6)
    rot_pts(p, ("head", "snout", "jaw"), p["neck"], -12)
    leg(p, "fn", (-9, 7), (-8, 8)); leg(p, "ff", (-9, 6), (-8, 7))   # reaching forward
    leg(p, "hn", (2, 12), (2, 13)); leg(p, "hf", (1, 11), (0, 12))
    return p


def land(dx=-9):
    p = mv(m(), dx, 2, BODY)
    rot_pts(p, ("head", "snout", "jaw", "neck", "withers", "spine", "fn_top", "ff_top"), p["hips"], -6)
    rot_pts(p, ("head", "snout", "jaw"), p["neck"], -16)
    leg(p, "fn", (-5, 11), (-4, 11)); leg(p, "ff", (-5, 10), (-4, 9))   # forehooves planted ahead
    leg(p, "hn", (2, 12), (4, 12)); leg(p, "hf", (2, 11), (2, 11))      # hind hooves back where they stood
    return p


def after(dx=-8):
    p = mv(m(), dx, 2, BODY)
    rot_pts(p, ("head", "snout", "jaw", "neck", "withers", "spine", "fn_top", "ff_top"), p["hips"], -3)
    rot_pts(p, ("head", "snout", "jaw"), p["neck"], -10)
    leg(p, "fn", (-2, 12), (-2, 12)); leg(p, "ff", (-2, 11), (-2, 10))
    leg(p, "hn", (3, 12), (1, 13)); leg(p, "hf", (2, 11), (0, 11))
    return p


def back_half(dx=-4):
    p = mv(m(), dx, 1, BODY)
    rot_pts(p, ("head", "snout", "jaw"), p["neck"], -4)
    leg(p, "fn", (0, 12), (-2, 12)); leg(p, "ff", (-1, 11), (-1, 10))
    leg(p, "hn", (4, 12), (-2, 13)); leg(p, "hf", (2, 11), (-3, 11))
    return p


def settle():
    return mv(m(), 1, 0, BODY)


def surge_coil():
    p = mv(m(), 5, 3, BODY)
    rot_pts(p, ("head", "snout", "jaw", "neck", "withers", "spine", "fn_top", "ff_top"), p["hips"], 4)
    rot_pts(p, ("head", "snout", "jaw"), p["neck"], -6)
    leg(p, "fn", (-4, 11), (-5, 10)); leg(p, "ff", (-5, 10), (-5, 9))
    leg(p, "hn", (7, 9), (-8, 12)); leg(p, "hf", (5, 9), (-8, 11))
    return p


def surge_out(dx=-14):
    p = mv(m(), dx, 3, BODY)
    rot_pts(p, ("head", "snout", "jaw", "neck", "withers", "spine", "fn_top", "ff_top"), p["hips"], -5)
    rot_pts(p, ("head", "snout", "jaw"), p["neck"], -26)   # neck stretched out, head toward the target
    leg(p, "fn", (-8, 9), (-6, 10)); leg(p, "ff", (-8, 8), (-6, 8))
    leg(p, "hn", (6, 11), (8, 10)); leg(p, "hf", (5, 10), (6, 9))   # hind legs driving, stretched back
    return p


MOUNT_STRIKE = {
    "rear": [m(), gather(), rear(), plunge(), land(), after(), back_half(), settle()],
    "surge": [m(), mv(gather(), 2, 1, BODY), surge_coil(), plunge(), surge_out(), surge_out(-12), back_half(-5),
              settle()],
    "stand": [m(), mv(m(), 1, 0, BODY), mv(gather(), 1, 0, BODY), mv(m(), -2, 0, BODY), mv(m(), -3, 1, BODY),
              mv(m(), -2, 1, BODY), mv(m(), -1, 0, BODY), settle()],
}
MOUNT_PLANTED = {
    "rear": [ALL, ALL, HIND, HIND, ALL, ALL, ALL, ALL],
    "surge": [ALL, ALL, ALL, HIND, ALL, ALL, ALL, ALL],
    "stand": [ALL] * 8,
}

# ---------------------------------------------------------------- rider tracks (in the saddle frame)
RIDER_STRIKE = {
    "swing": [
        r(),
        rider_lean(r(r_el=(68, 56), r_ha=(72, 47), tip=(86, 18)), 4),
        rider_lean(r(r_el=(72, 51), r_ha=(79, 45), tip=(104, 58), l_el=(93, 64), l_ha=(90, 70)), 10),
        rider_lean(r(r_el=(68, 52), r_ha=(60, 50), tip=(30, 52)), -6),
        rider_lean(r(r_sh=(72, 59), r_el=(64, 64), r_ha=(57, 70), tip=(33, 94), l_el=(94, 64), l_ha=(98, 70)), -14),
        rider_lean(r(r_sh=(72, 59), r_el=(65, 67), r_ha=(60, 76), tip=(44, 106), l_el=(94, 65), l_ha=(97, 71)), -10),
        rider_lean(r(r_el=(68, 63), r_ha=(64, 64), tip=(54, 30)), -3),
        r(r_ha=(66, 71), tip=(57, 35)),
    ],
    "thrust": [
        r(),
        r(r_el=(70, 66), r_ha=(70, 68), tip=(36, 70)),
        rider_lean(r(r_el=(76, 66), r_ha=(76, 66), tip=(42, 72)), -4),
        rider_lean(r(r_el=(70, 65), r_ha=(66, 67), tip=(32, 78)), -10),
        rider_lean(r(r_sh=(72, 59), r_el=(64, 64), r_ha=(57, 68), tip=(25, 86), l_el=(94, 64), l_ha=(97, 70)), -16),
        rider_lean(r(r_sh=(72, 59), r_el=(64, 65), r_ha=(57, 70), tip=(26, 92), l_el=(94, 65), l_ha=(97, 71)), -12),
        rider_lean(r(r_el=(69, 64), r_ha=(65, 64), tip=(48, 38)), -4),
        r(r_ha=(66, 71), tip=(57, 35)),
    ],
    "shoot": [
        r(r_ha=(66, 66), tip=(62, 48)),
        rider_lean(r(r_el=(68, 62), r_ha=(66, 58), tip=(60, 41), l_el=(80, 66), l_ha=(70, 60)), -4),
        rider_lean(r(r_sh=(73, 58), r_el=(65, 54), r_ha=(57, 50), tip=(55, 31), l_el=(95, 55), l_ha=(83, 52)), 8),
        rider_lean(r(r_el=(65, 59), r_ha=(57, 59), tip=(53, 41), l_el=(95, 58), l_ha=(83, 54)), -2),
        rider_lean(r(r_el=(65, 63), r_ha=(56, 67), tip=(49, 50), l_el=(95, 52), l_ha=(102, 45)), -8),
        rider_lean(r(r_el=(65, 64), r_ha=(56, 68), tip=(46, 54), l_el=(95, 50), l_ha=(104, 48)), -6),
        rider_lean(r(r_el=(68, 65), r_ha=(64, 65), tip=(60, 47)), -2),
        r(r_ha=(66, 66), tip=(62, 48)),
    ],
}
def carry(mrest, mnow, pts):
    """Rider joints carried by the mount's barrel from mrest to mnow (origin spine, axis hips -> withers)."""
    o0, o1 = mrest["spine"], mnow["spine"]
    a = lambda p: math.atan2(p["withers"][1] - p["hips"][1], p["withers"][0] - p["hips"][0])
    t = a(mnow) - a(mrest)
    c, s = math.cos(t), math.sin(t)
    out = {}
    for k, v in pts.items():
        if k == "shield":
            x, y = v[0] - o0[0], v[1] - o0[1]
            out[k] = (round(o1[0] + c * x - s * y, 1), round(o1[1] + s * x + c * y, 1), v[2], v[3])
        elif isinstance(v, tuple) and len(v) == 2 and isinstance(v[0], (int, float)):
            x, y = v[0] - o0[0], v[1] - o0[1]
            out[k] = (round(o1[0] + c * x - s * y, 1), round(o1[1] + s * x + c * y, 1))
        else:
            out[k] = v
    return out


def compose(mount, rider, carried=True, **extra):
    p = {f"m_{k}": v for k, v in mount.items()}
    p.update(carry(M, mount, rider) if carried else rider)
    p.update(extra)
    return p


# The weapon's final screen angle on the frames that aim at the target (degrees below level, pointing to the
# lower-left; the target tile lies 27 degrees below level), set after the rider's lean and the mount's tilt are
# composed, so the blow lands on the attack line whichever mount track carries it.
AIM = {("thrust", 3): 20, ("thrust", 4): 27, ("thrust", 5): 33, ("swing", 3): 4, ("swing", 4): 42, ("swing", 5): 62}


def aim(p, deg):
    hx, hy = p["r_ha"]
    L = math.dist(p["r_ha"], p["tip"])
    a = math.radians(180 - deg)
    p["tip"] = (round(hx + L * math.cos(a), 1), round(hy + L * math.sin(a), 1))
    return p


def strike(rider_action, mount_action):
    fr = []
    for i in range(8):
        p = compose(MOUNT_STRIKE[mount_action][i], RIDER_STRIKE[rider_action][i], planted=list(MOUNT_PLANTED[mount_action][i]))
        if (rider_action, i) in AIM:
            aim(p, AIM[rider_action, i])
        if i in (3, 4) and mount_action in ("rear", "surge"):
            p["q_order"] = ["ff", "hf", "tail", "body", "hn", "neck", "head", "fn"]
        if i == 4:
            p["abs"] = {"neck": 0.4, "head": 0.4}
        fr.append(p)
    return fr


# ---------------------------------------------------------------- idle, flinch, death
def breathe(dy, tail_dx=0):
    p = mv(m(), 0, dy, ("head", "snout", "jaw", "neck", "withers", "spine"))
    p["tail_tip"] = (p["tail_tip"][0] + tail_dx, p["tail_tip"][1])
    return p


IDLE = [compose(m(), r()),
        compose(breathe(-1), mv(r(), 0, -1, UPPER[:-1])),
        compose(breathe(-1, 1), mv(r(tip=(58, 33)), 0, -1, UPPER[:-1])),
        compose(breathe(0, 1), r())]


def shy(dx, deg, head_deg):
    p = mv(m(), dx, 0, BODY)
    rot_pts(p, ("head", "snout", "jaw", "neck", "withers", "spine", "fn_top", "ff_top"), p["hips"], deg)
    rot_pts(p, ("head", "snout", "jaw"), p["neck"], head_deg)
    p = mv(p, 0, 2, ("hips", "hn_top", "hf_top"))
    leg(p, "fn", (-3, 10), (1, 9)); leg(p, "ff", (-4, 9), (1, 9))
    leg(p, "hn", (6, 10), (-6, 12)); leg(p, "hf", (4, 10), (-6, 11))
    return p


JOLT = rider_lean(r(r_el=(66, 56), r_ha=(64, 48), tip=(56, 18), l_el=(97, 60), l_ha=(103, 56), shield=(104, 56, 7, 9),
                    head=(83, 46)), 12)
FLINCH = [
    compose(shy(4, 10, 22), JOLT, planted=list(HIND), eyes="shut", head_tilt=14),
    compose(shy(6, 15, 26), rider_lean(r(r_el=(67, 58), r_ha=(63, 52), tip=(52, 24), l_el=(97, 62), l_ha=(102, 60),
                                         shield=(103, 60, 7, 9)), 14), planted=list(HIND), eyes="shut", head_tilt=10),
    compose(mv(m(), 3, 0, BODY), rider_lean(r(), 5), head_tilt=5),
    compose(mv(m(), 1, 0, BODY), r(), head_tilt=2),
]


def kneel():
    p = mv(m(), 4, 10, ("head", "snout", "jaw", "neck", "withers", "fn_top", "ff_top"))
    rot_pts(p, ("head", "snout", "jaw"), p["neck"], -14)
    p = mv(p, 4, 5, ("spine", "hips", "hn_top", "hf_top", "tail_base", "tail_mid", "tail_tip"))
    leg(p, "fn", (-6, 8), (8, 4)); leg(p, "ff", (-6, 7), (8, 3))      # forelegs folded under, on the knees
    leg(p, "hn", (7, 9), (-5, 12)); leg(p, "hf", (5, 9), (-5, 11))
    return p


def chest_down():
    p = mv(m(), 5, 17, BODY)
    rot_pts(p, ("head", "snout", "jaw"), p["neck"], -18)
    leg(p, "fn", (-8, 5), (-9, 2)); leg(p, "ff", (-8, 4), (-9, 2))
    leg(p, "hn", (9, 6), (-7, 6)); leg(p, "hf", (8, 5), (-7, 5))
    return p


def belly(stage, lift=0):
    """Collapsed on its belly with the legs folded under it (the image model draws this; it could not draw a mount
    rolled onto its side with a rider lying beside it). stage 0: the neck lowering forward; 1: the head lying on the
    ground toward the lower-left. lift raises the head a little (the bounce)."""
    p = mv(m(), 5, 19, BODY)
    leg(p, "fn", (-8, 4), (9, 2)); leg(p, "ff", (-8, 3), (9, 2))        # forelegs folded under the chest
    leg(p, "hn", (8, 5), (-12, 3)); leg(p, "hf", (7, 4), (-11, 3))      # hind legs folded under the belly
    p.update(tail_mid=(120, 110), tail_tip=(127, 117))                  # the tail lies on the ground
    if stage == 0:
        p.update(neck=(67, 97), head=(55, 97), snout=(45, 109), jaw=(48, 111))
    else:
        p.update(neck=(66, 100), head=(51, 108 - lift), snout=(38, 117 - lift), jaw=(42, 119 - lift))
    return p


def slump(lean, droop=0, weapon=115, arms_up=0):
    """The rider gone limp in the saddle: the head, neck and chest folded forward over the mount's neck by `lean`
    degrees about the pelvis, the head drooping `droop` px further; the shoulders close in around the chest as he
    folds (a 2D turn of the rest shoulders would lift the near one above his back), both arms hanging straight down.
    weapon: the held weapon's screen angle in degrees (slipping, pointing down toward the lower-left), or None when it
    has fallen (the frame then gives it lying on the ground as `sword`)."""
    core = rider_lean({k: R[k] for k in ("head", "neck", "chest", "pelvis")}, -lean)
    p = r(**core)
    t = min(1.0, lean / 60)
    ch = p["chest"]
    rest_off = {"r": (R["r_sh"][0] - R["chest"][0], R["r_sh"][1] - R["chest"][1]),
                "l": (R["l_sh"][0] - R["chest"][0], R["l_sh"][1] - R["chest"][1])}
    slump_off = {"r": (-4, -3), "l": (4, 3)}  # far shoulder up-left of the chest, near shoulder down toward the viewer
    for side, dx in (("r", -1.0), ("l", 1.0)):
        o = tuple(rest_off[side][i] + (slump_off[side][i] - rest_off[side][i]) * t for i in (0, 1))
        sh = (round(ch[0] + o[0], 1), round(ch[1] + o[1], 1))
        p[f"{side}_sh"] = sh
        p[f"{side}_el"] = (sh[0] + dx, sh[1] + 8 - arms_up)
        p[f"{side}_ha"] = (sh[0] + dx * 1.5, sh[1] + 16 - arms_up * 2)
    p["shield"] = (p["l_ha"][0] + 3, p["l_ha"][1] - 2, 7, 9)
    p["head"] = (p["head"][0], p["head"][1] + droop)
    if weapon is None:
        del p["tip"]
    else:
        L = math.dist(R["r_ha"], R["tip"])
        a = math.radians(weapon)
        p["tip"] = (round(p["r_ha"][0] + L * math.cos(a), 1), round(p["r_ha"][1] + L * math.sin(a), 1))
    return p


# the weapon fallen from the rider's hand, lying on the ground beside the mount's head (lower-left)
DROPPED = ((50, 124), (26, 125))
SLUMPED = dict(planted=[], abs={"leg": 0.8, "neck": 0.7, "head": 0.6, "body": 0.5}, ground="body", eyes="shut")

DEATH = [
    dict(FLINCH[0]),
    # 1 the mount rears back, the rider thrown back in the saddle, arms flung up
    compose(shy(8, 22, 30), rider_lean(r(r_el=(64, 54), r_ha=(60, 46), tip=(48, 18), l_el=(98, 58), l_ha=(105, 52),
                                         shield=(106, 52, 7, 9)), 20), planted=list(HIND), eyes="shut", head_tilt=20),
    # 2 the forelegs fold and it drops onto its knees; the rider pitches back in the saddle, arms up
    compose(kneel(), rider_lean(r(r_el=(66, 52), r_ha=(62, 44), tip=(54, 14), l_el=(98, 58), l_ha=(104, 52),
                                  shield=(105, 52, 7, 9), leg_kn=(86, 84), leg_ft=(82, 94)), 32),
            eyes="shut", head_tilt=30, abs={"neck": 0.4}),
    # 3 it sinks onto its chest; the rider sags in the saddle, arms dropping
    compose(chest_down(), slump(12, droop=1, weapon=112, arms_up=4), **SLUMPED, head_tilt=-15),
    # 4 down on its belly, legs folded under, the neck lowering forward; the rider slumps forward over the neck, the
    #   weapon slipping from his hand
    compose(belly(0), slump(45, droop=4, weapon=128), **SLUMPED, head_tilt=-50),
    # 5 lying collapsed on its belly, the head on the ground toward the lower-left; the rider slumped forward over the
    #   neck, arms hanging, the weapon on the ground beside the head
    compose(belly(1), slump(74, droop=9, weapon=None), sword=DROPPED, **SLUMPED, ear="flat", head_tilt=-85),
    # 6 bounce: the rider's shoulders lift a hair, the mount's head lifts a little
    compose(belly(1, lift=1), slump(70, droop=8, weapon=None), sword=DROPPED, **SLUMPED, ear="flat",
            head_tilt=-80),
]
DEATH.append(dict(DEATH[5]))

STRIKES = {f"{ra}+{ma}": strike(ra, ma) for ra in RIDER_STRIKE for ma in MOUNT_STRIKE}


SOURCE_RIDER = {"head_r": 8, "limb": 0.9}  # the source rider's head and limbs as drawn in the pose sheet; a unit's own
#                                          head_r and limb replace them in its guides


def jsonable(p):
    q = dict(SOURCE_RIDER)
    for k, v in p.items():
        if k == "sword":
            q[k] = [list(v[0]), list(v[1])]
        elif isinstance(v, tuple):
            q[k] = list(v)
        else:
            q[k] = v
    return q


def as_json():
    anims = {"idle": IDLE, "strike": STRIKES["swing+rear"], "flinch": FLINCH, "death": DEATH}
    return {"canvas": [160, 128], "anchor": [80, 126], "body_plan": "mounted", "facing": "lower-left",
            "anims": {a: [jsonable(p) for p in fr] for a, fr in anims.items()},
            "strikes": {k: [jsonable(p) for p in fr] for k, fr in STRIKES.items()}}


if __name__ == "__main__":
    data = as_json()
    for a, fr in list(data["anims"].items()) + list(data["strikes"].items()):
        for i, p in enumerate(fr):
            for k in list(M):
                assert f"m_{k}" in p, (a, i, k)
            for k in R:
                assert k in p or (k == "tip" and "sword" in p), (a, i, k)
    (ROOT / "poses_mounted.json").write_text(json.dumps(data, indent=1))
    print("wrote", ROOT / "poses_mounted.json")
