# Rules for the key-pose prompts

`chunk_*.prompts.json` hold, per unit, `{name, who, attack, hit, unconventional}`, written on 5 November 2026 by six
agents from `roster.json` (the anim-all roster's facing, weapon and description per sprite) under these rules. Each
pose is drawn by an image model from the unit's own sprite: `the character from the first image, <who>, redrawn
<attack|hit>.` (lib/prompt_pose.py), or as a cell of a sprite sheet (lib/pose_sheet.py).

- `who`: 6-14 words naming what the unit is and looks like.
- `attack`: 20-45 words, one frozen key frame at the instant of the blow, the whole body in a dynamic pose; held about
  a second in battle, so readable and dramatic. `hit`: 20-40 words, the instant it is struck, recoiling.
1. Direction: toward the side the sprite faces ("toward the lower right" / "lower left"; a front-facing sprite: lower
   right). In `hit` it is struck from that side and recoils away from it.
2. The unit's own identity, items and anatomy only: no new weapons, same limbs and heads, a rider stays on the same
   mount, a quadruped stays one. Name the held item.
3. Unconventional attacks (the owner's request) wherever the unit's kind and what it carries allow: a scout clubbing
   with his spyglass, a ranger loosing his hawk, a worker swinging his shovel flat, a settler hurling a cooking pot, a
   priest smiting with the holy book, a horse rearing to kick. Elite units and heroes may do a spectacular version of
   their real attack. About half; `unconventional: true`.
4. Casters and healers: the attack frame also shows their spell or heal, so a pose that works for both (the staff,
   hand or holy symbol thrust toward the target, magic flaring).
5. Archers and gunners: loosing or firing toward the target (may be unconventional).
6. No other characters, ground, text, or effects leaving the figure; the whole figure and weapon inside the picture.
7. Hit poses varied (staggering, doubled over, head snapped back, shield raised too late, mount shying), items still
   held or slipping; wincing, gritted teeth.
8. Plain English, no style words, no names of games or artists.
9. Melee attacks are caught mid-lunge at the instant of contact (the owner's note, 5 November 2026): the body thrown
   forward, the front foot planted, the weapon or limb fully extended at the end of its swing or thrust; never raised
   or drawn back about to strike. The descriptions predate this note and some read as wind-ups ("swung down
   overhead"), so the sheet prompt (lib/pose_sheet.py) states it over them.
