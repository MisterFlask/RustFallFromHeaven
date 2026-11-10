#!/usr/bin/env python3
"""Autopsy of AI games (docs/AUTOPSY.md): what goes wrong, counted and attributed to a cause.

  autopsy.py --bin DIR --seeds 1-24 --turns 300 [--civs 6] [--size WORLDSIZE_STANDARD]
             [--env K=V ...] [--jobs N] --out DIR [--by civ] [--json FILE]
  autopsy.py --load DIR [--by civ] [--json FILE]

With --bin, plays each seed with `ffh stats` and FFH_TELEMETRY_FILE, writing DIR/seed_N.json
(the stats) and DIR/seed_N.jsonl (the events); a seed whose two files exist is not played again.
Each game holds one machine-wide CPU slot (tools/agents, docs/AGENTS.md); these runs bypass the
run cache, which cannot replay the events file. Then reports from the files (--load: only that).

The report: cities fallen (to barbarians and to civilizations, by third of the game) and the
cause class of each, computed here from the facts the rules recorded (`city_fell` events) with
the cut-offs below; eliminations and the chain of cities each lost; the army census (where the
land military stood, at war and at peace); units lost attacking and defending.
"""
import argparse
import json
import os
import subprocess
import sys
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor

try:
    sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', 'tools', 'agents'))
    import ffh_agents
except ImportError:
    ffh_agents = None
finally:
    del sys.path[0]

# ---- Cut-offs of the cause classes (ratios of quantities the game reports) ----
# `outclassed`: the old owner's power below this share of the captor's (civilization captors).
OUTCLASSED_POWER_RATIO = 0.60
# `army_away`: the old owner's military beyond the near radius (5 plots, set in the rules side's
# telemetry) had more than this multiple of the summed strength of the captor's units within 2
# plots of the city.
ARMY_AWAY_RATIO = 1.00
# Strength-share buckets of a battle (attacker's strength / both sides'), for losses.
SHARE_BUCKETS = [0.0, 0.2, 0.4, 0.6, 0.8, 1.01]
EXAMPLES = 5

CLASSES = ['empty', 'outclassed', 'army_away', 'thin', 'other']


def parse_seeds(s):
    out = []
    for part in s.split(','):
        if '-' in part:
            a, b = part.split('-')
            out.extend(range(int(a), int(b) + 1))
        elif part:
            out.append(int(part))
    return out


# ---------------------------------------------------------------------------- running games

def ffh_binary(path):
    return os.path.join(path, 'ffh') if os.path.isdir(path) else path


def play(args, seed):
    js = os.path.join(args.out, 'seed_%d.json' % seed)
    ev = os.path.join(args.out, 'seed_%d.jsonl' % seed)
    if os.path.exists(js) and os.path.exists(ev):
        return seed, 'exists'
    cmd = [ffh_binary(args.bin), 'stats', '--seed', str(seed), '--civs', str(args.civs), '--size', args.size,
           '--turns', str(args.turns)]
    env = dict(os.environ)
    for kv in args.env:
        k, _, v = kv.partition('=')
        env[k] = v
    env['FFH_TELEMETRY_FILE'] = ev + '.part'
    slots = ffh_agents.cpu_slots(1, ' '.join(cmd)) if ffh_agents else None
    if slots:
        slots.__enter__()
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, env=env)
    finally:
        if slots:
            slots.__exit__(None, None, None)
    if r.returncode != 0:
        return seed, 'failed: ' + (r.stderr or r.stdout)[-500:]
    line = [l for l in r.stdout.splitlines() if l.startswith('{')][-1]
    with open(js + '.part', 'w') as f:
        f.write(line + '\n')
    os.replace(ev + '.part', ev)
    os.replace(js + '.part', js)
    return seed, 'played'


def run_games(args):
    os.makedirs(args.out, exist_ok=True)
    seeds = parse_seeds(args.seeds)
    with ThreadPoolExecutor(max_workers=args.jobs) as ex:
        for seed, how in ex.map(lambda s: play(args, s), seeds):
            print('seed %d: %s' % (seed, how), file=sys.stderr)


# ---------------------------------------------------------------------------- loading

class Game:
    def __init__(self, name, stats, events):
        self.name = name
        self.stats = stats
        self.events = events
        self.seed = stats.get('seed', name)
        self.final_turn = stats.get('final_turn') or max((e['t'] for e in events), default=0)
        self.players = {}
        snaps = stats.get('snapshots', {})
        for t in sorted(snaps, key=int):
            for r in snaps[t]:
                self.players[int(r['p'])] = r

    def civ(self, p):
        r = self.players.get(p)
        return r.get('civ', 'P%d' % p) if r else 'P%d' % p

    def barb(self, p):
        r = self.players.get(p)
        return bool(r and r.get('barb'))


def load_dir(d):
    games = []
    for fn in sorted(os.listdir(d)):
        if not fn.endswith('.jsonl'):
            continue
        base = fn[:-len('.jsonl')]
        js = os.path.join(d, base + '.json')
        if not os.path.exists(js):
            print('%s: no stats file %s' % (fn, js), file=sys.stderr)
            continue
        with open(js) as f:
            text = f.read().strip()
        stats = json.loads([l for l in text.split('\n') if l.startswith('{')][-1])
        events = []
        with open(os.path.join(d, fn)) as f:
            for line in f:
                line = line.strip()
                if line.startswith('{'):
                    try:
                        events.append(json.loads(line))
                    except json.JSONDecodeError:
                        pass
        games.append(Game(base, stats, events))
    games.sort(key=lambda g: (str(type(g.seed)), g.seed))
    return games


# ---------------------------------------------------------------------------- classification

def strength(pair):
    return pair[1] if isinstance(pair, list) else 0


def classify(e):
    """(class, sub-class, facts) for one `city_fell` event."""
    defended_today = e.get('siege_turn') == e['t']
    def_n = e.get('def_n', 0) if defended_today else 0
    away = {k: strength(e.get(k)) for k in ('other_cities', 'home_field', 'abroad')}
    away_str = sum(away.values())
    near_str = e.get('captor_near_str', 0)
    own_str = sum(strength(e.get(k)) for k in ('in_city', 'near', 'other_cities', 'home_field', 'abroad'))
    captor_str = strength(e.get('captor_mil'))
    power, cpower = e.get('power', 0), e.get('captor_power', 0)
    facts = {
        'def_n': def_n, 'def_str': e.get('def_str', 0) if defended_today else 0,
        'def_full': e.get('def_full', 0) if defended_today else 0,
        'away_str': away_str, 'captor_near_str': near_str, 'own_mil_str': own_str, 'captor_mil_str': captor_str,
        'power_ratio': power / cpower if cpower else None,
        'mil_ratio': own_str / captor_str if captor_str else None,
        'away_ratio': away_str / near_str if near_str else None,
    }
    if 'captor_near_str' not in e:
        return 'other', None, facts
    if def_n == 0:
        return 'empty', None, facts
    if not e.get('barb') and cpower and power < OUTCLASSED_POWER_RATIO * cpower:
        return 'outclassed', None, facts
    if away_str > ARMY_AWAY_RATIO * near_str:
        sub = max(away, key=lambda k: away[k])
        return 'army_away', sub, facts
    return 'thin', None, facts


# The value whose median picks a class's typical examples.
TYPICAL_KEY = {'empty': 'own_mil_str', 'outclassed': 'power_ratio', 'army_away': 'away_ratio', 'thin': 'mil_ratio',
               'other': 'own_mil_str'}


def third(g, t):
    n = max(1, g.final_turn)
    return min(2, t * 3 // (n + 1))


THIRDS = ['early', 'middle', 'late']


# ---------------------------------------------------------------------------- analysis

def add_loss(v, side, b, e, n, lost, where):
    v[side + '_battles'] += 1
    v[side + '_committed'] += n
    v[side + '_lost'] += lost
    v[side + '_lost_' + where] += lost
    if b is not None and side == 'att':
        tot = b.get('att_str', 0) + b.get('def_str', 0)
        share = b['att_str'] / tot if tot else 1.0
        for i in range(len(SHARE_BUCKETS) - 1):
            if SHARE_BUCKETS[i] <= share < SHARE_BUCKETS[i + 1]:
                v['att_share_%d' % i] += 1
                v['att_share_%d_lost' % i] += lost
                v['att_share_%d_committed' % i] += n
                v['att_share_%d_taken' % i] += e.get('res') == 'taken'
                break


def analyse(games, by_civ):
    rep = {'cutoffs': {'OUTCLASSED_POWER_RATIO': OUTCLASSED_POWER_RATIO, 'ARMY_AWAY_RATIO': ARMY_AWAY_RATIO,
                       'near_radius_plots': 5, 'captor_radius_plots': 2},
           'games': [], 'falls': [], 'eliminations': [], 'census': {}, 'losses': {}}
    falls = []
    barb_cities_taken = 0
    for g in games:
        per_game = {'game': g.name, 'seed': g.seed, 'final_turn': g.final_turn, 'fell_to_barb': 0, 'fell_to_civ': 0,
                    'barb_cities_taken': 0}
        for e in g.events:
            if e.get('e') != 'city_fell':
                continue
            if g.barb(e['p']) or e.get('owner_host'):
                per_game['barb_cities_taken'] += 1
                barb_cities_taken += 1
                continue
            cls, sub, facts = classify(e)
            f = {'game': g.name, 'seed': g.seed, 't': e['t'], 'third': THIRDS[third(g, e['t'])],
                 'city': e.get('city'), 'x': e.get('x'), 'y': e.get('y'), 'pop': e.get('pop'),
                 'owner': e['p'], 'owner_civ': g.civ(e['p']), 'captor': e['to'],
                 'captor_civ': g.civ(e['to']) + (' (crusade host)' if e.get('host') else ''),
                 'barb': e.get('barb', False), 'razed': e.get('razed'), 'capital': e.get('capital'),
                 'cls': cls, 'sub': sub, **facts, 'raw': e}
            falls.append(f)
            per_game['fell_to_barb' if f['barb'] else 'fell_to_civ'] += 1
        rep['games'].append(per_game)
        # eliminations
        fell_by = defaultdict(list)
        for f in falls:
            if f['game'] == g.name:
                fell_by[f['owner']].append(f)
        for e in g.events:
            if e.get('e') != 'civ_died':
                continue
            chain = [{'t': f['t'], 'to': f['captor_civ'], 'barb': f['barb'], 'cls': f['cls']} for f in fell_by[e['p']]]
            rep['eliminations'].append({'game': g.name, 'seed': g.seed, 't': e['t'], 'civ': g.civ(e['p']),
                                        'killer': g.civ(e['killer']) if e.get('killer') is not None else None,
                                        'peak_cities': e.get('peak_cities'), 'peak_turn': e.get('peak_turn'),
                                        'chain': chain, 'third': THIRDS[third(g, e['t'])]})
    rep['falls'] = [{k: v for k, v in f.items() if k != 'raw'} for f in falls]
    rep['barb_cities_taken'] = barb_cities_taken

    # army census (stats aggregates) and losses (events)
    def keys_of(g, p):
        return ['all', g.civ(p)] if by_civ else ['all']

    census = defaultdict(lambda: defaultdict(int))
    losses = defaultdict(lambda: defaultdict(int))
    for g in games:
        for ps, tp in g.stats.get('telemetry', {}).get('players', {}).items():
            p = int(ps)
            if g.barb(p):
                continue
            for k in keys_of(g, p):
                for name, (n, s) in tp.get('counts', {}).get('army', {}).items():
                    census[k][name] += s
                    if name.endswith('_city'):
                        census[k][name[:-len('_city')] + '_turns'] += n
                # duel odds (ships, air): from the aggregates
                for b, (n, s) in tp.get('counts', {}).get('attack_odds', {}).items():
                    losses[k]['duel_attacks_' + b] += n
                    losses[k]['duel_wins_' + b] += s
        pending = {}
        for e in g.events:
            kind = e.get('e')
            if kind == 'battle':
                pending[(e['t'], e['p'], e['x'], e['y'])] = e
            elif kind == 'engagement':
                b = pending.pop((e['t'], e['p'], e['x'], e['y']), None)
                pa, pd = e['p'], e.get('vs')
                for side, p, lost, n in (('att', pa, e.get('att_lost', 0), e.get('att', 0)),
                                         ('def', pd, e.get('def_lost', 0), e.get('def', 0))):
                    if p is None or g.barb(p):
                        continue
                    where = 'unknown'
                    if b is not None:
                        owner = b.get('owner', -1)
                        where = 'own' if owner == p else ('none' if owner < 0 else 'foreign')
                    for k in keys_of(g, p):
                        add_loss(losses[k], side, b, e, n, lost, where)
    rep['census'] = {k: dict(v) for k, v in census.items()}
    rep['losses'] = {k: dict(v) for k, v in losses.items()}
    return rep


# ---------------------------------------------------------------------------- report

def pct(a, b):
    return '%5.1f%%' % (100.0 * a / b) if b else '    -'


def fmt_ratio(x):
    return '-' if x is None else '%.2f' % x


def typical(falls, cls, n):
    key = TYPICAL_KEY[cls]
    vals = sorted(f[key] for f in falls if f[key] is not None)
    if not vals:
        return falls[:n]
    med = vals[len(vals) // 2]
    return sorted(falls, key=lambda f: (abs((f[key] if f[key] is not None else med) - med), str(f['seed']), f['t']))[:n]


def print_falls(rep, group_by):
    falls = rep['falls']
    groups = defaultdict(list)
    for f in falls:
        groups[f['owner_civ'] if group_by else 'all'].append(f)
    for gname in sorted(groups):
        fs = groups[gname]
        if group_by:
            print('\n-- %s: %d cities fallen' % (gname, len(fs)))
        for captor in ('barbarians', 'civilizations'):
            sel = [f for f in fs if f['barb'] == (captor == 'barbarians')]
            if not sel:
                print('\n  to %s: none' % captor)
                continue
            print('\n  to %s: %d (razed %d)' % (captor, len(sel), sum(1 for f in sel if f['razed'])))
            print('    %-12s %5s %7s   %s' % ('class', 'n', 'share', 'by third (early/middle/late)'))
            for cls in CLASSES:
                c = [f for f in sel if f['cls'] == cls]
                if not c:
                    continue
                th = [sum(1 for f in c if f['third'] == t) for t in THIRDS]
                extra = ''
                if cls == 'army_away':
                    subs = defaultdict(int)
                    for f in c:
                        subs[f['sub']] += 1
                    extra = '  (' + ', '.join('%s %d' % (k, v) for k, v in sorted(subs.items())) + ')'
                if cls == 'thin':
                    rs = sorted(f['mil_ratio'] for f in c if f['mil_ratio'] is not None)
                    if rs:
                        extra = '  (own military / captor\'s: median %.2f)' % rs[len(rs) // 2]
                print('    %-12s %5d %7s   %d/%d/%d%s' % (cls, len(c), pct(len(c), len(sel)), th[0], th[1], th[2], extra))
            if group_by:
                continue
            for cls in CLASSES:
                c = [f for f in sel if f['cls'] == cls]
                if not c:
                    continue
                print('    typical %s:' % cls)
                for f in typical(c, cls, EXAMPLES):
                    print('      seed %s t%d city %s (%s,%s) pop %s P%d %s <- P%d %s%s: def %d (str %d of %d at full health), captor near %d, '
                          'own mil %d (away %d), captor mil %d, power ratio %s, %s' % (
                              f['seed'], f['t'], f['city'], f['x'], f['y'], f['pop'], f['owner'], f['owner_civ'],
                              f['captor'], f['captor_civ'],
                              ' capital' if f['capital'] else '', f['def_n'], f['def_str'] // 100, f['def_full'] // 100,
                              f['captor_near_str'] // 100, f['own_mil_str'] // 100, f['away_str'] // 100,
                              f['captor_mil_str'] // 100, fmt_ratio(f['power_ratio']),
                              'razed' if f['razed'] else 'kept'))


def print_report(rep, by_civ):
    c = rep['cutoffs']
    print('== Autopsy: %d games' % len(rep['games']))
    print('cut-offs: outclassed = power < %.2f x captor\'s (civ captors); army_away = own military beyond %d plots '
          '> %.2f x captor\'s strength within %d plots; strengths are the rules\' (x hit points), shown /100'
          % (c['OUTCLASSED_POWER_RATIO'], c['near_radius_plots'], c['ARMY_AWAY_RATIO'], c['captor_radius_plots']))
    print('classes, first that applies: empty (no defender when the day\'s assault began) > outclassed > army_away > thin; '
          'other = facts missing')

    print('\n== Cities fallen (civilizations\' cities; barbarian and crusade-host cities taken: %d, not counted below)'
          % rep['barb_cities_taken'])
    print('  %-10s %6s %8s %8s %10s' % ('game', 'turns', 'to barb', 'to civ', 'barb taken'))
    for g in rep['games']:
        print('  %-10s %6d %8d %8d %10d' % (g['game'], g['final_turn'], g['fell_to_barb'], g['fell_to_civ'], g['barb_cities_taken']))
    falls = rep['falls']
    tb = sum(1 for f in falls if f['barb'])
    print('  total: %d (to barbarians %d, to civilizations %d)' % (len(falls), tb, len(falls) - tb))
    th = defaultdict(lambda: [0, 0])
    for f in falls:
        th[f['third']][0 if f['barb'] else 1] += 1
    print('  by third of the game: ' + ', '.join('%s %d barb / %d civ' % (t, th[t][0], th[t][1]) for t in THIRDS))
    print_falls(rep, False)
    if by_civ:
        print('\n== Cities fallen by civilization (the old owner)')
        print_falls(rep, True)

    print('\n== Eliminations: %d' % len(rep['eliminations']))
    if rep['eliminations']:
        ths = defaultdict(int)
        for e in rep['eliminations']:
            ths[e['third']] += 1
        print('  by third: ' + ', '.join('%s %d' % (t, ths[t]) for t in THIRDS))
        for e in rep['eliminations']:
            chain = ', '.join('t%d->%s(%s)' % (x['t'], 'barb' if x['barb'] else x['to'], x['cls']) for x in e['chain'])
            print('  seed %s t%d %s (peak %s cities at t%s), last taken by %s: %s' % (
                e['seed'], e['t'], e['civ'], e['peak_cities'], e['peak_turn'], e['killer'] or '-', chain or 'no city lost by conquest'))

    print('\n== Army census (land military unit-turns at the end of the owner\'s turn)')
    print('  %-14s %9s %7s %7s %7s %7s %9s %10s' % ('group / state', 'unit-turns', 'city', 'home', 'enemy', 'other',
                                                     'still', 'idle@war'))
    for k in sorted(rep['census']):
        v = rep['census'][k]
        for state in ('war', 'peace'):
            tot = sum(v.get('%s_%s' % (state, pl), 0) for pl in ('city', 'home', 'enemy', 'other'))
            still = sum(v.get('%s_%s_still' % (state, pl), 0) for pl in ('city', 'home', 'enemy', 'other'))
            idle = v.get('%s_home' % state, 0) + v.get('%s_other' % state, 0)
            print('  %-14s %9d %7s %7s %7s %7s %9s %10s' % (
                '%s %s' % (k[:8], state), tot, pct(v.get(state + '_city', 0), tot), pct(v.get(state + '_home', 0), tot),
                pct(v.get(state + '_enemy', 0), tot), pct(v.get(state + '_other', 0), tot), pct(still, tot),
                pct(idle, tot) if state == 'war' else ''))
        wt, pt = v.get('war_turns', 0), v.get('peace_turns', 0)
        print('  %-14s player-turns at war with a civ: %d of %d' % ('', wt, wt + pt))
    print('  (idle@war: share of army-turns at war outside any city and outside enemy territory; '
          'still: spent no movement that turn)')

    print('\n== Losses in engagements (units lost / committed; "own" = on the side\'s own territory)')
    for k in sorted(rep['losses']):
        v = rep['losses'][k]
        print('  %s' % k)
        for side, label in (('att', 'attacking'), ('def', 'defending')):
            print('    %-9s battles %6d  committed %6d  lost %6d (%s)  own %d / foreign %d / unowned %d' % (
                label, v.get(side + '_battles', 0), v.get(side + '_committed', 0), v.get(side + '_lost', 0),
                pct(v.get(side + '_lost', 0), v.get(side + '_committed', 0)), v.get(side + '_lost_own', 0),
                v.get(side + '_lost_foreign', 0), v.get(side + '_lost_none', 0)))
        print('    attacks by strength share (attacker / both, at launch): ' + ', '.join(
            '%.1f-%.1f: %d (lost %s, taken %s)' % (
                SHARE_BUCKETS[i], min(1.0, SHARE_BUCKETS[i + 1]), v.get('att_share_%d' % i, 0),
                pct(v.get('att_share_%d_lost' % i, 0), v.get('att_share_%d_committed' % i, 0)).strip(),
                pct(v.get('att_share_%d_taken' % i, 0), v.get('att_share_%d' % i, 0)).strip())
            for i in range(len(SHARE_BUCKETS) - 1)))
        duels = [(b[len('duel_attacks_'):], n, v.get('duel_wins_' + b[len('duel_attacks_'):], 0))
                 for b, n in sorted(v.items()) if b.startswith('duel_attacks_') and n]
        if duels:
            print('    duels (ships, air) by odds decile: ' + ', '.join('%s%%: %d won %d' % d for d in duels))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--bin', help='build output directory holding ffh (or the ffh binary)')
    ap.add_argument('--seeds', default='1-6')
    ap.add_argument('--turns', type=int, default=300)
    ap.add_argument('--civs', type=int, default=6)
    ap.add_argument('--size', default='WORLDSIZE_STANDARD')
    ap.add_argument('--env', action='append', default=[], help='K=V for the games (repeatable)')
    ap.add_argument('--jobs', type=int, default=os.cpu_count() or 4)
    ap.add_argument('--out', help='directory for the games played (with --bin)')
    ap.add_argument('--load', help='report from the games in this directory')
    ap.add_argument('--by', choices=['civ'], help='also per civilization')
    ap.add_argument('--json', help='write everything to this file')
    args = ap.parse_args()
    if args.bin:
        if not args.out:
            ap.error('--bin needs --out')
        run_games(args)
        d = args.out
    elif args.load:
        d = args.load
    else:
        ap.error('give --bin and --out, or --load')
    games = load_dir(d)
    if not games:
        sys.exit('no games in %s' % d)
    by_civ = args.by == 'civ'
    rep = analyse(games, by_civ)
    print_report(rep, by_civ)
    if args.json:
        with open(args.json, 'w') as f:
            json.dump(rep, f, indent=1, default=str)


if __name__ == '__main__':
    main()
