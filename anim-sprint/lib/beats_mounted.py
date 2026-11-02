"""Beat texts for the mounted pose set (lib/poses_mounted_src.py -> poses_mounted.json): a rider on a mount, both looking
LOWER-LEFT. The cells the image model draws are strike 1-6, flinch 0-2 and death 1-7; timings and the "hit" event on
strike 4 are lib/beats.py's.

A strike text is the phase, the rider's clause for the unit's "rider_action" (swing | thrust | shoot) and the mount's
clause for its "mount_action" (rear | surge | stand). Generic words in braces are swapped for the unit's own (a unit
file's "words": {"mount": "wolf", "rider": "goblin", "weapon": "spear", "feet": "paws", "his": "her"}). Directions are
written for the lower-left facing and pass recipe.wrong_way for a lower-left unit; quad.beat_text mirrors them with
beats.swap_sides for a unit that looks lower-right.
"""
import re

WORDS = {"mount": "horse", "rider": "rider", "weapon": "weapon", "feet": "hooves", "his": "his", "him": "him"}

PHASE = {1: "anticipation", 2: "wind-up extreme", 3: "swing", 4: "contact", 5: "follow-through", 6: "recover"}
PHASE_SHOOT = {1: "nock", 2: "full draw, the wind-up extreme", 3: "aim", 4: "release", 5: "recoil", 6: "recover"}

MOUNT = {
    "rear": {
        1: "the {mount} gathers its weight back onto its haunches and lifts its head",
        2: "the {mount} rears up high on its hind legs, forelegs folded in the air, head up",
        3: "the {mount} plunges forward and down, its forelegs reaching toward the lower-left",
        4: "the {mount}'s front {feet} land ahead at the lower-left, its forehand low and its head stretched forward",
        5: "the {mount} stands with its weight on its forehand, head low",
        6: "the {mount} steps back halfway toward where it stood",
    },
    "surge": {
        1: "the {mount} sinks back onto its haunches",
        2: "the {mount} coils back, crouched on its hind legs, ready to spring",
        3: "the {mount} springs forward toward the lower-left, forelegs reaching off the ground",
        4: "the {mount} lunges far forward toward the lower-left, low, its neck stretched out, hind legs driving",
        5: "the {mount} holds the lunge, front {feet} planted ahead",
        6: "the {mount} steps back halfway toward where it stood",
    },
    "stand": {
        1: "the {mount} stands still",
        2: "the {mount} stands still, its weight shifted back a little",
        3: "the {mount} stands, leaning a little forward",
        4: "the {mount} stands steady",
        5: "the {mount} stands steady",
        6: "the {mount} stands still",
    },
}

RIDER = {
    "swing": {
        1: "the {rider} lifts the {weapon} up over {his} head, its point up and back",
        2: "the {rider} leans back in the saddle with the {weapon} cocked behind {his} head, hanging down behind {his} "
           "shoulders",
        3: "the {rider} whips forward and sweeps the {weapon} over the top, level and pointing to the lower-left",
        4: "the {rider} leans far forward over the {mount}'s neck, {his} weapon arm fully extended toward the "
           "lower-left and the {weapon} driving down at the lower-left past the {mount}'s head",
        5: "the {rider} follows through, the {weapon} carrying on down at the lower-left",
        6: "the {rider} sits up and brings the {weapon} back up toward the ready angle",
    },
    "thrust": {
        1: "the {rider} lowers the {weapon} level, pointing forward to the lower-left, tucked under {his} arm",
        2: "the {rider} draws the {weapon} back, still level, leaning in",
        3: "the {rider} drives the {weapon} forward, its point toward the lower-left",
        4: "the {rider} leans far forward over the {mount}'s neck with {his} weapon arm fully extended, the {weapon} "
           "thrust out along a line to the lower-left, its point the farthest point of the figure",
        5: "the {rider} holds the thrust, the point of the {weapon} dipping a little",
        6: "the {rider} sits up and raises the {weapon} back toward the ready angle",
    },
    "shoot": {
        1: "the {rider} raises the {weapon} upright in front of {his} chest and sets an arrow on the string",
        2: "the {rider} leans back with the {weapon} arm pushed out to the left and a little up, the string "
           "pulled back to {his} jaw, the draw elbow high and back",
        3: "the {rider} brings the aim down onto the target at the lower-left, still at full draw",
        4: "the {rider}'s {weapon} arm locked straight toward the lower-left, the arrow gone, the draw hand "
           "flung back past {his} ear toward the upper-right",
        5: "the {weapon} rolls forward and dips, the draw hand still out behind",
        6: "the {rider} lowers the {weapon} back to the ready angle",
    },
}

OTHER = {
    "flinch": {
        0: "impact: hit from the lower-left; the {mount} throws its head up and shies back toward the upper-right, its "
           "front {feet} off the ground; the {rider} jolts back in the saddle, eyes squeezed shut, arms flung out",
        1: "recoil extreme: the {mount} half-rears back toward the upper-right, head tossed; the {rider} leans back, "
           "still in the saddle",
        2: "recovering: the {mount}'s front {feet} come back down; the {rider} sits up, halfway back to the ready pose",
        3: "settle: nearly the ready pose",
    },
    "death": {
        0: "impact: hit from the lower-left; the {mount} throws its head up, the {rider} jolts back, eyes shut",
        1: "stagger: the {mount} rears back toward the upper-right, head thrown back; the {rider} is thrown back in the "
           "saddle, arms flung up, eyes shut",
        2: "the {mount}'s forelegs fold and it drops onto its knees, head sinking; the {rider} pitches back in the "
           "saddle, arms up, eyes shut",
        3: "the {mount} sinks down onto its chest, head sinking, eyes shut; the {rider} sags in the saddle, arms "
           "dropping, eyes shut",
        4: "the {mount} sinks fully onto its belly with its legs folded under it, its neck lowering forward toward the "
           "lower-left; the {rider}, still in the saddle, slumps forward over its neck, the {weapon} slipping from {his} "
           "hand, eyes shut",
        5: "the {mount} lying collapsed on its belly, legs folded under it, its head lying on the ground toward the "
           "lower-left, eyes shut; the {rider} slumped forward over the neck, arms hanging down, the {weapon} on the "
           "ground beside the {mount}'s head, eyes shut",
        6: "bounce: the {rider}'s shoulders lift a hair and the {mount}'s head lifts a little off the ground",
        7: "corpses: the {mount} lying still, collapsed on its belly with its head on the ground, the {rider} slumped "
           "forward over its neck, eyes closed, the {weapon} on the ground beside the head",
    },
}


def text(anim, i, rider_action="swing", mount_action="rear", words=None):
    """The beat text for one cell (lower-left directions), with the unit's words swapped in."""
    w = dict(WORDS, **(words or {}))
    if anim == "strike":
        ph = (PHASE_SHOOT if rider_action == "shoot" else PHASE)[i]
        t = f"{ph}: {RIDER[rider_action][i]}; {MOUNT[mount_action][i]}; both face the lower-left"
    else:
        t = OTHER[anim][i]
    return re.sub(r"\{(\w+)\}", lambda m: w[m.group(1)], t)


_ALL = [t for d in (MOUNT, RIDER) for fr in d.values() for t in fr.values()] + \
       [t for fr in OTHER.values() for t in fr.values()]
assert not any(re.search(r"\b(lower-right|down-right|upper-left|up-left)\b", t, re.I) for t in _ALL)
