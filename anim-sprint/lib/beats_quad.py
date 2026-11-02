"""Beat texts for the quadruped pose set (lib/poses_quad_src.py -> poses_quad.json): a four-legged animal that looks
LOWER-LEFT and fights with its jaws and claws. The cells the image model draws are strike 1-6, flinch 0-2 and death 1-7
(death 0 is flinch 0); timings and the "hit" event on strike 4 are lib/beats.py's.

Generic words in braces are swapped for the unit's own (a unit file's "words": {"jaws": "beak", "claws": "talons",
"paws": "hooves", "tail": "bushy tail", "ears": "horns"}); text(anim, i, words) fills them. A unit whose body differs
more (a dragon's wings, an elephant's trunk) rewrites the frames that matter in its own "beats". Directions are written
for the lower-left facing and pass recipe.wrong_way for a lower-left unit (no lower-right / upper-left); quad.beat_text
mirrors them with beats.swap_sides for a unit that looks lower-right.
"""
import re

WORDS = {"jaws": "jaws", "claws": "claws", "paws": "paws", "tail": "tail", "ears": "ears", "animal": "animal"}

TEXT = {
    "strike": {
        1: "crouch: the weight shifts back onto the hind legs and the body sinks lower, the head drops level with the "
           "shoulders and aims at the lower-left, all four {paws} planted, the {jaws} closed",
        2: "coil, the wind-up extreme: the body drawn back toward the upper-right and pressed low over the folded hind "
           "legs, the chest close to the ground, the head low and aimed at the lower-left, the {tail} held stiff out "
           "behind, the {jaws} closed; the face looks to the lower-left",
        3: "launch: the hind legs drive and the body springs forward and up toward the lower-left, stretched out, the "
           "forelegs lifted off the ground and reaching forward, the {jaws} opening; the hind {paws} still on the "
           "ground",
        4: "contact: the body fully stretched out low toward the lower-left, the head the farthest point of the figure "
           "with the {jaws} wide open biting at the lower-left, both forelegs reaching out toward the lower-left with "
           "the {claws} spread, the hind {paws} still planted, the {tail} streaming back",
        5: "follow-through: the front {paws} land on the ground ahead at the lower-left, the head bites down low with "
           "the {jaws} closing, the hind legs drawn up under the body",
        6: "recover: the body pulls back halfway toward where it stood, the head rises, the {jaws} closing, the front "
           "{paws} stepping back",
    },
    "flinch": {
        0: "impact: hit from the lower-left; the head tossed up and back toward the upper-right, eyes squeezed shut, "
           "the {jaws} open in a yelp, the body jolts back toward the upper-right and the front {paws} leave the ground",
        1: "recoil extreme: half-rearing back toward the upper-right on the hind legs, the head still thrown back, the "
           "{tail} tucked low",
        2: "recovering: the front {paws} back on the ground, the head coming forward, halfway back to the ready pose",
        3: "settle: nearly the ready pose",
    },
    "death": {
        0: "impact: hit from the lower-left; the head tossed up and back, eyes squeezed shut, the body jolts back",
        1: "stagger back: thrown up and back onto the hind legs toward the upper-right, the forelegs dangling, the head "
           "thrown back, eyes shut",
        2: "legs buckle: the front {paws} come down and fold, the haunches sink, the head droops low, eyes shut",
        3: "collapse: down on its belly with the legs folded under it, the head sinking toward the ground, eyes shut",
        4: "rolls over onto its side: the body goes down flat, the legs swing out stiff and straight, the head flops "
           "onto the ground with the {ears} laid back, eyes shut",
        5: "hits the ground: lying on its side along the ground, its legs stretched out stiff and straight (forelegs "
           "forward, hind legs back), the head lying flat on the ground with the {ears} laid back and the jaw slack, "
           "the {tail} lying flat behind, eyes shut",
        6: "bounce: the head and the stiff legs lift a little off the ground",
        7: "corpse: lying still on its side, legs stretched out, eyes closed",
    },
}


def text(anim, i, words=None):
    """The beat text for one cell, with the unit's words swapped in (lower-left directions)."""
    w = dict(WORDS, **(words or {}))
    return re.sub(r"\{(\w+)\}", lambda m: w[m.group(1)], TEXT[anim][i])


assert sorted(TEXT["strike"]) == [1, 2, 3, 4, 5, 6] and sorted(TEXT["death"]) == list(range(8))
assert not any(re.search(r"\b(lower-right|down-right|upper-left|up-left)\b", t, re.I)
               for fr in TEXT.values() for t in fr.values())
