# Which units are animated

Status of the M7 unit sprites (`tools/m7/roster/units/`, 56 of them). The method and its limits are in
`ANIMATING_UNITS.md`; this file says what has been done with it, what is waiting and why. Nothing under `crates/` loads
these frames yet, the three finished sets included.

## Done: idle, strike, flinch, death

`swordsman` (`final_swordsman/`), `ogre` (`final_ogre/`), `hyborem` (`final_hyborem/`). None has a walk yet (only the
experimental `final_swordsman/walk/`).

## Idle done, the rest waiting for image credit

Unit file written, guides checked (`--check-guides`, `lib/walk.py --check-guides`), idle built and looked at (frame 0 is
the exact sprite, no tearing, a one-pixel breath). Strike, flinch, death and walk need the image model, and the OpenAI
account behind the session key had no credit on 2026-10-26: every call failed with `429 credit_balance_exhausted` (the
key and the model were fine; nothing was billed). Once the balance is back:

```bash
cd anim-sprint
./animate_batch.sh                  # the six units below: 3 sheet calls + 1 walk call each, about $0.31 a unit
# then, per unit, ANIMATING_UNITS.md step 3 and 4: pick variants by eye, --rebuild --pick, lib/preview.py
```

| Unit | Looks | Weapon | Pose set | Notes |
|---|---|---|---|---|
| warrior | lower-right | club, shield | near | the pilot; walk guides checked |
| skeleton | lower-right | sword, shield | near | no hair, no eyes to squeeze shut: texts reworded |
| phalanx | lower-right | long spear, shield | near | the spear is longer than the canvas area in strikes 1-5: expect clipping, reject variants that cut it |
| lizardman | lower-right | long spear, shield | near | tail and crest in the texts |
| paladin | lower-right | mace, kite shield | near | |
| goblin | lower-right | spear in the far hand (screen-right), no shield | far, mirrored | 56 px tall (`scale` 0.65); its other hand is "an empty clawed hand" in the texts |

To watch the idles: `previews/idle_roster.{gif,mp4}` (all six side by side) and `final_<unit>/idle_3x.{gif,mp4}`, made
by `python3 lib/preview_idle.py` (free; `lib/preview.py` needs a full set).

`animate_batch.sh` stops at its first failed call. `anim/units` in `budget.json` was raised from $5 to $12 for the
batch (the ledger prices are deliberately high, so the real bill is lower; the $20 `anim` and `total` caps still bind).

## Not started, same body plan (melee biped), each needs a decision first

- `champion`, `axeman`: weapon held in both hands. The guide says no two-handed unit has been generated; do one and
  check the strike closely before queueing the other. The champion's facing is also unclear (the visor slit is on the
  left of the helmet and the plume sweeps right: probably lower-left, check at 8×).
- `commander`: one arm outstretched with a gold-pommelled baton, a cape; the far (screen-right) hand.
- `assassin`: two daggers; the recipe holds one weapon (say the other hand is an empty fist, or accept a dagger in each).
- `angel`: sword and shield like the swordsman, plus wings the model must redraw in every pose.
- `worker`: a hoe in both hands (the guide's two-handed notes); a cold test got as far as step 1e.
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
