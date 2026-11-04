# Leader portraits, HD key-art direction (proposal, not wired in)

A candidate replacement for the leader busts in `../leaders/`: the look of two reference images the
owner supplied on 2026-11-04 (a robed figure under a sun banner and an eclipse; a masked jester-king
in a carnival). Half-length key art instead of a head-and-shoulders bust: the figure in big visible
pixel blocks with a dark outline, turned three-quarters, one hand acting, in front of a softer,
out-of-focus painted vista (framing architecture, story props, a landmark of the leader's people,
a dramatic sky).

- `../style_leader_hd.txt` is the shared preamble; each `<key>.txt` here follows it, exactly as
  `style_leader.txt` + `leaders/<key>.txt` are joined today. Same 38 keys.
- Each description keeps its leader's `VIBE:` moment, likeness and signature item (in capitals)
  from `../leaders/`, re-staged for this composition: a three-quarter pose, what the hands and the
  waist carry, a near/middle/far setting with the civilization's own landmark (so the 38 vistas
  differ) and a `Colours:` line for the grade.

Not yet done, if this direction is adopted:

1. **Anchors.** `leaders.py` sends `anchors/anchor_ui.png` and `anchors/anchor_portrait.png` (the
   16-bit worker bust); those would pull the model back to the old look. Use the two reference
   images instead (they are not in the repo yet: about 3 MB each as PNG).
2. **Cut size.** `leaders.py cut` reduces every painting to 80 art px. This style has about 150
   blocks across and the face is about a fifth of the height, so at 80 px the face is ~16 px and
   the vista is mush. It needs ~150 art px (or the painting as it comes) and a display slot of
   ~300 px or more.
3. **Small slots.** The game shows leaders at 160, 80 and 40 px. A half-length figure does not read
   at 40-80 px; crop the head and shoulders out of the same painting for those slots.
