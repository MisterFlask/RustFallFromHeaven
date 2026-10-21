#!/usr/bin/env python3
"""Head to head: the FFH2 C++ AI port (ffh-cppai) against ffh-ai in the same games.

Each seed is played twice with the seats swapped (C++ AI on the even player slots, then on the
odd ones; barbarians stay on ffh-ai), so map and civilization luck cancel out per seed pair.

  python3 crates/ffh-cppai/scripts/h2h.py --bin $B/ffh --seeds 1-8 --turns 300 [--civs 6]
      [--size WORLDSIZE_STANDARD] [--at 100,200,300] [--jobs 4] [--out runs.json | --load runs.json]
      [--handicap-cpp HANDICAP_X] [--handicap-ffh HANDICAP_Y] [--until-supremacy]

Games go through tools/agents/cached-run.sh (run cache + CPU slots; FFH_* variables are part of
the cache key). Reported per side, at each checkpoint: summed world share (score v2, basis points;
10000 = the whole world), cities, population, techs, military strength, civs alive; per game, the
side of the top-scoring civ at the end ("game won"), and an exact sign test over seed pairs.
"""
import argparse
import json
import math
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
CACHED_RUN = os.path.join(ROOT, 'tools', 'agents', 'cached-run.sh')


def seeds_of(spec):
    out = []
    for part in spec.split(','):
        if '-' in part:
            a, b = part.split('-')
            out.extend(range(int(a), int(b) + 1))
        else:
            out.append(int(part))
    return out


def run_game(args, seed, cpp_parity):
    env = dict(os.environ)
    env['FFH_CPPAI'] = cpp_parity
    hcap = []
    for slot in range(args.civs):
        cpp = (slot % 2 == 0) == (cpp_parity == 'even')
        h = args.handicap_cpp if cpp else args.handicap_ffh
        if h:
            hcap.append('%d:%s' % (slot, h))
    if hcap:
        env['FFH_PLAYER_HANDICAP'] = ','.join(hcap)
    else:
        env.pop('FFH_PLAYER_HANDICAP', None)
    cmd = [CACHED_RUN, args.bin, 'stats', '--seed', str(seed), '--turns', str(args.turns), '--civs', str(args.civs),
           '--size', args.size, '--at', args.at]
    for o in args.option:
        cmd += ['--option', o]
    if args.until_supremacy:
        cmd += ['--until-supremacy']
    if args.detail:
        cmd += ['--detail']
    p = subprocess.run(cmd, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if p.returncode != 0:
        tail = '\n'.join(p.stderr.strip().split('\n')[-15:])
        return {'seed': seed, 'cpp': cpp_parity, 'error': tail}
    line = [l for l in p.stdout.strip().split('\n') if l.startswith('{')][-1]
    return {'seed': seed, 'cpp': cpp_parity, 'stats': json.loads(line)}


def side_of(p, parity):
    return 'cpp' if (p % 2 == 0) == (parity == 'even') else 'ffh'


def side_of_row(r, parity):
    """The side that played this player: its controller when the stats name it (civilizations
    created mid-game are played by their creator's controller), else the slot's parity."""
    if 'ctrl' in r:
        return 'cpp' if r['ctrl'] == 'cppai' else 'ffh'
    return side_of(r['p'], parity)


FIELDS = ['score', 'cities', 'pop', 'techs', 'mil_str', 'legacy_score', 'land']


def summarize_game(g):
    st = g['stats']
    snaps = st['snapshots']
    out = {}
    for t, rows in snaps.items():
        agg = {s: {f: 0.0 for f in FIELDS} | {'alive': 0, 'n': 0} for s in ('cpp', 'ffh')}
        for r in rows:
            if r.get('barb'):
                continue
            s = side_of_row(r, g['cpp'])
            agg[s]['n'] += 1
            if r.get('alive'):
                agg[s]['alive'] += 1
            for f in FIELDS:
                agg[s][f] += float(r.get(f) or 0)
        out[int(t)] = agg
    final_t = max(out)
    rows = [r for r in snaps[str(final_t)] if not r.get('barb')]
    top = max(rows, key=lambda r: (r.get('score') or 0, r.get('legacy_score') or 0))
    winner = side_of_row(top, g['cpp'])
    oc = st['outcome']
    if oc.get('kind') in ('victory', 'last_standing', 'supremacy') and oc.get('team', -1) >= 0:
        members = [r for r in rows if r.get('team') == oc['team']]
        if members:
            winner = side_of_row(max(members, key=lambda r: r.get('score') or 0), g['cpp'])
    return {'per_turn': out, 'final_turn': final_t, 'winner': winner, 'outcome': oc, 'victory': oc.get('victory') or oc.get('kind'),
            'ms_per_turn': st.get('ms_per_turn'), 'elim': st.get('eliminated_at', {})}


def sign_test_p(wins, losses):
    n = wins + losses
    if n == 0:
        return 1.0
    k = min(wins, losses)
    p = sum(math.comb(n, i) for i in range(0, k + 1)) / 2 ** n
    return min(1.0, 2 * p)


def report(games, args):
    ok = [g for g in games if 'stats' in g]
    bad = [g for g in games if 'error' in g]
    print('games: %d ok, %d failed' % (len(ok), len(bad)))
    for g in bad:
        print('  FAILED seed %d (C++ AI on %s slots):\n    %s' % (g['seed'], g['cpp'], g['error'].replace('\n', '\n    ')))
    if not ok:
        return
    sums = [(g, summarize_game(g)) for g in ok]
    turns = sorted({t for _, s in sums for t in s['per_turn']})
    checkpoints = [int(t) for t in args.at.split(',')]
    print()
    print('per side, mean over games (share = summed world share, basis points; 10000 = everything)')
    print('%6s %6s | %8s %8s | %6s %6s | %6s %6s | %6s %6s | %8s %8s | %5s %5s' % (
        'turn', 'games', 'shareC', 'shareF', 'citC', 'citF', 'popC', 'popF', 'techC', 'techF', 'milC', 'milF', 'aliC', 'aliF'))
    for t in checkpoints + ['end']:
        vals = []
        for g, s in sums:
            tt = s['final_turn'] if t == 'end' else (t if t in s['per_turn'] else None)
            if tt is None:
                # the game ended earlier: carry its last value forward
                tt = s['final_turn'] if s['final_turn'] < t else None
            if tt is not None:
                vals.append(s['per_turn'][tt])
        if not vals:
            continue
        m = lambda side, f: sum(v[side][f] for v in vals) / len(vals)
        print('%6s %6d | %8.0f %8.0f | %6.1f %6.1f | %6.1f %6.1f | %6.1f %6.1f | %8.1f %8.1f | %5.2f %5.2f' % (
            t, len(vals), m('cpp', 'score'), m('ffh', 'score'), m('cpp', 'cities'), m('ffh', 'cities'),
            m('cpp', 'pop'), m('ffh', 'pop'), m('cpp', 'techs'), m('ffh', 'techs'), m('cpp', 'mil_str'), m('ffh', 'mil_str'),
            m('cpp', 'alive'), m('ffh', 'alive')))
    wins = {'cpp': 0, 'ffh': 0}
    for g, s in sums:
        wins[s['winner']] += 1
    print()
    print('games won (top civ at the end, or the victor): C++ AI %d, ffh-ai %d' % (wins['cpp'], wins['ffh']))
    # per seed pair: which side had the higher end share summed over both seatings
    pair = {}
    for g, s in sums:
        e = s['per_turn'][s['final_turn']]
        d = pair.setdefault(g['seed'], [0.0, 0])
        d[0] += e['cpp']['score'] - e['ffh']['score']
        d[1] += 1
    full = {k: v for k, v in pair.items() if v[1] == 2}
    w = sum(1 for v in full.values() if v[0] > 0)
    l = sum(1 for v in full.values() if v[0] < 0)
    print('seed pairs (both seatings played): %d; C++ AI ahead on end share in %d, ffh-ai in %d (sign test p=%.3f)' % (
        len(full), w, l, sign_test_p(w, l)))
    kinds = {}
    for g, s in sums:
        k = (s['victory'], s['winner'])
        kinds[k] = kinds.get(k, 0) + 1
    print('outcomes (how, winning side):', dict(sorted(kinds.items())))
    mst = [s['ms_per_turn'] for _, s in sums if s['ms_per_turn']]
    if mst:
        print('ms/turn: mean %.0f, max %.0f' % (sum(mst) / len(mst), max(mst)))
    print()
    print('per game (end): seed, C++ seats, final turn, share C/F, cities C/F, alive C/F, winner')
    for g, s in sorted(sums, key=lambda x: (x[0]['seed'], x[0]['cpp'])):
        e = s['per_turn'][s['final_turn']]
        print('  %3d %4s t%-4d  %5.0f/%-5.0f  %4.0f/%-4.0f  %d/%d  %s' % (
            g['seed'], g['cpp'], s['final_turn'], e['cpp']['score'], e['ffh']['score'], e['cpp']['cities'], e['ffh']['cities'],
            e['cpp']['alive'], e['ffh']['alive'], s['winner']))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--bin', default=os.path.join(ROOT, 'target', 'release', 'ffh'))
    ap.add_argument('--seeds', default='1-4')
    ap.add_argument('--turns', type=int, default=300)
    ap.add_argument('--civs', type=int, default=6)
    ap.add_argument('--size', default='WORLDSIZE_STANDARD')
    ap.add_argument('--at', default='100,200,300')
    ap.add_argument('--jobs', type=int, default=4)
    ap.add_argument('--option', action='append', default=[])
    ap.add_argument('--until-supremacy', action='store_true', help='stop a game at clear supremacy (ffh stats)')
    ap.add_argument('--detail', action='store_true', help='per-player unit/building/project type counts (ffh stats)')
    ap.add_argument('--handicap-cpp', default=None, help='HANDICAP_X for the C++ AI slots')
    ap.add_argument('--handicap-ffh', default=None, help='HANDICAP_X for the ffh-ai slots')
    ap.add_argument('--out', default=None)
    ap.add_argument('--load', default=None)
    args = ap.parse_args()
    if args.load:
        games = json.load(open(args.load))
    else:
        jobs = [(s, par) for s in seeds_of(args.seeds) for par in ('even', 'odd')]
        with ThreadPoolExecutor(args.jobs) as ex:
            games = list(ex.map(lambda j: run_game(args, *j), jobs))
        if args.out:
            json.dump(games, open(args.out, 'w'))
    report(games, args)


if __name__ == '__main__':
    main()
