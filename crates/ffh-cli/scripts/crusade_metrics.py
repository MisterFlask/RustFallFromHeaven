#!/usr/bin/env python3
"""Crusade self-play measures (docs/CRUSADE.md, "Self-play measures").

Plays `ffh autoplay` games (in parallel) with council crusades injected against the
leading civ at the `--call` turns (AI councils rarely pass war resolutions on their own), one
player made a free rider (it pledges the minimum to every call), and reads the dissolved
crusades the runner prints (`crusade {...}` lines). Prints:

- engagement: share of delivered Host strength that fought within `engage_turns` of delivery;
- bandwagon leak: share of spoils going to strength lent in the final quarter of a crusade
  (each lender's spoils x its late credit / its credit);
- free-rider probe: the free rider's share of the spoils of the crusades it was called to,
  next to its share of the delivered strength and its fair 1/(lenders) share;
- refusals, crusades that took cities, dissolution reasons, loans.

Host competence (the Host against a scripted defender) is a constructed scenario:
`cargo test --release -p ffh-ai --test crusade -- --ignored --nocapture`.

usage: crusade_metrics.py [--bin target/release/ffh] [--seeds 1-8] [--civs 6]
                          [--size WORLDSIZE_SMALL] [--turns 300] [--call 120,200]
                          [--free-rider 1] [--jobs 4] [--out results.jsonl]
"""
import argparse
import json
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor


def seeds(spec):
    out = []
    for part in spec.split(','):
        if '-' in part:
            a, b = part.split('-')
            out.extend(range(int(a), int(b) + 1))
        else:
            out.append(int(part))
    return out


def play(args, seed):
    cmd = [args.bin, 'autoplay', '--seed', str(seed), '--civs', str(args.civs), '--size', args.size,
           '--turns', str(args.turns), '--crusade-call', args.call]
    if args.free_rider is not None:
        cmd += ['--free-rider', str(args.free_rider)]
    out = subprocess.run(cmd, capture_output=True, text=True, check=True).stdout
    reports = []
    for line in out.splitlines():
        if line.startswith('crusade '):
            r = json.loads(line[len('crusade '):])
            r['seed'] = seed
            reports.append(r)
    return reports


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--bin', default='target/release/ffh')
    ap.add_argument('--seeds', default='1-8')
    ap.add_argument('--civs', type=int, default=6)
    ap.add_argument('--size', default='WORLDSIZE_SMALL')
    ap.add_argument('--turns', type=int, default=300)
    ap.add_argument('--call', default='120,200')
    ap.add_argument('--free-rider', type=int, default=1)
    ap.add_argument('--jobs', type=int, default=4)
    ap.add_argument('--out')
    args = ap.parse_args()

    with ThreadPoolExecutor(args.jobs) as ex:
        games = list(ex.map(lambda s: play(args, s), seeds(args.seeds)))
    reports = [r for g in games for r in g]
    if args.out:
        with open(args.out, 'w') as f:
            for r in reports:
                f.write(json.dumps(r) + '\n')

    council = [r for r in reports if r['kind'] == 'council']
    loans = [r for r in reports if r['kind'] == 'loan']
    print(f"{len(games)} games, {len(council)} council crusades, {len(loans)} loan generals")

    def engagement(rs):
        d = sum(r['delivered'] for r in rs)
        return sum(r['engaged'] for r in rs) / d if d else float('nan')

    print(f"engagement: council {engagement(council):.0%}, loans {engagement(loans):.0%} "
          f"(share of delivered strength that fought within the engage window)")

    spoils = late = 0.0
    for r in council:
        for l in r['lenders']:
            spoils += l['spoils']
            if l['credit'] > 0:
                late += l['spoils'] * l['late_credit'] / l['credit']
    print(f"bandwagon leak: {late / spoils:.0%} of {spoils:.0f} spoils went to strength lent in the final quarter"
          if spoils else "bandwagon leak: no spoils were shared")

    if args.free_rider is not None:
        fr_spoils = fr_deliv = all_spoils = all_deliv = 0
        fair = []
        for r in council:
            called = [l for l in r['lenders'] if l['called']]
            if not any(l['player'] == args.free_rider for l in called):
                continue
            lenders = [l for l in r['lenders'] if l['delivered'] > 0]
            fair.append(1 / max(1, len(lenders)))
            for l in r['lenders']:
                all_spoils += l['spoils']
                all_deliv += l['delivered']
                if l['player'] == args.free_rider:
                    fr_spoils += l['spoils']
                    fr_deliv += l['delivered']
        share = fr_spoils / all_spoils if all_spoils else float('nan')
        dshare = fr_deliv / all_deliv if all_deliv else float('nan')
        fairs = sum(fair) / len(fair) if fair else float('nan')
        print(f"free rider (player {args.free_rider}, pledges the minimum): {share:.0%} of the spoils, "
              f"{dshare:.0%} of the delivered strength, fair share {fairs:.0%} ({len(fair)} calls)")

    called = sum(1 for r in council for l in r['lenders'] if l['called'])
    refused = sum(r['refusals'] for r in council)
    print(f"refusals: {refused} of {called} calls")
    took = sum(1 for r in council + loans if any(l['cities'] for l in r['lenders']))
    pool = sum(r['pool'] for r in council)
    print(f"crusades whose draft gave out a city: {took}; council pools {pool} gold-equivalent in total")
    reasons = {}
    for r in reports:
        reasons[(r['kind'], r['reason'])] = reasons.get((r['kind'], r['reason']), 0) + 1
    print("dissolutions: " + ", ".join(f"{k}/{v}: {n}" for (k, v), n in sorted(reasons.items())))


if __name__ == '__main__':
    sys.exit(main())
