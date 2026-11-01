#!/usr/bin/env python3
"""A/B for behaviour-preserving performance work (see docs/PERF.md, docs/AGENTS.md).

  perf-ab.sh [--base X] [--new X] same [--set quick|dev|val|all] [--soak]
  perf-ab.sh [--base X] [--new X] time [--rounds N] [--config NAME]

X is a build output dir (has bin/ffh), a worktree dir (built with build-tree) or a revision
(built with build-rev). Default base: the merge-base of HEAD and master; default new: this
worktree.

`same` plays the configs of a set with both builds (`ffh autoplay`, through the run cache and
the CPU slots, so the base side is usually replayed) and compares every turn's (turn, units,
cities, sync checksum) and the final summary: IDENTICAL or the first difference. `--soak` also
plays 3 human-slot games (`ffh-play soak run`: the human-only rules branches) and compares their
final checksum and the hash of the final save bytes. Choose with one set (dev), validate with
another (val) that was not used to choose.

`time` runs `ffh bench` on both builds, interleaved for --rounds rounds (order alternating),
holding every CPU slot and the build lock, and reports CPU seconds (the child's user + system
time) and ms/turn: the median per build and the median of the per-round new/base ratios. On this
shared host whole runs drift by 5-20% between sessions; only ratios within one session mean
something.
"""
import argparse
import os
import re
import statistics
import subprocess
import sys
import threading

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ffh_agents as fa  # noqa: E402

CONFIGS = {
    'D1': '--seed 1 --civs 8 --size WORLDSIZE_STANDARD --turns 400',
    'D2': '--seed 2 --civs 12 --size WORLDSIZE_LARGE --turns 300',
    'D3': '--seed 5 --civs 6 --size WORLDSIZE_SMALL --turns 300 --option GAMEOPTION_RAGING_BARBARIANS',
    'D4': '--seed 9 --civs 10 --size WORLDSIZE_TINY --turns 300',
    'V1': '--seed 11 --civs 18 --size WORLDSIZE_HUGE --turns 250',
    'V2': '--seed 12 --civs 7 --size WORLDSIZE_STANDARD --turns 450',
    'V3': '--seed 13 --civs 3 --size WORLDSIZE_DUEL --turns 400 --option GAMEOPTION_RAGING_BARBARIANS',
    'V4': '--seed 14 --civs 9 --size WORLDSIZE_LARGE --turns 350 --option GAMEOPTION_AGGRESSIVE_AI',
    'H1': '--seed 3 --civs 18 --size WORLDSIZE_HUGE --turns 300',
    'E1': '--seed 1 --civs 6 --size WORLDSIZE_STANDARD --turns 150',
}
SETS = {'quick': ['D1', 'D3'], 'dev': ['D1', 'D2', 'D3', 'D4'], 'val': ['V1', 'V2', 'V3', 'V4']}
SETS['all'] = SETS['dev'] + SETS['val']
# Human-slot games whose human civilization lives to the end (seed, size, civ).
SOAKS = [('22', 'standard', 'bannor'), ('25', 'small', 'sheaim'), ('24', 'large', 'malakim')]
TURN_LINE = re.compile(r'^turn +(\d+) +[0-9.]+ ms ')


def resolve(x, bins):
    """A build output dir for X (an output dir, a worktree or a revision)."""
    if os.path.isfile(os.path.join(x, 'bin', 'ffh')):
        return os.path.abspath(x)
    if os.path.isdir(x):
        return fa.build_tree(x, bins=bins)[0]
    return fa.build_rev(x, bins=bins)[0]


def run_all(jobs):
    """Runs (key, argv) jobs in parallel through the run cache and CPU slots: {key: stdout}."""
    res, errs = {}, []

    def one(key, argv):
        rc, out, err, _ = fa.run_cached(argv)
        if rc != 0:
            errs.append('%s: exit %d: %s' % (' '.join(argv), rc, err.decode(errors='replace')[-500:]))
        res[key] = out.decode(errors='replace')
    ts = [threading.Thread(target=one, args=j) for j in jobs]
    for t in ts:
        t.start()
    for t in ts:
        t.join()
    if errs:
        raise fa.Fail('\n'.join(errs))
    return res


def fingerprint(out):
    """`ffh autoplay` output without its timings."""
    lines = []
    for line in out.splitlines():
        if 'ms/turn' in line:
            continue
        lines.append(TURN_LINE.sub(lambda m: 'turn %s ' % m.group(1), line))
    return lines


def first_difference(a, b):
    for i, (x, y) in enumerate(zip(a, b)):
        if x != y:
            return 'line %d: base %r / new %r' % (i + 1, x, y)
    return 'base %d lines / new %d lines' % (len(a), len(b))


def cmd_same(base, new, names, soak):
    jobs = []
    for side, d in (('base', base), ('new', new)):
        for c in names:
            jobs.append(((side, c), [os.path.join(d, 'ffh'), 'autoplay'] + CONFIGS[c].split()))
        if soak:
            for seed, size, civ in SOAKS:
                argv = [os.path.join(d, 'ffh-play'), 'soak', 'run', '--seed', seed, '--size', size, '--civ', civ, '--turns', '300']
                jobs.append(((side, 'soak' + seed), argv))
    res = run_all(jobs)
    ok = True
    for c in names:
        a, b = fingerprint(res[('base', c)]), fingerprint(res[('new', c)])
        turns = sum(1 for x in a if x.startswith('turn '))
        if a == b:
            print('%s IDENTICAL (%d turns; %s)' % (c, turns, CONFIGS[c]))
        else:
            ok = False
            print('%s DIFFERENT (%s): %s' % (c, CONFIGS[c], first_difference(a, b)))
    if soak:
        for seed, size, civ in SOAKS:
            ends = [next((re.sub(r'\([0-9.]+ s, ', '(', x) for x in res[(s, 'soak' + seed)].splitlines() if x.startswith('END')), '?')
                    for s in ('base', 'new')]
            same = ends[0] == ends[1] and ends[0] != '?'
            ok = ok and same
            print('soak %s %s %s %s: %s' % (seed, size, civ, 'IDENTICAL' if same else 'DIFFERENT', ends[1] if same else ' / '.join(ends)))
    return 0 if ok else 1


def bench_once(d, args):
    """(CPU seconds, ms/turn) of one `ffh bench` run of build d."""
    p = subprocess.Popen([os.path.join(d, 'ffh'), 'bench'] + args, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    out = p.stdout.read().decode(errors='replace')
    _, status, ru = os.wait4(p.pid, 0)
    if status != 0:
        raise fa.Fail('ffh bench %s failed (%s)' % (' '.join(args), d))
    m = re.search(r'\(([0-9.]+) ms/turn', out)
    return ru.ru_utime + ru.ru_stime, float(m.group(1)) if m else float('nan')


def cmd_time(base, new, rounds, config):
    args = CONFIGS.get(config, config).split()
    print('timing `ffh bench %s`, %d interleaved rounds (waiting for every CPU slot and the build lock)' % (' '.join(args), rounds), flush=True)
    runs = {'base': [], 'new': []}
    with fa.cpu_slots(fa.slot_count(), 'perf-ab time', verbose=True), \
            fa.flocked(os.path.join(fa.ROOT, 'locks', 'build.lock'), wait_msg='perf-ab: waiting for the build lock'):
        for r in range(rounds):
            order = ('base', 'new') if r % 2 == 0 else ('new', 'base')
            for side in order:
                cpu, ms = bench_once(base if side == 'base' else new, args)
                runs[side].append((cpu, ms))
                print('  round %d %-4s %8.2f CPU s %8.2f ms/turn' % (r + 1, side, cpu, ms), flush=True)
    for side in ('base', 'new'):
        print('%-4s median %8.2f CPU s %8.2f ms/turn' % (side, statistics.median(x[0] for x in runs[side]),
                                                         statistics.median(x[1] for x in runs[side])))
    ratios = [n[0] / b[0] for b, n in zip(runs['base'], runs['new'])]
    med = statistics.median(ratios)
    print('new/base CPU, median of rounds: %.3f (%+.1f%%; rounds %s)' % (med, 100 * (med - 1), ' '.join('%.3f' % x for x in ratios)))
    return 0


def default_base():
    return subprocess.run(['git', 'merge-base', 'HEAD', 'master'], capture_output=True, text=True, check=True).stdout.strip()


def main():
    ap = argparse.ArgumentParser(prog='perf-ab.sh', description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--base')
    ap.add_argument('--new', default='.')
    sub = ap.add_subparsers(dest='cmd', required=True)
    s = sub.add_parser('same')
    s.add_argument('--set', default='dev', help='quick, dev, val, all, or comma-separated config names')
    s.add_argument('--soak', action='store_true')
    t = sub.add_parser('time')
    t.add_argument('--rounds', type=int, default=3)
    t.add_argument('--config', default='D1', help='a config name or `ffh bench` arguments in quotes')
    a = ap.parse_args()
    bins = ('ffh', 'ffh-play') if a.cmd == 'same' and a.soak else ('ffh',)
    try:
        base = resolve(a.base or default_base(), bins)
        new = resolve(a.new, bins)
        fa.log('base %s, new %s' % (base, new))
        if a.cmd == 'same':
            names = SETS.get(a.set) or a.set.split(',')
            return cmd_same(base, new, names, a.soak)
        return cmd_time(base, new, a.rounds, a.config)
    except fa.Fail as e:
        print('perf-ab: %s' % e, file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
