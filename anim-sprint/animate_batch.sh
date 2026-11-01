#!/usr/bin/env bash
# The paid steps of ANIMATING_UNITS.md for units that already have a units/<unit>.json: the three sheets
# (strike, flinch, death: about $0.23) and the walk (about $0.08), one unit after another.
#
#   ./animate_batch.sh                  # every unit listed in UNITS.md as "ready"
#   ./animate_batch.sh warrior goblin   # just these
#
# It stops at the first failure (an empty OpenAI balance fails every call with 429 credit_balance_exhausted), so a
# queue cannot burn money on retries. It does NOT pick variants: afterwards open work/final_<unit>/variants_*.png,
# choose by eye (ANIMATING_UNITS.md step 3) and run `--rebuild --pick ...`, then lib/preview.py.
set -euo pipefail
cd "$(dirname "$0")"
units=("$@")
if [ ${#units[@]} -eq 0 ]; then
  units=(warrior skeleton phalanx lizardman paladin goblin)
fi
for u in "${units[@]}"; do
  [ -f "units/$u.json" ] || { echo "no units/$u.json" >&2; exit 1; }
  echo "== $u: sheets"
  python3 lib/recipe.py --unit "units/$u.json" --out "final_$u"
  echo "== $u: walk"
  python3 lib/walk.py --unit "units/$u.json" --out "final_$u"
done
echo "done: ${units[*]}. Now pick variants by eye (work/final_<unit>/variants_*.png, variants_walk.png)."
