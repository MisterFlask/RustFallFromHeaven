"""Beat texts for the shot strike (lib/poses_shot_src.py -> poses_shot.json): a ranged shot with a bow, by a
shooter that looks LOWER-LEFT with the bow in the far, screen-left hand and draws with the other. Strike frames 1-6
are the cells the image model draws; frame 0 (the ready pose) and frame 7 (the settle) come from the real sprite.
Timing as lib/beats.py (100, 120, 160, 50, 120, 120, 100, 100 ms); the engine fires "hit" on frame 4, the release, and
plays the impact on the defender there (no arrow is drawn in flight).

"bow" and "arrow" are the generic prop words: strike_beats("longbow") / ("crossbow", "bolt") swaps them for a unit's
own. A crossbow or gun has no string to draw: such a unit rewrites the draw-hand clauses (frames 1-5) in its unit file
(the free hand cranks or steadies the stock, then is thrown back by the recoil on frame 4). Directions are written for
the lower-left facing and pass recipe.wrong_way for a lower-left unit (no lower-right / upper-left); a mirrored use
needs beats.swap_sides.
"""
import re

STRIKE_TEXT = {
    1: "nock: a small dip, the knees bend a little and the torso leans slightly forward, the head bowed to look at the "
       "bow; the bow hand lifts the bow upright in front of the chest, its top tilted forward toward the lower-left; "
       "the draw hand comes across to the string at the grip and sets an arrow on it; both feet planted; the face looks "
       "to the lower-left",
    2: "full draw, the wind-up extreme: the torso coils back toward the upper-right, crouched on the bent rear leg, "
       "the front leg reaching out to the lower-left with its foot barely touching the ground; the bow arm pushed "
       "straight out to the left and a little upward, the bow upright, aimed above the target; the string pulled all "
       "the way back, the draw hand anchored at the jaw below the ear and the draw elbow cocked high and far back to "
       "the upper-right; the face looks along the arrow to the left",
    3: "the aim snaps down: the torso drives forward, the front foot stamps down toward the lower-left and the weight "
       "moves onto it; the bow arm swings down from high onto the target, straight out to the lower-left with the bow "
       "upright; still at full draw, the draw hand at the jaw and the draw elbow back; the face looks along the arrow "
       "to the lower-left",
    4: "release, the moment the arrow flies: a deep lunge toward the lower-left, front knee bent, back leg straight, "
       "torso low and leaning in; the bow arm locked straight toward the lower-left, the bow upright with its top "
       "rolling forward toward the target, the string straight and empty; the draw hand has snapped back past the ear "
       "and out to the upper-right, fingers spread open, so the bow, both arms and the draw hand make one long line; "
       "the face looks along the shot to the lower-left",
    5: "follow-through: the bow keeps rolling forward, its top tipping toward the lower-left as the bow hand dips; the "
       "draw hand carries on up and back to the upper-right, the arm almost straight, fingers open; the torso "
       "rebounds up a little from the lunge, both feet still planted wide; the face looks to the lower-left",
    6: "recover: the torso straightens and rises, the front foot pulls back halfway, the bow comes back down to its "
       "carrying angle at the side, the draw hand drops back to the hip; still facing the lower-left",
}


def strike_beats(prop="bow", missile="arrow"):
    """{"1": text, ..} for a unit file's "beats": {"strike": ...}, with the prop and missile words swapped in."""
    return {str(i): re.sub(r"\barrow\b", missile, re.sub(r"\bbow\b", prop, t)) for i, t in STRIKE_TEXT.items()}


assert sorted(STRIKE_TEXT) == [1, 2, 3, 4, 5, 6]
assert not any(re.search(r"\b(lower-right|down-right|upper-left|up-left)\b", t, re.I) for t in STRIKE_TEXT.values())
assert not any(re.search(r"\b(sword|blade|shield|staff)\b", t, re.I) for t in STRIKE_TEXT.values())
