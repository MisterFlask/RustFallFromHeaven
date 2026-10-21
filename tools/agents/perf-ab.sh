#!/bin/sh
# perf-ab.sh [--base X] [--new X] same [--set quick|dev|val|all] [--soak] | time [--rounds N] [--config C]
# A/B for behaviour-preserving performance work: bit-identical games, then interleaved CPU timing.
# See docs/PERF.md; the logic is in perf_ab.py (python3, std only).
exec python3 "$(dirname "$0")/perf_ab.py" "$@"
