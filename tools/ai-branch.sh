#!/bin/sh
# ai-branch.sh: fork a game at a firing of an AI behaviour and play both branches -- the first
# step in judging an AI change (CLAUDE.md, "AI changes"; docs/AI.md, "Tools").
#
#   tools/ai-branch.sh --bin DIR --seed S --turn T --switch NAME [--turns N] [--trace P]
#                      [--out DIR] [--save FILE] [run options: --civs N --size WORLDSIZE_X ...] -- WATCH...
#
# Plays seed S (as `ffh stats` does, run options included) to the start of turn T with the
# behaviour on, saves it, then resumes the save twice for N turns (default 10): the behaviour on
# (FFH_AI_<NAME>_ONLY=ALL) and off (FFH_AI_<NAME>_OFF=ALL). Up to the fork the branches are the
# same game. Prints each WATCH (`ffh resume --watch`: city:X,Y, player:P, area:X,Y,R, unit:ID)
# turn by turn for both, marking the lines that differ; with --trace P, also writes player P's
# decision trace (FFH_AI_TRACE) of each branch to the output directory. DIR is a build output
# (`tools/agents/build-tree.sh --bins ffh`). Find the firings first: the behaviour's trace lines
# (FFH_AI_TRACE=all) or telemetry name the turn, the player and the place. --save FILE resumes an
# existing save (an earlier fork's fork.ffhsave) instead of playing to turn T: after fixing the
# behaviour, replay the same position with the new binary (--seed and --turn then only name it).
set -e
BIN= SEED= TURN= SWITCH= TURNS=10 TRACE= OUT= FROM=
RUN="--civs 6 --size WORLDSIZE_STANDARD"
EXTRA=
while [ $# -gt 0 ]; do
  case "$1" in
    --bin) BIN=$2; shift 2 ;;
    --seed) SEED=$2; shift 2 ;;
    --turn) TURN=$2; shift 2 ;;
    --switch) SWITCH=$2; shift 2 ;;
    --turns) TURNS=$2; shift 2 ;;
    --trace) TRACE=$2; shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    --save) FROM=$2; shift 2 ;;
    --) shift; break ;;
    *) EXTRA="$EXTRA $1"; shift ;;
  esac
done
[ -n "$BIN" ] && [ -n "$SEED" ] && [ -n "$TURN" ] && [ -n "$SWITCH" ] || { sed -n '2,17p' "$0"; exit 2; }
[ -n "$EXTRA" ] && RUN=$EXTRA
OUT=${OUT:-${TMPDIR:-/tmp}/ffh-branch/s$SEED-t$TURN-$SWITCH}
mkdir -p "$OUT"
SLOT="$(dirname "$0")/agents/slot.sh"
ON="FFH_AI_${SWITCH}_ONLY=ALL"
OFF="FFH_AI_${SWITCH}_OFF=ALL"
WATCH=
for w in "$@"; do WATCH="$WATCH --watch $w"; done

SAVE="$OUT/fork.ffhsave"
if [ -n "$FROM" ]; then
  [ "$FROM" -ef "$SAVE" ] || cp "$FROM" "$SAVE"
else
  env "$ON" "$SLOT" "$BIN/ffh" save "$SAVE" --to-turn "$TURN" --seed "$SEED" $RUN >/dev/null
fi
for side in on off; do
  if [ $side = on ]; then E=$ON; else E=$OFF; fi
  if [ -n "$TRACE" ]; then
    env "$E" FFH_AI_TRACE="$TRACE" "$SLOT" "$BIN/ffh" resume "$SAVE" --turns "$TURNS" $WATCH >"$OUT/$side.txt" 2>"$OUT/$side.trace"
  else
    env "$E" "$SLOT" "$BIN/ffh" resume "$SAVE" --turns "$TURNS" $WATCH >"$OUT/$side.txt" 2>/dev/null
  fi
done
echo "seed $SEED forked at turn $TURN, $SWITCH on / off, $TURNS turns ($OUT)"
awk 'NR == FNR { on[FNR] = $0; n = FNR; next }
     { mark = (on[FNR] == $0) ? "  " : "* "; print mark "on : " on[FNR]; if (mark == "* ") print mark "off: " $0 }' "$OUT/on.txt" "$OUT/off.txt"
[ -n "$TRACE" ] && echo "traces: $OUT/on.trace $OUT/off.trace"
exit 0
