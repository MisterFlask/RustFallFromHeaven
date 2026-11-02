"""Beat texts for the cast strike (lib/poses_cast_src.py -> poses_cast.json): a spell cast with a staff, by a
caster that looks LOWER-LEFT with the staff in the far, screen-left hand. Strike frames 1-6 are the cells the image
model draws; frame 0 (the ready pose) and frame 7 (the settle) come from the real sprite. Timing as lib/beats.py
(100, 120, 160, 50, 120, 120, 100, 100 ms); the engine fires "hit" on frame 4 and draws the spell itself there.

"staff" is the generic prop word: strike_beats("wand") / ("gnarled staff") swaps it for a unit's own prop. A unit whose
free hand holds something (the disciple's book, the lich's orb) rewrites the free-hand clauses in its unit file.
Directions are written for the lower-left facing and pass recipe.wrong_way for a lower-left unit (no lower-right /
upper-left); a mirrored use needs beats.swap_sides.
"""
import re

STRIKE_TEXT = {
    1: "gather: weight shifts back onto the rear leg and the torso leans back toward the upper-right; the staff hand "
       "rises to chest height in front of the body, holding the staff upright with its head lifted above the head; the "
       "free hand opens out at the side, palm up; both feet stay planted and the face still looks to the lower-left",
    2: "charge, the wind-up extreme: body arched back toward the upper-right and crouched on the bent rear leg, the "
       "front leg straight and its foot still planted; the staff raised high overhead at full arm's length and tilted "
       "back, its head up behind the head; the free arm raised high out to the right, palm open; the face still looks "
       "to the lower-left",
    3: "swing: the front foot steps toward the lower-left and the torso whips forward; the staff arm swings forward "
       "over the top and the staff is level, its head pointing to the left at head height; the free arm sweeps down "
       "and back; the face looks to the lower-left",
    4: "release, the moment the spell leaves the staff: a deep lunge toward the lower-left, front knee bent, back leg "
       "straight, body low and leaning in; the staff arm fully extended toward the lower-left and the staff thrust out "
       "in line with it, aimed at the lower-left, its head the farthest point of the whole figure; the free arm flung "
       "back behind the hip, palm open; the face looks along the staff to the lower-left",
    5: "hold: the staff stays thrust out toward the lower-left, its head dipping a little lower; the body stays low in "
       "the lunge and sinks a touch, both feet planted; the free arm lowers behind the hip; the face looks to the "
       "lower-left",
    6: "recover: the body rises, the front foot pulls back halfway, the staff comes back up to its ready angle beside "
       "the body, the free hand returns to the side; still facing the lower-left",
}


def strike_beats(prop="staff"):
    """{"1": text, ..} for a unit file's "beats": {"strike": ...}, with the prop word swapped in."""
    return {str(i): re.sub(r"\bstaff\b", prop, t) for i, t in STRIKE_TEXT.items()}


assert sorted(STRIKE_TEXT) == [1, 2, 3, 4, 5, 6]
assert not any(re.search(r"\b(lower-right|down-right|upper-left|up-left)\b", t, re.I) for t in STRIKE_TEXT.values())
