"""Quadruped and mounted rigs for the recipe: grey pose mannequins for four-legged units (wolves, lions, bears, dragons
on four legs, spiders with extra legs) and for riders on mounts (a horse, wolf, boar, camel or bison; centaurs), in the
flat-grey style of guides.render_guide, and the retarget of a source pose set onto a unit's own frame-0 skeleton.

  python lib/quad.py --unit units_draft/wolf.json --check-guides          # work/quad/wolf_guides.png
  python lib/quad.py --unit units_draft/horseman.json --check-guides      # work/quad/horseman_guides.png
  python lib/quad.py --unit units_draft/horseman.json --check-guides --action thrust   # another rider action
  python lib/quad.py --unit units_draft/wolf.json --show-beats            # the texts each sheet cell gets
  python lib/quad.py --poses poses_quad.json --out work/quad/src_quad.png  # every frame of a source set (--wings)

What the recipe needs from here for these body plans (recipe.py is not wired to it yet): unit_poses(u) in place of
recipe.unit_poses, render_guide(pose) in place of guides.render_guide (sheet templates, x_snap, perframe),
warp_to_pose(base, rest, pose) in place of rig.warp_to_pose (idle, strike 7, flinch 3), beat_text(u, anim, i) in place
of recipe.beat_text, and load_unit(path) in place of recipe.load_unit (it validates these joints instead of the biped's).

Unit file (units_draft/wolf.json, units_draft/horseman.json are worked examples):
  name, sprite, facing ("lower-left" | "lower-right": where the head / snout looks), tag, description, hands (for the
  prompt's hands sentence: "it has no weapon: it fights with its fangs and claws"), guide_style "grey",
  body_plan        "quadruped" | "mounted"
  joints           quadruped: a flat dict of QUAD_JOINTS (+ optional ones); mounted: {"mount": {...QUAD_JOINTS},
                   "rider": {...RIDER_JOINTS}}; canvas coordinates [x, y] placed on the sprite as drawn
  dims             the mannequin's widths in px (body_r, head_r, neck_r, snout_w, leg_w, paw_r, tail_w, tail_tuft,
                   ear (0 = none, 1 = a wolf's), wing_w); missing ones are the source's times the unit's scale
  scale            optional; default: the unit's body and front-leg length over the source's
  words            the beat texts' generic words: quadruped {"jaws", "claws", "paws", "tail", "ears"}; mounted
                   {"mount", "rider", "weapon", "feet", "his", "him"}
  beats            own texts per cell, as in recipe unit files (written for the unit's real facing)
  mounted only:    rider_action "swing" (overhand chop or stab) | "thrust" (couched lance, spear) | "shoot" (bow; r_ha =
                   the grip, tip = the top of the upper limb, l_ = the draw arm); mount_action "rear" | "surge" |
                   "stand" (default: rear, surge, stand by rider action); head_r and limb size the rider as in the
                   biped files; rider_far ("r" | "l", default from the shoulders: the arm on the side the rider looks
                   to is the far one); fused: true for a centaur (the torso grows from the mount: always carried,
                   never thrown, no rider leg)
  quadruped only:  extra_legs {"m1n": 0.33, ...} for spiders; wings via the wn_/wf_ joints

Body plans:

  quadruped  joints = the QUAD_JOINTS below (flat dict), optional tail_mid (a curled tail), wings (wn_/wf_ root, wrist,
             tip) and extra legs; source set poses_quad.json (lib/poses_quad_src.py, texts lib/beats_quad.py).
  mounted    joints = {"mount": {QUAD_JOINTS}, "rider": {RIDER_JOINTS}} (stored flat: the mount's joints get the
             prefix "m_"); source set poses_mounted.json (lib/poses_mounted_src.py, texts lib/beats_mounted.py), whose
             strike is picked by "rider_action" and "mount_action".

Both source sets look LOWER-LEFT; a unit that looks lower-right gets them mirrored (x -> 159 - x), as recipe.pose_set
does for the biped sets. Legs are named by depth, not by left/right, so mirroring keeps them: f/h = fore/hind,
n/f = near (the side toward the viewer, drawn lighter and in front) / far (drawn darker, behind the body):

  head        centre of the skull (between the eye and the ear)
  snout       tip of the nose / upper jaw (beak tip, trunk tip)
  jaw         tip of the lower jaw (open jaws: below the snout line; the mannequin shows the mouth dark when the gape
              passes 20 degrees)
  neck        base of the neck, where it leaves the forequarters
  withers     centre of the forequarters (the shoulder mass the front legs hang from)
  spine       centre of the barrel, mid-body (the root of the skeleton)
  hips        centre of the hindquarters
  tail_base   where the tail leaves the rump; tail_mid (optional) its bend; tail_tip its end
  fn_top fn_knee fn_paw    front near leg: elbow (top of the visible leg), knee/wrist, paw or hoof (the ankle)
  ff_*, hn_*, hf_*         front far, hind near, hind far (hind: top = the haunch/stifle, knee = the hock)
  wn_root wn_wrist wn_tip  optional near wing: where it leaves the back, its front bend, its outer tip (wf_*: far)
  extra legs  "extra_legs": {"m1n": 0.33, "m1f": 0.33, ...}: <name>_top/_knee/_paw joints that move as a blend of the
              same side's front (0) and hind (1) legs (spiders, scorpions)

RIDER_JOINTS are the biped's upper body (recipe.JOINTS without the legs): head, neck, chest, pelvis (the seat), r_sh
r_el r_ha tip (the weapon arm, r_ha -> tip along the weapon), l_sh l_el l_ha, optional shield [cx, cy, rx, ry], and the
NEAR leg hanging on the flank: leg_kn, leg_ft (knee, foot in the stirrup). The far leg is hidden by the mount. The rider
is drawn in the biped mannequin's greys with its torso scaled to head_r (render_rider); the mount a shade darker than
a lone animal (MOUNT_GREY) so the two read apart.

Retarget (unit_poses): the skeleton is a tree rooted at the barrel's centre (spine); the root moves by the source's
displacement times the unit's scale, and every bone keeps the unit's own frame-0 length and angle plus the source
bone's turn away from its rest (as recipe.retarget). Then:
  - a leg the source frame lists in "planted" (default: all four) is solved by two-bone IK so its paw lands where the
    unit's own frame-0 paw is, moved by the source paw's displacement (times the scale): feet stay planted whatever
    the unit's proportions; the knee bends to the side the source's knee bends to;
  - a leg in the air, and the rider's weapon arm and weapon, turn toward the source's ABSOLUTE angle as the source
    bone turns further from rest (fully past 60 degrees), like recipe's "drawn" mode, so a sprite whose legs or weapon
    rest at another angle still pounces or strikes where the beat sheet says;
  - a source frame's "abs" ({group: weight} or one weight for all; groups body, neck, head, tail, leg, wing, weapon,
    rider) blends the unit's bones toward the source's absolute angles: the key strike frames use it a little (a
    crouching wolf's low head must not end in the ground) and the death's lying frames fully, so a unit drawn
    sitting (a dragon on its haunches) still ends lying along the ground;
  - the jaw opens relative to the snout: the unit's own gape plus the source's change, kept between 4 and 48 degrees;
  - the rider's pelvis rides on the mount's back (it follows the barrel's position and turn) plus the source rider's
    own offset from the saddle, so he stays on the mount; poses_mounted.json keeps him in the saddle in every frame
    (the death slumps him over the mount's neck). A source frame marked "rider_free" would instead move him by his own
    displacement from the saddle and ground him on his own (a thrown rider; unused: the local model could not draw
    a mount on its side with its rider lying beside it);
  - a source frame with "ground": "body" (lying, falling) is moved up or down so the unit's barrel keeps the source's
    height above the ground; then legs, head and a thrown rider that would go below the ground are turned or lifted
    onto it;
  - a guide that leaves the safe region (x 16-143) by up to 4 px slides back in; bigger overflows are reported.
mode "disp" (idle and settle frames, which are mesh-warps of the real sprite) moves every joint by the source joint's
displacement times the scale, as recipe.retarget's "disp".
"""
import argparse, json, math, sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageOps

sys.path.insert(0, str(Path(__file__).resolve().parent))
import guides  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
W, H = 160, 128
FAR_DARKEN = guides.FAR_DARKEN
SETS = {"quadruped": "poses_quad.json", "mounted": "poses_mounted.json"}
LEG_NAMES = ("ff", "hf", "fn", "hn")
QUAD_JOINTS = ("head", "snout", "jaw", "neck", "withers", "spine", "hips", "tail_base", "tail_tip") + tuple(
    f"{l}_{j}" for l in ("fn", "ff", "hn", "hf") for j in ("top", "knee", "paw"))
WING_JOINTS = tuple(f"{w}_{j}" for w in ("wn", "wf") for j in ("root", "wrist", "tip"))
RIDER_JOINTS = ("head", "neck", "chest", "pelvis", "r_sh", "r_el", "r_ha", "tip", "l_sh", "l_el", "l_ha",
                "leg_kn", "leg_ft")
RIDER_TREE = [("pelvis", "chest"), ("chest", "neck"), ("neck", "head"), ("chest", "r_sh"), ("r_sh", "r_el"),
              ("r_el", "r_ha"), ("r_ha", "tip"), ("chest", "l_sh"), ("l_sh", "l_el"), ("l_el", "l_ha"),
              ("pelvis", "leg_kn"), ("leg_kn", "leg_ft")]
WEAPON_CHAIN = {("r_sh", "r_el"), ("r_el", "r_ha"), ("r_ha", "tip")}
RIDER_ACTIONS = ("swing", "thrust", "shoot")
MOUNT_FOR_ACTION = {"swing": "rear", "thrust": "surge", "shoot": "stand"}

# The source mannequin's proportions (px at 1x); a unit's "dims" override them, the rest scale with the unit.
SRC_DIMS = {
    "quadruped": dict(body_r=11, head_r=7, neck_r=5.5, leg_w=5, paw_r=3, tail_w=4, snout_w=6, ear=1.0, tail_tuft=0,
                      wing_w=3),
    "mounted": dict(body_r=12, head_r=6, neck_r=6, leg_w=4.5, paw_r=2.5, tail_w=4, snout_w=6, ear=0.8, tail_tuft=0,
                    wing_w=3),
}
# Flat greys in the biped mannequin's range (guides.GREY): the body a little darker than the biped's tunic so a rider
# on top reads apart from his mount; far limbs FAR_DARKEN darker, as in the biped guide.
QUAD_GREY = dict(body=(146, 146, 146), leg=(160, 160, 160), neck=(152, 152, 152), head=(172, 172, 172),
                 jaw=(160, 160, 160), paw=(90, 90, 90), tail=(132, 132, 132), ear=(95, 95, 95), mouth=(70, 70, 70),
                 eye=(40, 40, 40), wing=(118, 118, 118), wingbone=(96, 96, 96))
MOUNT_GREY = dict(QUAD_GREY, body=(124, 124, 124), leg=(136, 136, 136), neck=(130, 130, 130), head=(146, 146, 146),
                  jaw=(136, 136, 136), tail=(108, 108, 108), paw=(78, 78, 78))
QUAD_ORDER = ["wf", "ff", "hf", "tail", "body", "wn", "hn", "fn", "neck", "head"]


# ---------------------------------------------------------------- geometry helpers
def _c(rgb, k=1.0):
    return tuple(int(max(0, min(255, v * k))) for v in rgb) + (255,)


def capsule(d, a, b, w, col):
    w = max(1, round(w))
    d.line([tuple(a[:2]), tuple(b[:2])], fill=col, width=w)
    r = (w - 1) / 2
    for p in (a, b):
        d.ellipse([p[0] - r, p[1] - r, p[0] + r, p[1] + r], fill=col)


def disc(d, c, r, col):
    d.ellipse([c[0] - r, c[1] - r, c[0] + r, c[1] + r], fill=col)


def _ang(a, b):
    return math.atan2(b[1] - a[1], b[0] - a[0])


def _wrap(a):
    return (a + math.pi) % (2 * math.pi) - math.pi


def _unit(a, b):
    vx, vy = b[0] - a[0], b[1] - a[1]
    n = math.hypot(vx, vy) or 1.0
    return vx / n, vy / n


def _cross(o, a, b):
    return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])


def _is_pt(v):
    return isinstance(v, (list, tuple)) and len(v) == 2 and all(isinstance(t, (int, float)) for t in v)


# ---------------------------------------------------------------- schema
def legs_of(p, pre=""):
    """Leg names present in a pose (ff, hf, fn, hn and any extra legs), far legs first."""
    names = sorted({k[len(pre):-4] for k in p if k.startswith(pre) and k.endswith("_paw")})
    return sorted(names, key=lambda n: (n[-1] != "f", n))


def is_near(leg):
    return leg.endswith("n")


def quad_tree(p, pre=""):
    """(parent, child) bones of the quadruped skeleton present in pose p, in root-first order (root: spine)."""
    j = lambda k: pre + k
    t = [("spine", "withers"), ("withers", "neck"), ("neck", "head"), ("head", "snout"), ("head", "jaw"),
         ("spine", "hips"), ("hips", "tail_base")]
    t += [("tail_base", "tail_mid"), ("tail_mid", "tail_tip")] if j("tail_mid") in p else [("tail_base", "tail_tip")]
    for leg in legs_of(p, pre):
        parent = {"f": "withers", "h": "hips"}.get(leg[0], "spine")
        t += [(parent, f"{leg}_top"), (f"{leg}_top", f"{leg}_knee"), (f"{leg}_knee", f"{leg}_paw")]
    for w in ("wn", "wf"):
        if j(f"{w}_root") in p:
            t += [("withers", f"{w}_root"), (f"{w}_root", f"{w}_wrist"), (f"{w}_wrist", f"{w}_tip")]
    return [(j(a), j(b)) for a, b in t if j(a) in p and j(b) in p]


def bone_group(a, b, plan="quadruped"):
    """The 'abs' group a bone belongs to: body, neck, head, tail, leg, wing (the animal or the mount), weapon, rider."""
    if plan == "mounted" and not a.startswith("m_"):
        return "weapon" if (a, b) in WEAPON_CHAIN else "rider"
    a0, b0 = a[2:] if a.startswith("m_") else a, b[2:] if b.startswith("m_") else b
    if b0.endswith(("_top",)):
        return "body"
    if b0.endswith(("_knee", "_paw")):
        return "leg"
    if b0.startswith(("wn_", "wf_")):
        return "wing"
    if b0.startswith("tail"):
        return "tail"
    if b0 in ("snout", "jaw"):
        return "head"
    if b0 in ("neck", "head"):
        return "neck"
    return "body"


def flatten_joints(u):
    """A unit file's joints as one flat dict: quadruped as given; mounted {"mount": .., "rider": ..} with the mount's
    joints prefixed "m_" (a flat dict that already has m_ keys is accepted as is)."""
    j = u["joints"]
    if "mount" in j or "rider" in j:
        out = {f"m_{k}": v for k, v in j.get("mount", {}).items()}
        out.update(j.get("rider", {}))
        return out
    return dict(j)


def mirror_pose(p):
    """The pose mirrored about the anchor (x -> W-1-x) and facing the other way; near and far stay as they are."""
    q = dict(p)
    for k, v in p.items():
        if k in ("sword",) and isinstance(v, (list, tuple)) and len(v) == 2 and _is_pt(v[0]):
            q[k] = [[W - 1 - v[0][0], v[0][1]], [W - 1 - v[1][0], v[1][1]]]
        elif k == "shield":
            q[k] = [W - 1 - v[0], v[1], v[2], v[3]]
        elif _is_pt(v):
            q[k] = [W - 1 - v[0], v[1]]
    q["facing"] = "lower-right" if p.get("facing", "lower-left") == "lower-left" else "lower-left"
    return q


# ---------------------------------------------------------------- the quadruped mannequin
def draw_quad(d, p, dims, pal, pre=""):
    """Draw the quadruped (or the mount, pre="m_") of pose p into ImageDraw d."""
    g = lambda k: tuple(p[pre + k][:2])
    has = lambda k: (pre + k) in p
    fwd = -1.0 if p.get("facing", "lower-left") == "lower-left" else 1.0
    br, hr, nr, lw, pr = dims["body_r"], dims["head_r"], dims["neck_r"], dims["leg_w"], dims["paw_r"]

    def leg(name):
        if not has(f"{name}_paw"):
            return
        k = 1.0 if is_near(name) else FAR_DARKEN
        top, kn, pw = g(f"{name}_top"), g(f"{name}_knee"), g(f"{name}_paw")
        hind = name.startswith("h")
        capsule(d, top, kn, lw * (1.6 if hind else 1.3), _c(pal["leg"], k))
        capsule(d, kn, pw, lw, _c(pal["leg"], k))
        # paw / hoof: toes toward the facing, drawn along the ground line
        toe = (pw[0] + fwd * pr * 1.1, pw[1] + pr * 0.35)
        capsule(d, (pw[0] - fwd * pr * 0.2, pw[1]), toe, pr * 1.9, _c(pal["paw"], k))

    def tail():
        if not has("tail_base"):
            return
        pts = [g("tail_base")] + ([g("tail_mid")] if has("tail_mid") else []) + [g("tail_tip")]
        tw = dims["tail_w"]
        for i in range(len(pts) - 1):
            capsule(d, pts[i], pts[i + 1], tw * (1.0 - 0.25 * i), _c(pal["tail"]))
        if dims.get("tail_tuft"):
            disc(d, pts[-1], dims["tail_tuft"], _c(pal["tail"], 0.85))

    def body():
        wi, sp, hp = g("withers"), g("spine"), g("hips")
        capsule(d, wi, sp, 2 * br * 0.95, _c(pal["body"]))
        capsule(d, sp, hp, 2 * br * 0.9, _c(pal["body"]))
        disc(d, wi, br * 1.05, _c(pal["body"]))
        disc(d, hp, br * 1.0, _c(pal["body"]))

    def neck():
        wi, nk, hd = g("withers"), g("neck"), g("head")
        capsule(d, wi, nk, 2 * nr * 1.3, _c(pal["neck"]))
        capsule(d, nk, hd, 2 * nr, _c(pal["neck"]))

    def head():
        hd, sn, jw = g("head"), g("snout"), g("jaw")
        u = _unit(hd, sn)
        n1 = (-u[1], u[0])
        # the top of the head is the side away from the jaw
        top = n1 if (n1[0] * (jw[0] - hd[0]) + n1[1] * (jw[1] - hd[1])) < 0 else (-n1[0], -n1[1])
        at = lambda a, b: (hd[0] + u[0] * a * hr + top[0] * b * hr, hd[1] + u[1] * a * hr + top[1] * b * hr)
        sw = dims["snout_w"]
        gap = abs(_wrap(_ang(hd, sn) - _ang(hd, jw)))
        if gap > math.radians(20):  # open jaws: the mouth shows dark between the muzzle and the jaw
            ju = _unit(hd, jw)
            corner = (hd[0] + (u[0] + ju[0]) * 0.35 * hr, hd[1] + (u[1] + ju[1]) * 0.35 * hr)
            d.polygon([corner, (sn[0] - u[0] * 1.5, sn[1] - u[1] * 1.5), (jw[0] - ju[0] * 1.5, jw[1] - ju[1] * 1.5)],
                      fill=_c(pal["mouth"]))
        capsule(d, at(0.1, -0.45), jw, sw * 0.7, _c(pal["jaw"]))
        disc(d, hd, hr, _c(pal["head"]))
        capsule(d, at(0.2, 0.05), sn, sw, _c(pal["head"]))
        e = dims.get("ear", 1.0)
        if e and p.get("ear") == "flat":  # lying on its side: the ear lies back along the head
            d.polygon([at(0.0, 0.85), at(-0.6, 0.25), at(-1.1 - 0.6 * e, 0.55 + 0.1 * e)], fill=_c(pal["ear"]))
        elif e:
            d.polygon([at(0.05, 0.8), at(-0.75, 0.5), at(-0.8 - 0.2 * e, 0.6 + 0.95 * e)], fill=_c(pal["ear"]))
        ex, ey = at(0.35, 0.3)
        if p.get("eyes", "open") == "shut":
            d.line([(ex - 1.5, ey), (ex + 1.5, ey)], fill=_c(pal["eye"]), width=1)
        else:
            d.rectangle([ex - 1, ey - 1, ex, ey], fill=_c(pal["eye"]))

    def wing(w):
        if not has(f"{w}_root"):
            return
        k = 1.0 if w == "wn" else FAR_DARKEN
        rt, wr, tp = g(f"{w}_root"), g(f"{w}_wrist"), g(f"{w}_tip")
        hp = g("hips")
        back = (rt[0] + (hp[0] - rt[0]) * 0.6, rt[1] + (hp[1] - rt[1]) * 0.6)
        d.polygon([rt, wr, tp, back], fill=_c(pal["wing"], k))
        capsule(d, rt, wr, dims.get("wing_w", 3) * 1.3, _c(pal["wingbone"], k))
        capsule(d, wr, tp, dims.get("wing_w", 3), _c(pal["wingbone"], k))

    parts = {"tail": tail, "body": body, "neck": neck, "head": head, "wn": lambda: wing("wn"),
             "wf": lambda: wing("wf")}
    order = p.get("q_order", QUAD_ORDER)
    for part in order:
        if part in parts:
            parts[part]()
        elif part in ("ff", "hf"):  # far legs, plus any extra far legs
            for lg in [x for x in legs_of(p, pre) if not is_near(x) and (x == part or x[0] not in "fh")]:
                if x_once(lg, part):
                    leg(lg)
        elif part in ("fn", "hn"):
            for lg in [x for x in legs_of(p, pre) if is_near(x) and (x == part or x[0] not in "fh")]:
                if x_once(lg, part):
                    leg(lg)


def x_once(leg, part):
    """Extra legs (not f*/h*) are drawn once, with the hind leg of their side (behind the front leg)."""
    return leg[0] in "fh" or part.startswith("h")


def render_quad(p, style="grey"):
    dims = dict(SRC_DIMS["quadruped"], **p.get("dims", {}))
    pal = dict(QUAD_GREY, **p.get("palette", {}))
    im = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw_quad(ImageDraw.Draw(im), p, dims, pal)
    return im


# ---------------------------------------------------------------- the mounted mannequin
def rider_far(p):
    """Which rider arm is the far one ("r" = the weapon arm, "l" = the other): the one whose shoulder is on the
    screen side the rider looks to (screen-left for a lower-left facer)."""
    if "rider_far" in p:
        return p["rider_far"]
    left = p["r_sh"][0] <= p["l_sh"][0]
    return "r" if left == (p.get("facing", "lower-left") == "lower-left") else "l"


def rider_pose(p):
    """The biped pose guides.render_guide draws for the rider: his upper body and the near leg on the flank."""
    far = rider_far(p)
    near = "l" if far == "r" else "r"
    q = {k: p[k] for k in ("head", "neck", "chest", "pelvis", "r_sh", "r_el", "r_ha", "l_sh", "l_el", "l_ha")}
    if "sword" in p:
        q["sword"] = p["sword"]
    else:
        q["tip"] = p["tip"]
    if "shield" in p:
        q["shield"] = p["shield"]
    pe = p["pelvis"]
    side = 1 if p["r_sh"][0] > p["l_sh"][0] else -1  # screen side of the weapon arm
    q["r_hip"] = (pe[0] + 3.5 * side, pe[1] + 1)
    q["l_hip"] = (pe[0] - 3.5 * side, pe[1] + 1)
    q[f"{near}_kn"], q[f"{near}_ft"] = p["leg_kn"], p["leg_ft"]
    q[f"{far}_kn"], q[f"{far}_ft"] = p["leg_kn"], p["leg_ft"]  # not drawn (hidden by the mount)
    for k in ("head_tilt", "eyes", "head_r", "limb", "palette", "facing"):
        if k in p:
            q[k] = p[k]
    q["far"] = far
    if "rider_order" in p:
        q["order"] = [x.replace("near_", near + "_").replace("far_", far + "_") for x in p["rider_order"]]
    else:
        wpn_far = far == "r"
        q["order"] = (["sword", "r_arm", "torso", f"{near}_leg", "head", "l_arm", "shield"] if wpn_far else
                      ["shield", "l_arm", "torso", f"{near}_leg", "head", "r_arm", "sword"])
    if "shield" not in p:
        q["order"] = [x for x in q["order"] if x != "shield"]
    if p.get("fused"):  # a centaur has no rider's leg on the flank
        q["order"] = [x for x in q["order"] if not x.endswith("_leg")]
    return q


def render_rider(p, style="grey"):
    """The rider in the biped mannequin's style and greys (guides.render_guide), with the torso, hips, boots and blade
    scaled to the rider's head_r (guides draws the swordsman's torso at a fixed size, too broad for a rider)."""
    q = rider_pose(p)
    if q.get("facing", "lower-left") == "lower-right":
        return ImageOps.mirror(_render_rider(guides.mirror_pose(q), style))
    return _render_rider(q, style)


def _render_rider(p, style):
    pal = dict(guides.COLOUR if style == "colour" else guides.GREY, **p.get("palette", {}))
    im = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    g = lambda k: tuple(p[k][:2])
    hr = p.get("head_r", 13)
    k = hr / 13
    tk = max(0.55, min(1.2, p.get("torso_k", k)))
    ls = p.get("limb", 1.0)
    cap = lambda a, b, w, col: capsule(d, a, b, max(2, w * ls), col)

    def leg(side, kk):
        hip, kn, ft = g(side + "_hip"), g(side + "_kn"), g(side + "_ft")
        cap(hip, kn, 8, _c(pal["legging"], kk))
        cap(kn, ft, 7, _c(pal["legging"], kk))
        bx, by = ft[0] - 2 * tk, ft[1] + 1
        d.ellipse([bx - 6 * tk, by - 4 * tk, bx + 5 * tk, by + 3 * tk], fill=_c(pal["boot"], kk))
        cap(kn, ((kn[0] + ft[0]) / 2, (kn[1] + ft[1]) / 2 + 2 * tk), 7, _c(pal["boot"], kk))

    def arm(side, kk):
        sh, el, ha = g(side + "_sh"), g(side + "_el"), g(side + "_ha")
        cap(sh, el, 7, _c(pal["sleeve"], kk))
        cap(el, ha, 6, _c(pal["skin"], kk))
        disc(d, ha, 3 * max(0.7, tk), _c(pal["skin"], kk))

    def torso():
        rs, lsh, rh, lh, ch, pe = g("r_sh"), g("l_sh"), g("r_hip"), g("l_hip"), g("chest"), g("pelvis")
        if rs[0] > lsh[0]:
            rs, lsh, rh, lh = lsh, rs, lh, rh
        e = 2 * tk
        d.polygon([(rs[0] - e, rs[1] - e), (lsh[0] + e, lsh[1] - e), (lh[0] + 2 * e, lh[1] + e),
                   (rh[0] - 2 * e, rh[1] + e)], fill=_c(pal["tunic"]))
        d.ellipse([ch[0] - 12 * tk, ch[1] - 7 * tk, ch[0] + 12 * tk, ch[1] + 9 * tk], fill=_c(pal["tunic"]))
        d.ellipse([pe[0] - 11 * tk, pe[1] - 3 * tk, pe[0] + 11 * tk, pe[1] + 7 * tk], fill=_c(pal["shorts"]))
        d.line([(rh[0] - 5 * tk, pe[1] - 3 * tk), (lh[0] + 5 * tk, pe[1] - 3 * tk)], fill=_c(pal["belt"]),
               width=max(2, round(3 * tk)))
        cap(g("neck"), ch, 6, _c(pal["skin"]))

    def head():
        hx, hy = g("head")
        tilt = p.get("head_tilt", 0)
        d.ellipse([hx - hr, hy - hr, hx + hr, hy + hr], fill=_c(pal["skin"]))
        ox, oy = guides.rot((3 * k, -4 * k), tilt)
        d.ellipse([hx + ox - 12 * k, hy + oy - 11 * k, hx + ox + 12 * k, hy + oy + 9 * k], fill=_c(pal["hair"]))
        fx, fy = guides.rot((-3 * k, 3 * k), tilt)
        d.ellipse([hx + fx - 9 * k, hy + fy - 7 * k, hx + fx + 8 * k, hy + fy + 8 * k], fill=_c(pal["skin"]))
        ex, ey = guides.rot((-4 * k, 1 * k), tilt)
        if p.get("eyes", "open") == "shut":
            d.line([(hx + ex - 2, hy + ey), (hx + ex + 1, hy + ey)], fill=_c(pal["eye"]), width=1)
        else:
            d.rectangle([hx + ex - 1, hy + ey - 1, hx + ex, hy + ey + 1], fill=_c(pal["eye"]))

    def sword():
        hilt, tip = p["sword"] if "sword" in p else (g("r_ha"), g("tip"))
        ux, uy = _unit(hilt, tip)
        base = (hilt[0] + ux * 3, hilt[1] + uy * 3)
        capsule(d, base, tip, 5 * max(0.7, tk), _c(pal["blade"]))
        px_, py_ = -uy * 4 * tk, ux * 4 * tk
        d.line([(base[0] + px_, base[1] + py_), (base[0] - px_, base[1] - py_)], fill=_c(pal["hilt"]), width=2)
        capsule(d, (hilt[0] - ux * 3, hilt[1] - uy * 3), base, 3, _c(pal["hilt"]))

    def shield():
        if "shield" in p:
            cx, cy, rx, ry = p["shield"]
            d.ellipse([cx - rx, cy - ry, cx + rx, cy + ry], fill=_c(pal["wood"]), outline=_c(pal["rim"]), width=2)
            disc(d, (cx, cy), 2, _c(pal["boss"]))

    kr, kl = (FAR_DARKEN, 1.0) if p.get("far", "r") == "r" else (1.0, FAR_DARKEN)
    parts = {"sword": sword, "r_arm": lambda: arm("r", kr), "l_arm": lambda: arm("l", kl),
             "r_leg": lambda: leg("r", kr), "l_leg": lambda: leg("l", kl), "torso": torso, "head": head,
             "shield": shield}
    for part in p["order"]:
        parts[part]()
    return im


def render_mounted(p, style="grey"):
    dims = dict(SRC_DIMS["mounted"], **p.get("dims", {}))
    pal = dict(MOUNT_GREY, **p.get("mount_palette", {}))
    mount = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw_quad(ImageDraw.Draw(mount), p, dims, pal, pre="m_")
    out = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    if p.get("no_rider"):
        return mount
    rider = render_rider(p, style)
    for layer in ((rider, mount) if p.get("rider_behind") else (mount, rider)):
        out.alpha_composite(layer)
    return out


def render_guide(p, style="grey"):
    """160x128 RGBA mannequin for a quadruped or mounted pose (dispatch on the pose's body_plan, else its keys)."""
    plan = p.get("body_plan") or ("mounted" if "m_spine" in p else "quadruped")
    return render_mounted(p, style) if plan == "mounted" else render_quad(p, style)


# ---------------------------------------------------------------- retarget
def _ik(top, target, L1, L2, sign):
    """Two-bone IK: (knee, paw) with the knee on the side `sign` of the line top -> paw (sign of the cross product)."""
    dx, dy = target[0] - top[0], target[1] - top[1]
    dist = math.hypot(dx, dy) or 1e-6
    dd = min(max(dist, abs(L1 - L2) + 1e-3), L1 + L2 - 1e-3)
    ux, uy = dx / dist, dy / dist
    paw = (top[0] + ux * dd, top[1] + uy * dd)
    ca = max(-1.0, min(1.0, (L1 * L1 + dd * dd - L2 * L2) / (2 * L1 * dd)))
    a = math.acos(ca)
    best = None
    for s_ in (1, -1):
        c, sn = math.cos(s_ * a), math.sin(s_ * a)
        kn = (top[0] + L1 * (ux * c - uy * sn), top[1] + L1 * (ux * sn + uy * c))
        if best is None or (_cross(top, paw, kn) > 0) == (sign > 0):
            best = kn
            if (_cross(top, paw, kn) > 0) == (sign > 0):
                break
    return best, paw


def _bend(top, kn, pw, L):
    c = _cross(top, pw, kn)
    return 0 if abs(c) < 0.06 * L * L else (1 if c > 0 else -1)


def _abs_weight(src, group, d_src, mode):
    if mode != "drawn":
        return 0.0
    a = src.get("abs", {})
    if isinstance(a, (int, float)):
        return float(a) if group not in ("weapon",) else max(float(a), _mag(d_src))
    if group in a:
        return float(a[group])
    if group in ("weapon", "leg"):
        return _mag(d_src)
    return 0.0


def _mag(d_src):
    return min(1.0, max(0.0, (abs(_wrap(d_src)) - math.radians(15)) / math.radians(45)))


def scale_of(u, src_rest):
    """How much bigger the unit is than the source mannequin (body length and front-leg length, averaged)."""
    if "scale" in u:
        return u["scale"]
    r, pre = u["rest"], ("m_" if u["body_plan"] == "mounted" else "")
    ratio = lambda a, b: math.dist(r[pre + a], r[pre + b]) / max(1e-6, math.dist(src_rest[pre + a], src_rest[pre + b]))
    leg = (math.dist(r[pre + "fn_top"], r[pre + "fn_knee"]) + math.dist(r[pre + "fn_knee"], r[pre + "fn_paw"])) / max(
        1e-6, math.dist(src_rest[pre + "fn_top"], src_rest[pre + "fn_knee"]) +
        math.dist(src_rest[pre + "fn_knee"], src_rest[pre + "fn_paw"]))
    return round(0.5 * (ratio("withers", "hips") + leg), 3)


def _carry(rest, now, pre="m_"):
    """The rigid motion of the mount's barrel (origin spine, axis hips -> withers) from rest to now: f(point)."""
    o0, o1 = rest[pre + "spine"], now[pre + "spine"]
    t = _wrap(_ang(now[pre + "hips"], now[pre + "withers"]) - _ang(rest[pre + "hips"], rest[pre + "withers"]))
    c, s = math.cos(t), math.sin(t)

    def f(pt, local=(0.0, 0.0)):
        x, y = pt[0] - o0[0] + local[0], pt[1] - o0[1] + local[1]
        return (o1[0] + c * x - s * y, o1[1] + s * x + c * y)
    return f, t


def retarget(u, src_rest, src, mode="drawn"):
    """The unit's pose for one source frame (see the module doc). u: a loaded unit (load_unit)."""
    rest, plan, s = u["rest"], u["body_plan"], u["scale"]
    pre = "m_" if plan == "mounted" else ""
    if mode == "disp":
        out = {}
        for j, rv in rest.items():
            if j == "shield":
                continue
            sp, sr = src.get(j), src_rest.get(j)
            out[j] = tuple(rv[:2]) if sp is None or sr is None else (rv[0] + (sp[0] - sr[0]) * s,
                                                                      rv[1] + (sp[1] - sr[1]) * s)
        if "shield" in rest and "shield" in src:
            rr, sp, sr = rest["shield"], src["shield"], src_rest["shield"]
            out["shield"] = (rr[0] + (sp[0] - sr[0]) * s, rr[1] + (sp[1] - sr[1]) * s, rr[2] * sp[2] / sr[2],
                             rr[3] * sp[3] / sr[3])
        return _tail(u, src, out)
    out = {}
    root = pre + "spine"
    out[root] = (rest[root][0] + (src[root][0] - src_rest[root][0]) * s,
                 rest[root][1] + (src[root][1] - src_rest[root][1]) * s)
    planted = set(src.get("planted", LEG_NAMES)) if mode == "drawn" else set()
    tree = quad_tree(rest, pre)
    ik_legs = []

    def bone(a, b, base=None):
        sa, sb, ra, rb = src.get(a), src.get(b), src_rest.get(a), src_rest.get(b)
        base = base if base is not None else out[a]
        L = math.dist(rest[a][:2], rest[b][:2])
        if sa is None or sb is None or ra is None or rb is None:  # no source bone: carry the unit's own offset
            return (base[0] + rest[b][0] - rest[a][0], base[1] + rest[b][1] - rest[a][1])
        d_src = _wrap(_ang(sa, sb) - _ang(ra, rb))
        ang = _ang(rest[a], rest[b]) + d_src
        w = _abs_weight(src, bone_group(a, b, plan), d_src, mode)
        ang += w * _wrap(_ang(sa, sb) - ang)
        return (base[0] + L * math.cos(ang), base[1] + L * math.sin(ang))

    for a, b in tree:
        leg = b[len(pre):].rsplit("_", 1)[0]
        if b.endswith("_knee") and leg in planted:
            ik_legs.append(leg)
            continue
        if b.endswith("_paw") and leg in planted:
            continue
        if b == pre + "jaw":  # the jaw opens relative to the snout: the unit's own gape plus the source's change
            out[b] = _jaw(rest, src_rest, src, out, pre)
            continue
        out[b] = bone(a, b)
    for leg in ik_legs:
        t, k, pw = (f"{pre}{leg}_{j}" for j in ("top", "knee", "paw"))
        L1, L2 = math.dist(rest[t], rest[k]), math.dist(rest[k], rest[pw])
        target = (rest[pw][0] + (src[pw][0] - src_rest[pw][0]) * s, rest[pw][1] + (src[pw][1] - src_rest[pw][1]) * s)
        su = _bend(rest[t], rest[k], rest[pw], L1) or 1
        ss, ssr = _bend(src[t], src[k], src[pw], L1), _bend(src_rest[t], src_rest[k], src_rest[pw], L1)
        sign = su * (ss * ssr if ss and ssr else 1)
        out[k], out[pw] = _ik(out[t], target, L1, L2, sign)
    if plan == "mounted":
        _retarget_rider(u, src_rest, src, mode, out, bone)
    if src.get("ground") == "body":
        _ground(u, src_rest, src, out)
    _legs_above_ground(u, out, pre)
    _head_above_ground(u, out, pre)
    if plan == "mounted":
        _rider_above_ground(u, out)
    out = _tail(u, src, out)
    if mode == "drawn":
        _fit_x(out)
    return out


SAFE = (16, 143)  # the region of the canvas the image model is shown (recipe: x 16-143)
MAX_FIT = 4


def _fit_x(p):
    """A guide that pokes out of the safe region on one side (a long tail, a stretched neck) slides back in by up to
    MAX_FIT px; the slicer later slides the drawn frame onto its guide, so the drawn figure moves with it. Bigger
    overflows stay (and are flagged as _overflow) for the canvas limit to deal with."""
    bb = render_guide(p).getchannel("A").point(lambda a: 255 if a > 60 else 0).getbbox()
    if not bb:
        return
    lo, hi = SAFE[0] - bb[0], (bb[2] - 1) - SAFE[1]
    dx = lo if lo > 0 >= hi else (-hi if hi > 0 >= lo else 0)
    if dx and abs(dx) <= MAX_FIT:
        for k, v in list(p.items()):
            if k == "sword":
                p[k] = ((v[0][0] + dx, v[0][1]), (v[1][0] + dx, v[1][1]))
            elif k == "shield":
                p[k] = (v[0] + dx, v[1], v[2], v[3])
            elif _is_pt(v):
                p[k] = (v[0] + dx, v[1])
    elif lo > 0 or hi > 0:
        p["_overflow"] = max(lo, hi)


MAX_GAPE = math.radians(48)


def _jaw(rest, src_rest, src, out, pre):
    """The jaw tip: the unit's snout direction now, plus the unit's own gape at rest, plus how much wider (or
    narrower) the source opens its jaws in this frame than at its rest; the gape stays between closed and 60 deg."""
    h, sn, jw = pre + "head", pre + "snout", pre + "jaw"
    gap = lambda p: _wrap(_ang(p[h], p[jw]) - _ang(p[h], p[sn]))
    g_u, g_sr, g_s = gap(rest), gap(src_rest), gap(src)
    side = 1 if (g_u if abs(g_u) > 1e-3 else g_sr) >= 0 else -1  # which side of the snout the jaw hangs on
    g = side * min(MAX_GAPE, max(math.radians(4), side * g_u + side * (g_s - g_sr)))
    a = _ang(out[h], out[sn]) + g
    L = math.dist(rest[h], rest[jw])
    return (out[h][0] + L * math.cos(a), out[h][1] + L * math.sin(a))


def ground_row(u):
    """The unit's ground: the lowest paw at frame 0 plus half a paw (where the sprite's feet are)."""
    pre = "m_" if u["body_plan"] == "mounted" else ""
    return max(v[1] for k, v in u["rest"].items() if k.startswith(pre) and k.endswith("_paw")) + u["dims"]["paw_r"] * 0.6


def _head_above_ground(u, out, pre):
    """A unit whose head rests low (a crouching wolf) must not bite into the ground when the source lowers its head:
    turn the head (with the snout and jaw) up about the neck until it clears the ground row."""
    g = ground_row(u)
    h, sn, jw, nk = (pre + k for k in ("head", "snout", "jaw", "neck"))
    if nk not in out or h not in out:
        return
    dm = u["dims"]
    low = lambda: max(out[h][1] + dm["head_r"], out[sn][1] + dm["snout_w"] / 2, out[jw][1] + dm["snout_w"] * 0.35)
    if low() <= g:
        return
    up = -1 if out[h][0] > out[nk][0] else 1  # screen rotation that lifts a head pointing right (-) or left (+)
    piv = out[nk]
    for _ in range(30):
        t = math.radians(3) * up
        c, s = math.cos(t), math.sin(t)
        for k in (h, sn, jw):
            x, y = out[k][0] - piv[0], out[k][1] - piv[1]
            out[k] = (piv[0] + c * x - s * y, piv[1] + s * x + c * y)
        if low() <= g:
            break


def _rot_about(out, keys, piv, t):
    c, s = math.cos(t), math.sin(t)
    for k in keys:
        x, y = out[k][0] - piv[0], out[k][1] - piv[1]
        out[k] = (piv[0] + c * x - s * y, piv[1] + s * x + c * y)


def _legs_above_ground(u, out, pre):
    """A leg whose paw would go below the ground (a long-legged unit lying down) turns about its top toward the
    ground line until the paw rests on it: legs lie flat on the ground instead of poking through it."""
    g = ground_row(u) - u["dims"]["paw_r"] * 0.6
    for leg in legs_of(out, pre):
        t, k, pw = (f"{pre}{leg}_{j}" for j in ("top", "knee", "paw"))
        if out[pw][1] <= g + 0.5:
            continue
        up = 1 if out[pw][0] < out[t][0] else -1  # turning a paw left of its top clockwise lifts it
        for _ in range(40):
            _rot_about(out, (k, pw), out[t], math.radians(2) * up)
            if out[pw][1] <= g + 0.5:
                break


def _retarget_rider(u, src_rest, src, mode, out, bone):
    rest, s = u["rest"], u["scale"]
    carry_s, t_s = _carry(src_rest, src)
    carry_u, t_u = _carry(rest, out)
    if u.get("fused"):  # a centaur: the man's waist is the horse's chest, wherever the source rider goes
        out["pelvis"] = carry_u(rest["pelvis"])
    elif src.get("rider_free"):  # thrown off: he moves as the source rider moves from the saddle, not with the mount
        out["pelvis"] = (rest["pelvis"][0] + (src["pelvis"][0] - src_rest["pelvis"][0]) * s,
                         rest["pelvis"][1] + (src["pelvis"][1] - src_rest["pelvis"][1]) * s)
    else:  # in the saddle: carried by the barrel, plus the source rider's own offset from it (barrel frame, scaled)
        cs = carry_s(src_rest["pelvis"])
        dx, dy = src["pelvis"][0] - cs[0], src["pelvis"][1] - cs[1]
        c, sn = math.cos(-t_s), math.sin(-t_s)
        lx, ly = c * dx - sn * dy, sn * dx + c * dy
        c, sn = math.cos(t_u), math.sin(t_u)
        pu = carry_u(rest["pelvis"])
        out["pelvis"] = (pu[0] + (c * lx - sn * ly) * s, pu[1] + (sn * lx + c * ly) * s)
    for a, b in RIDER_TREE:
        if a in out and b in rest and a in rest:
            nb = bone(a, b)
            if b == "tip":
                nb = _clamp_weapon(out[a], nb, rest, out)
            out[b] = nb
    if "sword" in src:  # the weapon dropped: lies centred where the source drops it, at the unit's length (shortened
        #                 to stay inside the canvas, like a held weapon's guide)
        L = math.dist(rest["r_ha"], rest["tip"])
        (h0, t0) = src["sword"]
        cx = out["pelvis"][0] + ((h0[0] + t0[0]) / 2 - src["pelvis"][0]) * s
        cy = out["pelvis"][1] + ((h0[1] + t0[1]) / 2 - src["pelvis"][1]) * s
        ux, uy = _unit(h0, t0)
        for _ in range(40):
            h, t = (cx - ux * L / 2, cy - uy * L / 2), (cx + ux * L / 2, cy + uy * L / 2)
            if all(18 <= v[0] <= 142 for v in (h, t)):
                break
            L *= 0.95
            out["_weapon_shortened"] = True
        out["sword"] = (h, t)
    if "shield" in rest and "shield" in src:
        dx, dy = src["shield"][0] - src["l_ha"][0], src["shield"][1] - src["l_ha"][1]
        rr = rest["shield"]
        out["shield"] = (out["l_ha"][0] + dx * s, out["l_ha"][1] + dy * s,
                         rr[2] * src["shield"][2] / src_rest["shield"][2], rr[3] * src["shield"][3] / src_rest["shield"][3])


def _clamp_weapon(base, nb, rest, out):
    """Shorten the GUIDE weapon along its angle so it stays inside the canvas (x 18-142, y 3-125), as recipe does."""
    L = math.dist(rest["r_ha"], rest["tip"])
    a = _ang(base, nb)
    for _ in range(40):
        if 18 <= nb[0] <= 142 and 3 <= nb[1] <= 125:
            break
        L *= 0.95
        nb = (base[0] + L * math.cos(a), base[1] + L * math.sin(a))
        out["_weapon_shortened"] = True
    return nb


def _shift_pose(p, dy):
    q = dict(p)
    for k, v in p.items():
        if k == "sword":
            q[k] = ((v[0][0], v[0][1] + dy), (v[1][0], v[1][1] + dy))
        elif k == "shield":
            q[k] = (v[0], v[1] + dy, v[2], v[3])
        elif _is_pt(v):
            q[k] = (v[0], v[1] + dy)
    return q


def _bottom(p, lift=40, parts=None, rider=True):
    """The lowest opaque row of the mannequin, or of some of its parts (measured lifted by `lift` px, so a part below
    the canvas counts)."""
    q = _shift_pose(p, -lift)
    if parts is not None:
        q["q_order"] = parts
    if not rider:
        q["no_rider"] = True
    bb = render_guide(q).getchannel("A").point(lambda a: 255 if a > 60 else 0).getbbox()
    return bb[3] - 1 + lift if bb else None


def _ground(u, src_rest, src, out):
    """Keep the source's height above the ground for a lying or falling frame: move the unit pose vertically so the
    lowest row of its barrel sits as far above the unit's frame-0 lowest row as the source's does (times the scale), so
    the body lies on the ground whatever the unit's leg length. The legs, the head and a fallen rider are then kept
    above the ground separately (_legs_above_ground, _head_above_ground, _rider_above_ground)."""
    tail = lambda p, dims: dict(p, body_plan=u["body_plan"], facing=u["facing"], dims=dims,
                                **({k: v for k, v in u["rider_keys"].items()} if u["body_plan"] == "mounted" else {}))
    src_dims = SRC_DIMS[u["body_plan"]]
    b = lambda p: _bottom(p, parts=["body"], rider=False)
    gap = b(tail(src_rest, src_dims)) - b(tail(src, src_dims))
    want = b(tail(u["rest"], u["dims"])) - gap * u["scale"]
    have = b(tail(out, u["dims"]))
    free = u["body_plan"] == "mounted" and src.get("rider_free") and not u.get("fused")
    keys = [k for k in out if k.startswith("m_")] if free else list(out)
    out.update(_shift_pose({k: out[k] for k in keys}, want - have))
    if free:  # the thrown rider lies as high above the ground as the source rider does
        legs = lambda p: _bottom(p, parts=["ff", "hf", "hn", "fn"], rider=False)
        gap = legs(tail(src_rest, src_dims)) - _rider_bottom(tail(src, src_dims))
        want = legs(tail(u["rest"], u["dims"])) - gap * u["scale"]
        have = _rider_bottom(tail(out, u["dims"]))
        keys = [k for k in RIDER_JOINTS if k in out] + [k for k in ("shield", "sword") if k in out]
        out.update(_shift_pose({k: out[k] for k in keys}, want - have))


def _rider_bottom(p, lift=40):
    bb = render_rider(_shift_pose(p, -lift)).getchannel("A").point(lambda a: 255 if a > 60 else 0).getbbox()
    return bb[3] - 1 + lift if bb else None


def _rider_above_ground(u, out):
    """A rider thrown to the ground lies on it, not under it: lift his joints if his figure reaches below the ground."""
    bot = _rider_bottom(dict(out, facing=u["facing"], **u["rider_keys"]))
    if bot is None:
        return
    dy = ground_row(u) - bot
    if dy < 0:
        keys = [k for k in RIDER_JOINTS if k in out] + [k for k in ("shield", "sword") if k in out]
        sub = _shift_pose({k: out[k] for k in keys}, dy)
        out.update(sub)


def _tail(u, src, out):
    for k in ("eyes", "ear", "head_tilt", "q_order", "rider_order", "rider_behind", "rider_free", "planted",
              "ground"):
        if k in src:
            out[k] = src[k]
    out["body_plan"] = u["body_plan"]
    out["facing"] = u["facing"]
    out["dims"] = u["dims"]
    if u["body_plan"] == "mounted":
        out.update(u["rider_keys"])
    if u.get("palette"):
        out["palette"] = u["palette"]
    return out


# ---------------------------------------------------------------- units and source sets
def load_unit(path_or_dict, action=None):
    u = json.loads(Path(path_or_dict).read_text()) if not isinstance(path_or_dict, dict) else dict(path_or_dict)
    plan = u.get("body_plan")
    if plan not in SETS:
        raise SystemExit(f"body_plan must be one of {tuple(SETS)}, got {plan!r}")
    if u.get("facing") not in ("lower-left", "lower-right"):
        raise SystemExit(f"'facing' must be lower-left or lower-right (the way the head looks), got {u.get('facing')!r}")
    u["rest"] = {k: (tuple(v) if k != "shield" else tuple(v)) for k, v in flatten_joints(u).items()}
    need = [f"m_{j}" for j in QUAD_JOINTS] + list(RIDER_JOINTS) if plan == "mounted" else list(QUAD_JOINTS)
    missing = [j for j in need if j not in u["rest"]]
    if missing:
        raise SystemExit(f"{u.get('name')}: joints {missing} are missing")
    if plan == "mounted":
        u["rider_action"] = action or u.get("rider_action", "swing")
        if u["rider_action"] not in RIDER_ACTIONS:
            raise SystemExit(f"rider_action must be one of {RIDER_ACTIONS}")
        u["mount_action"] = u.get("mount_action") if not action else None
        u["mount_action"] = u["mount_action"] or MOUNT_FOR_ACTION[u["rider_action"]]
        u["rider_keys"] = {"head_r": u.get("head_r", 13), "limb": u.get("limb", 1.0),
                           "rider_far": u.get("rider_far") or rider_far(dict(u["rest"], facing=u["facing"]))}
        if u.get("fused"):
            u["rider_keys"]["fused"] = True
    u["mirror"] = u["facing"] == "lower-right"
    src = source_set(u)
    u["scale"] = scale_of(u, src["idle"][0])
    base = SRC_DIMS[plan]
    u["dims"] = {k: round(v * u["scale"], 2) if k not in ("ear",) else v for k, v in base.items()}
    u["dims"].update(u.get("dims", {}) if isinstance(u.get("dims"), dict) else {})
    sp = u.get("sprite")
    if sp:
        u["sprite_img"] = Image.open(ROOT / sp).convert("RGBA")
        u["offset"] = (int(80 - u["sprite_img"].width / 2), H - u["sprite_img"].height)
    return u


def source_set(u):
    """The source frames for this unit, mirrored when it looks lower-right; extra legs synthesized."""
    data = json.loads((ROOT / u.get("poses_from", SETS[u["body_plan"]])).read_text())
    anims = dict(data["anims"])
    if u["body_plan"] == "mounted":
        key = f"{u['rider_action']}+{u['mount_action']}"
        anims["strike"] = data["strikes"][key]
    for name, w in (u.get("extra_legs") or {}).items():
        side = name[-1]
        for fr in anims.values():
            for p in fr:
                for j in ("top", "knee", "paw"):
                    a, b = p[f"f{side}_{j}"], p[f"h{side}_{j}"]
                    p[f"{name}_{j}"] = [a[0] + (b[0] - a[0]) * w, a[1] + (b[1] - a[1]) * w]
                if "planted" in p and (f"f{side}" in p["planted"] or f"h{side}" in p["planted"]):
                    p["planted"] = list(p["planted"]) + [name]
    if u["mirror"]:
        anims = {a: [mirror_pose(p) for p in fr] for a, fr in anims.items()}
    return anims


WARP_FRAMES = {("idle", 1), ("idle", 2), ("idle", 3), ("strike", 7), ("flinch", 3)}


def unit_poses(u):
    """Every frame of every animation for this unit (the recipe's unit_poses for these body plans)."""
    src = source_set(u)
    rest = src["idle"][0]

    def mode(anim, i):
        if i == 0 and anim in ("idle", "strike"):
            return "rel"
        return "disp" if (anim, i) in WARP_FRAMES else "drawn"
    return {anim: [retarget(u, rest, p, mode(anim, i)) for i, p in enumerate(frames)] for anim, frames in src.items()}


def beat_text(u, anim, i):
    """The unit's own override (written for its real facing), else the source set's text with the unit's words,
    mirrored (left <-> right) for a unit that looks lower-right."""
    import beats
    own = u.get("beats", {}).get(anim, {}).get(str(i))
    if own:
        return own
    if u["body_plan"] == "mounted":
        import beats_mounted as bm
        t = bm.text(anim, i, u["rider_action"], u["mount_action"], u.get("words", {}))
    else:
        import beats_quad as bq
        t = bq.text(anim, i, u.get("words", {}))
    return beats.swap_sides(t) if u["mirror"] else t


# ---------------------------------------------------------------- warps (idle, settles) and the check image
def tree_of(p):
    pre = "m_" if "m_spine" in p else ""
    t = quad_tree(p, pre)
    if pre:
        t += [(a, b) for a, b in RIDER_TREE if a in p and b in p]
    return t


def warp_points(p, keys, tree, bone_samples=1):
    """MLS handles for rig.mls_warp: the given joints plus points along each bone of the tree."""
    pts = [tuple(p[k][:2]) for k in keys]
    for a, b in tree:
        for k in range(1, bone_samples + 1):
            t = k / (bone_samples + 1)
            pts.append((p[a][0] + (p[b][0] - p[a][0]) * t, p[a][1] + (p[b][1] - p[a][1]) * t))
    return pts


def warp_to_pose(base, pose_src, pose_dst, sharpen=3.0):
    """The real sprite bent from its frame-0 pose to pose_dst (idle breaths, settles), as rig.warp_to_pose does for
    bipeds; every joint the two poses share, and points along the bones, are handles."""
    import rig
    keys = sorted(k for k, v in pose_src.items() if _is_pt(v) and _is_pt(pose_dst.get(k)))
    tree = [(a, b) for a, b in tree_of(pose_src) if a in keys and b in keys]
    return rig.mls_warp(base, warp_points(pose_src, keys, tree), warp_points(pose_dst, keys, tree), sharpen=sharpen)


def base_canvas(u):
    c = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    c.alpha_composite(u["sprite_img"], u["offset"])
    return c


def skeleton_view(u, rest, scale=2):
    ground = (126, 170, 60, 255)
    sk = Image.new("RGBA", (W, H), ground)
    sk.alpha_composite(base_canvas(u))
    sk = sk.resize((W * scale, H * scale), Image.NEAREST)
    d = ImageDraw.Draw(sk)
    for a, b in tree_of(rest):
        if a.startswith("m_") or u["body_plan"] == "quadruped":
            near = (b[len("m_") if a.startswith("m_") else 0:].split("_")[0] in ("fn", "hn", "wn"))
            col = (255, 140, 0, 255) if near else (255, 40, 40, 255)
            if b.endswith(("snout", "jaw", "head")):
                col = (40, 200, 255, 255)
        else:
            col = (255, 220, 0, 255) if (a, b) in WEAPON_CHAIN else (255, 60, 200, 255)
        d.line([(rest[a][0] * scale, rest[a][1] * scale), (rest[b][0] * scale, rest[b][1] * scale)], fill=col, width=2)
    for j, v in rest.items():
        if _is_pt(v) or (isinstance(v, tuple) and len(v) == 2 and isinstance(v[0], (int, float))):
            d.ellipse([v[0] * scale - 2, v[1] * scale - 2, v[0] * scale + 2, v[1] * scale + 2], fill=(255, 255, 255, 255))
    if "shield" in rest:
        cx, cy, rx, ry = rest["shield"]
        d.ellipse([(cx - rx) * scale, (cy - ry) * scale, (cx + rx) * scale, (cy + ry) * scale], outline=(255, 255, 255, 255))
    return sk


def check_guides(u, out_png, scale=2):
    """out_png: row 1 = the frame-0 skeleton over the sprite (mount/animal bones red, near legs orange, head cyan,
    rider magenta, weapon yellow, joints white), then strike 1-6; row 2 = the frame-0 mannequin over the sprite in a
    red tint, then flinch 0-2 and idle 1-3 (the idle and settles are warps of the sprite; their guides only show the
    motion); row 3 = death 1-7. Every cell is labelled."""
    poses = unit_poses(u)
    rest = poses["idle"][0]
    ground = (126, 170, 60, 255)
    base = base_canvas(u)
    g = render_guide(rest)
    red = Image.new("RGBA", g.size, (255, 40, 40, 255))
    g2 = Image.blend(g, red, 0.45)
    g2.putalpha(g.getchannel("A").point(lambda a: int(a * 0.75)))
    over = Image.new("RGBA", (W, H), ground)
    over.alpha_composite(base)
    over.alpha_composite(g2)
    cell = lambda im: im if im.size == (W * scale, H * scale) else im.resize((W * scale, H * scale), Image.NEAREST)
    rows = [[("frame 0 skeleton", skeleton_view(u, rest, scale))] +
            [(f"strike {i}", render_guide(poses["strike"][i])) for i in range(1, 7)],
            [("frame 0 mannequin", over)] + [(f"flinch {i}", render_guide(poses["flinch"][i])) for i in range(3)] +
            [(f"idle {i} (warp)", render_guide(poses["idle"][i])) for i in range(1, 4)],
            [(f"death {i}", render_guide(poses["death"][i])) for i in range(1, 8)]]
    Wd = max(len(r) for r in rows) * W * scale
    sheet = Image.new("RGBA", (Wd, H * scale * len(rows)), (255, 0, 255, 255))
    dd = ImageDraw.Draw(sheet)
    for y, row in enumerate(rows):
        for x, (lab, f) in enumerate(row):
            sheet.alpha_composite(cell(f), (x * W * scale, y * H * scale))
            dd.text((x * W * scale + 4, y * H * scale + 3), lab, fill=(255, 255, 255, 255))
    title = f"{u['name']}: {u['body_plan']}, facing {u['facing']}{' (source mirrored)' if u['mirror'] else ''}, " \
            f"scale {u['scale']}" + (f", rider {u['rider_action']}, mount {u['mount_action']}"
                                     if u["body_plan"] == "mounted" else "")
    dd.text((4, H * scale * 3 - 14), title, fill=(255, 255, 255, 255))
    Path(out_png).parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out_png)
    short = [f"{a} {i}" for a, fr in poses.items() for i, p in enumerate(fr) if p.get("_weapon_shortened")]
    if short:
        print(f"warning: the guide weapon was shortened to fit the canvas in {', '.join(short)}", file=sys.stderr)
    over = [f"{a} {i} ({p['_overflow']} px)" for a, fr in poses.items() for i, p in enumerate(fr) if p.get("_overflow")]
    if over:
        print(f"warning: the guide leaves the safe region x {SAFE[0]}-{SAFE[1]} in {', '.join(over)}: expect the drawn "
              "figure cut off at the canvas edge there (a bigger canvas, ANIMATING_UNITS.md 'Known limits')",
              file=sys.stderr)
    return out_png


def pose_sheet(poses_path, out, strike=None, scale=2, wings=False):
    """Every frame of a source set (lower-left), one row per animation, frame numbers in the cells; the source's wing
    joints are drawn only with wings=True (only winged units use them)."""
    data = json.loads(Path(poses_path).read_text())
    rows = list(data["anims"].items())
    if strike and "strikes" in data:
        rows = [(k if k != "strike" else f"strike {strike}", v if k != "strike" else data["strikes"][strike])
                for k, v in rows]
    elif "strikes" in data:
        rows = [r for r in rows if r[0] != "strike"] + [(f"strike {k}", v) for k, v in data["strikes"].items()]
    n = max(len(fr) for _, fr in rows)
    sheet = Image.new("RGBA", ((n + 1) * W, len(rows) * H), (60, 60, 60, 255))
    d = ImageDraw.Draw(sheet)
    for y, (anim, fr) in enumerate(rows):
        d.text((4, y * H + 4), anim, fill=(255, 255, 255, 255))
        for i, pz in enumerate(fr):
            if not wings:
                pz = {k: v for k, v in pz.items() if not k.startswith(("wn_", "wf_", "m_wn_", "m_wf_"))}
            c = Image.new("RGBA", (W, H), (255, 0, 255, 255))
            c.alpha_composite(render_guide(pz))
            cd = ImageDraw.Draw(c)
            cd.text((3, 2), str(i), fill=(255, 255, 255, 255))
            cd.line([(0, 125), (W, 125)], fill=(255, 255, 255, 90))
            sheet.alpha_composite(c, ((i + 1) * W, y * H))
    sheet = sheet.resize((sheet.width * scale, sheet.height * scale), Image.NEAREST)
    Path(out).parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--unit")
    ap.add_argument("--check-guides", action="store_true")
    ap.add_argument("--action", choices=RIDER_ACTIONS, help="mounted: preview another rider action")
    ap.add_argument("--out")
    ap.add_argument("--poses", help="render every frame of a source set to --out")
    ap.add_argument("--strike", help="with --poses on poses_mounted.json: one strike variant, e.g. swing+rear")
    ap.add_argument("--wings", action="store_true", help="with --poses: draw the source's wing joints too")
    ap.add_argument("--show-beats", action="store_true")
    a = ap.parse_args()
    if a.poses:
        print(pose_sheet(a.poses, a.out or ROOT / "work/quad/source.png", a.strike, wings=a.wings))
        sys.exit(0)
    u = load_unit(a.unit, a.action)
    if a.show_beats:
        print(f"{u['name']}: {u['body_plan']}, facing {u['facing']}{' (mirrored)' if u['mirror'] else ''}"
              + (f", rider {u['rider_action']}, mount {u['mount_action']}" if u["body_plan"] == "mounted" else ""))
        for anim, idx in (("strike", range(1, 7)), ("flinch", range(3)), ("death", range(1, 8))):
            for i in idx:
                own = "own " if u.get("beats", {}).get(anim, {}).get(str(i)) else "    "
                print(f"{anim:6} {i}  {own}{beat_text(u, anim, i)}")
        sys.exit(0)
    if a.check_guides:
        suffix = f"_{a.action}" if a.action else ""
        print(check_guides(u, a.out or ROOT / f"work/quad/{u['name']}{suffix}_guides.png"))
