#!/bin/sh
# ai-branch.sh: fork a game at a firing of an AI behaviour and play both branches -- the first
# step in judging an AI change (CLAUDE.md, "AI changes"; docs/AI.md, "Tools").
#
#   tools/ai-branch.sh --bin DIR (--switch NAME | --bin2 DIR2) [--seed S --turn T | --save FILE]
#                      [--turns N] [--rolls R] [--trace P] [--out DIR] [--keep NAME]
#                      [run options: --civs N --size WORLDSIZE_X ...] -- WATCH...
#
# Plays seed S (as `ffh stats` does, run options included) to the start of turn T with the
# behaviour on, saves it, then resumes the save twice for N turns (default 10): the behaviour on
# (FFH_AI_<NAME>_ONLY=ALL) and off (FFH_AI_<NAME>_OFF=ALL). Up to the fork the branches are the
# same game. Prints each WATCH (`ffh resume --watch`: city:X,Y, player:P, area:X,Y,R, unit:ID)
# turn by turn for both, marking the lines that differ; with --trace P, also writes player P's
# decision trace (FFH_AI_TRACE) of each branch to the output directory. DIR is a build output
# (`tools/agents/build-tree.sh --bins ffh`). Find the firings first: the behaviour's trace lines
# (FFH_AI_TRACE=all) or telemetry name the turn, the player and the place. --save FILE resumes an
# existing save (an earlier fork's fork.ffhsave, or a kept position) instead of playing to turn T:
# after fixing the behaviour, replay the same position with the new binary.
#
# --rolls R (default 1: the above) resumes each branch R times, `ffh resume --reseed K --summary`
# for K = 1..R (the same K on both sides, so rolls pair), each holding a CPU slot, in parallel,
# and prints per watched value the distribution over the rolls (tools/ai-branch-rolls.py): mean
# [min..max] per branch, the difference of the means and a paired t; counts for owners and
# alive. Per-roll JSON lines: DIR/on.rolls.jsonl, DIR/off.rolls.jsonl.
# --bin2 DIR2 instead of --switch: no switch; the same fork resumed with binary DIR and DIR2
# (branches `bin` / `bin2`), to compare two builds on one position. Both must load the save
# (the same save::FORMAT_VERSION). Without --save, the fork is played with DIR.
# --keep NAME copies the fork save to the position bank, target/ai-positions/NAME.ffhsave in the
# main checkout (FFH_POSITIONS to override), with NAME.txt beside it: seed, turn, run options,
# switch, watches, the build and git revision. Saves load only with builds of the same
# save::FORMAT_VERSION; the bank is a local working set, never committed (target/ is ignored).
set -e
BIN= BIN2= SEED= TURN= SWITCH= TURNS=10 TRACE= OUT= FROM= ROLLS=1 KEEP=
RUN="--civs 6 --size WORLDSIZE_STANDARD"
EXTRA=
while [ $# -gt 0 ]; do
  case "$1" in
    --bin) BIN=$2; shift 2 ;;
    --bin2) BIN2=$2; shift 2 ;;
    --seed) SEED=$2; shift 2 ;;
    --turn) TURN=$2; shift 2 ;;
    --switch) SWITCH=$2; shift 2 ;;
    --turns) TURNS=$2; shift 2 ;;
    --trace) TRACE=$2; shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    --save) FROM=$2; shift 2 ;;
    --rolls) ROLLS=$2; shift 2 ;;
    --keep) KEEP=$2; shift 2 ;;
    --) shift; break ;;
    *) EXTRA="$EXTRA $1"; shift ;;
  esac
done
usage() { sed -n '2,32p' "$0"; exit 2; }
[ -n "$BIN" ] || usage
{ [ -n "$SWITCH" ] && [ -z "$BIN2" ]; } || { [ -z "$SWITCH" ] && [ -n "$BIN2" ]; } || { echo "ai-branch.sh: give one of --switch NAME and --bin2 DIR2" >&2; usage; }
[ -n "$FROM" ] || { [ -n "$SEED" ] && [ -n "$TURN" ]; } || { echo "ai-branch.sh: --seed S --turn T, or --save FILE" >&2; usage; }
[ -n "$EXTRA" ] && RUN=$EXTRA
TAG=${SWITCH:-bins}
if [ -n "$SEED$TURN" ]; then NAME="s$SEED-t$TURN-$TAG"; else NAME="$(basename "$FROM" .ffhsave)-$TAG"; fi
OUT=${OUT:-${TMPDIR:-/tmp}/ffh-branch/$NAME}
mkdir -p "$OUT"
HERE="$(dirname "$0")"
SLOT="$HERE/agents/slot.sh"
if [ -n "$SWITCH" ]; then
  A=on B=off BA=$BIN BB=$BIN
  EA="FFH_AI_${SWITCH}_ONLY=ALL" EB="FFH_AI_${SWITCH}_OFF=ALL"
else
  A=bin B=bin2 BA=$BIN BB=$BIN2 EA=- EB=-
fi
# `env -` would clear the environment: a side without a switch sets a harmless variable instead.
envof() { if [ "$1" = - ]; then echo "FFH_BRANCH_SIDE=1"; else echo "$1"; fi; }
WATCH=
for w in "$@"; do WATCH="$WATCH --watch $w"; done

SAVE="$OUT/fork.ffhsave"
if [ -n "$FROM" ]; then
  [ "$FROM" -ef "$SAVE" ] || cp "$FROM" "$SAVE"
else
  env "$(envof "$EA")" "$SLOT" "$BIN/ffh" save "$SAVE" --to-turn "$TURN" --seed "$SEED" $RUN >/dev/null
fi

if [ -n "$KEEP" ]; then
  if [ -n "$FFH_POSITIONS" ]; then BANK=$FFH_POSITIONS; else
    COMMON=$(git -C "$HERE" rev-parse --path-format=absolute --git-common-dir)
    BANK="$(dirname "$COMMON")/target/ai-positions"
  fi
  mkdir -p "$BANK"
  cp "$SAVE" "$BANK/$KEEP.ffhsave"
  {
    echo "seed: ${SEED:-?}"
    echo "turn: ${TURN:-?}"
    [ -n "$FROM" ] && echo "from save: $FROM"
    echo "run options: $RUN"
    echo "switch: ${SWITCH:-none (--bin2)}"
    echo "watches: $*"
    echo "binary: $BIN ($(sed -n 's/.*"source": "\(.*\)",/\1/p' "$BIN/info.json" 2>/dev/null))"
    echo "git revision: $(git -C "$HERE" rev-parse HEAD)$(git -C "$HERE" diff --quiet HEAD -- 2>/dev/null || echo ' +uncommitted')"
    echo "save format: $(sed -n 's/^pub const FORMAT_VERSION: u32 = \([0-9]*\);/\1/p' "$HERE/../crates/ffh-core/src/save/mod.rs") (crates/ffh-core/src/save/mod.rs)"
  } >"$BANK/$KEEP.txt"
  echo "kept: $BANK/$KEEP.ffhsave"
fi

if [ "$ROLLS" -gt 1 ]; then
  if [ -n "$SWITCH" ]; then echo "fork $NAME: $SWITCH on / off, $TURNS turns, $ROLLS rolls ($OUT)"
  else echo "fork $NAME: $BIN vs $BIN2, $TURNS turns, $ROLLS rolls ($OUT)"; fi
  exec python3 "$HERE/ai-branch-rolls.py" --save "$SAVE" --turns "$TURNS" --rolls "$ROLLS" --out "$OUT" \
    ${TRACE:+--trace "$TRACE"} --side "$A" "$BA/ffh" "$EA" --side "$B" "$BB/ffh" "$EB" -- "$@"
fi

for side in $A $B; do
  if [ $side = $A ]; then E=$(envof "$EA") X=$BA; else E=$(envof "$EB") X=$BB; fi
  if [ -n "$TRACE" ]; then
    env "$E" FFH_AI_TRACE="$TRACE" "$SLOT" "$X/ffh" resume "$SAVE" --turns "$TURNS" $WATCH >"$OUT/$side.txt" 2>"$OUT/$side.trace" || { tail -1 "$OUT/$side.trace" >&2; exit 1; }
  else
    env "$E" "$SLOT" "$X/ffh" resume "$SAVE" --turns "$TURNS" $WATCH >"$OUT/$side.txt" 2>"$OUT/$side.err" || { tail -1 "$OUT/$side.err" >&2; exit 1; }
    rm -f "$OUT/$side.err"
  fi
done
if [ -n "$SWITCH" ]; then echo "seed ${SEED:-?} forked at turn ${TURN:-?}, $SWITCH on / off, $TURNS turns ($OUT)"
else echo "fork $NAME: $BIN vs $BIN2, $TURNS turns ($OUT)"; fi
awk -v a="$A" -v b="$B" 'BEGIN { la = sprintf("%-4s: ", a); lb = sprintf("%-4s: ", b); if (a == "on") { la = "on : "; lb = "off: " } }
     NR == FNR { on[FNR] = $0; n = FNR; next }
     { mark = (on[FNR] == $0) ? "  " : "* "; print mark la on[FNR]; if (mark == "* ") print mark lb $0 }' "$OUT/$A.txt" "$OUT/$B.txt"
[ -n "$TRACE" ] && echo "traces: $OUT/$A.trace $OUT/$B.trace"
exit 0
