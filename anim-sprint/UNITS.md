# Which units are animated

Status of the M7 unit sprites (`tools/m7/roster/units/`, the first 56 of them). The method and its limits are in
`ANIMATING_UNITS.md`; this file says what has been done with it, what is waiting and why. Nothing under `crates/` loads
these frames yet, the finished sets included. The hero, religion and unique-unit sprites added to the roster on
2026-10-28 are not animated, except four heroes (below).

## Done: idle, strike, flinch, death

`swordsman` (`final_swordsman/`), `ogre` (`final_ogre/`), `hyborem` (`final_hyborem/`), and since 2026-10-28
`warrior`, `skeleton`, `phalanx`, `lizardman`, `paladin` and `goblin` (`final_<unit>/`, `final_<unit>_nofx/`), each
with a walk (`final_<unit>/walk/`, experimental like the swordsman's). The six were run with `./animate_batch.sh`
(3 sheet calls and 1 walk call each, about $0.31 a unit at the ledger's prices) and picked by eye from
`work/final_<unit>/variants_*.png` (ANIMATING_UNITS.md step 3):

| Unit | strike | flinch | death | walk | Why |
|---|---|---|---|---|---|
| warrior | 2 | 2 | 2 | 1 | strike 2 slides less; flinch 1 swells the hair; death 2 lies on the diagonal, eyes shut |
| skeleton | 2 | 2 | 1 | 1 | flinch 1 swells the skull; death 2 has pink smudges in the corpse |
| phalanx | 2 | 1 | 1 | 2 | strike 1 slides up to 17 px |
| lizardman | 2 | 1 | 1 | 2 | walk 1 needed 20+ px ground snaps |
| paladin | 1 | 1 | 1 | 1 | strike 2 has red specks on frame 5; death 2 throws the mace to the wrong side |
| goblin | 1 | 1 | 2 | 1 | strike 2 grows on frame 1; death 1 needed 25 px snaps; walk 2 16 px |

Every strike lands on the side the sprite looks (lower-right for all six) and every death falls away from it. Like
the first three, these sets have not been judged blind; the strike's turn into the lunge (the shield swinging
behind the back on frames 4-5, "Known limits" in the guide) shows in all five shield units.

Then, the same day, by agents following the guide (step 1 to 4, no re-rolls, one sheet set and walk each):

| Unit | Looks | Pose set | strike | flinch | death | walk | Notes |
|---|---|---|---|---|---|---|---|
| axeman | lower-right | near | 2 | 1 | 2 | 1 | the first two-handed unit: the free hand lets go and the axe stays in the weapon hand; the hit spark lands on his mail shirt (use `_nofx`) |
| champion | lower-left | far | 1 | 1 | 2 | 1 | facing read at 12x (visor cross left of centre, plume sweeping right); redrawn frames a little broader than the slim sprite; the sword tip touches the top edge on strike 1 and death 1 |
| donal | lower-right | far, mirrored | 1 | 2 | 1 | 2 | hero; the empty fist stands in for the shield; the death bounce lifts the torso a bit much |
| losha | lower-right | far, mirrored | 2 | 1 | 2 | 2 | hero; dark sabre, so faint smears; the raised sabre touches the top edge on strike 1 |
| maros | lower-right (nearly frontal) | near | 1 | 1 | 1 | 1 | hero, two-handed axe; the shield on his back pops out on strike 1, 4-5 and the flinch impact |
| guybrush | lower-right | far, mirrored | 2 | 1 | 1 | 2 | hero |

## Not started, same body plan (melee biped), each needs a decision first

- `commander`: one arm outstretched with a gold-pommelled baton, a cape; the far (screen-right) hand.
- `assassin`: two daggers; the recipe holds one weapon (say the other hand is an empty fist, or accept a dagger in each).
- `angel`: sword and shield like the swordsman, plus wings the model must redraw in every pose.
- `worker`: a hoe in both hands (the guide's two-handed notes, now tried on the axeman and Maros); a cold test got as
  far as step 1e.
- No weapon in hand, so no `tip`: `golem` (fists), `werewolf` (claws), `manes`, `treant`, `demon`, `spectre`. Untested:
  the guides always draw a sword blade, so every text must name the fists or claws.
- `hill_giant`: 134 px tall, over the 128 px canvas the recipe refuses (see "Canvas size" in the guide).

## Not startable: the method has no poses for the body plan

The guide says to build a new pose file first, written for the unit's pose set and with all 16 texts overridden, and
says to report it rather than animate it with a sword swing. Not built.

- archers (`archer`, `hunter`, `crossbowman`, `ljosalfar_archer`): a shot instead of a swing;
- casters and civilians (`adept`, `mage`, `archmage`, `disciple`, `priest`, `priest_of_leaves`, `druid`, `lich`,
  `sheaim_mage`, `great_person`, `scout`, `settler`): a cast, a point, a walk-and-look;
- mounted and flying (`horseman`, `knight`, `chariot`, `war_elephant`, `griffon`);
- animals (`bear`, `lion`, `wolf`, `giant_spider`, `sea_serpent`);
- ships (`frigate`, `trireme`, `workboat`), siege (`catapult`);
- elementals (`air_elemental`, `earth_elemental`, `fire_elemental`, `water_elemental`).

Their idle is the one thing that needs no model: the same mesh-warp breath, with joints placed for the body plan. It
has not been tried on a non-biped.
