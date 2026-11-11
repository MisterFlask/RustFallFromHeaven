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

# ---- Naval (docs/AUTOPSY.md, "Naval") ----
# A military landing (units put ashore from ships on a landmass where their owner had no city,
# grouped by player, turn and plot) is followed by an assault when its player launches an
# engagement against a city of a civilization it was at war with, within LANDING_CITY_DIST plots
# of the landing plot, within LANDING_ASSAULT_TURNS turns; by a capture when it takes such a city
# within LANDING_CAPTURE_TURNS turns.
LANDING_CITY_DIST = 1
LANDING_ASSAULT_TURNS = 3
LANDING_CAPTURE_TURNS = 10

CLASSES = ['empty', 'outclassed', 'army_away', 'thin', 'other']
SECTIONS = ['falls', 'eliminations', 'census', 'losses', 'naval']
SEA_CLS = ['transport', 'warship', 'other']
SEA_PLACES = ['port', 'own', 'sea']
NAVAL_GROUPS = ['all', 'isolated', 'shared', 'late']


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
           '--turns', str(args.turns)] + [a for kv in args.arg for a in kv.split(' ', 1)]
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


def print_report(rep, by_civ, sections=None):
    sections = sections or SECTIONS
    c = rep['cutoffs']
    print('== Autopsy: %d games' % len(rep['games']))
    print('cut-offs: outclassed = power < %.2f x captor\'s (civ captors); army_away = own military beyond %d plots '
          '> %.2f x captor\'s strength within %d plots; strengths are the rules\' (x hit points), shown /100'
          % (c['OUTCLASSED_POWER_RATIO'], c['near_radius_plots'], c['ARMY_AWAY_RATIO'], c['captor_radius_plots']))
    print('classes, first that applies: empty (no defender when the day\'s assault began) > outclassed > army_away > thin; '
          'other = facts missing')

    if 'falls' in sections:
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

    if 'eliminations' in sections:
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

    if 'census' in sections:
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

    if 'losses' in sections:
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


# ---------------------------------------------------------------------------- naval

def naval_groups(g):
    """Player -> 'isolated' (no other civilization started on its landmass), 'shared', or 'late'
    (a civilization that entered the game without a start, e.g. from an event)."""
    out = {}
    geo = next((e for e in g.events if e.get('e') == 'geo'), None)
    if geo:
        for p, _area, _tiles, others in geo.get('civs', []):
            out[p] = 'isolated' if others == 0 else 'shared'
    for p, r in g.players.items():
        if p not in out and not r.get('barb'):
            out[p] = 'late'
    return out


def median(xs):
    xs = sorted(xs)
    return xs[len(xs) // 2] if xs else None


def analyse_naval(games):
    acc = {k: defaultdict(int) for k in NAVAL_GROUPS}
    roles = defaultdict(lambda: [0, 0, 0])  # role -> [unit-turns, in port, moved] (civilizations)
    barb_roles = defaultdict(lambda: [0, 0, 0])
    firsts = {k: defaultdict(list) for k in NAVAL_GROUPS}
    waits = {k: [] for k in NAVAL_GROUPS}  # military trips: turns from loading to leaving port
    fights = defaultdict(int)
    imp_lost = defaultdict(int)
    per_game = []
    wars = defaultdict(int)
    for g in games:
        grp = naval_groups(g)
        civs = set(grp)
        G = {'game': g.name, 'seed': g.seed, 'final_turn': g.final_turn}
        geo = next((e for e in g.events if e.get('e') == 'geo'), None)
        if geo:
            G['water_pct'] = round(100.0 * geo['water'] / max(1, geo['plots']), 1)
            G['start_landmasses'] = geo['start_areas']
            G['land_areas'] = geo['land_areas']
            G['isolated_civs'] = sum(1 for v in grp.values() if v == 'isolated')
        for k in ('ships', 'sea_fights', 'civ_sea_fights', 'landing_ops', 'landing_assaulted', 'landing_captured',
                  'falls', 'falls_by_sea', 'overseas_founded', 'overseas_kept', 'wars_no_land', 'wars_no_land_fought'):
            G[k] = 0

        def add(p, key, v=1):
            if p in grp:
                acc['all'][key] += v
                acc[grp[p]][key] += v

        # final standing and counts
        for p, gname in grp.items():
            r = g.players.get(p, {})
            add(p, 'civs')
            add(p, 'final_alive', 1 if r.get('alive') else 0)
            add(p, 'final_cities', r.get('cities', 0))
            add(p, 'final_techs', r.get('techs', 0))
            add(p, 'final_score', r.get('legacy_score', 0) or 0)
            add(p, 'final_power', r.get('power', 0) or 0)
        tel = g.stats.get('telemetry', {}).get('players', {})
        for ps, tp in tel.items():
            p = int(ps)
            c = tp.get('counts', {})
            if p not in grp:
                if g.barb(p):
                    for role, (n, inport) in c.get('sea_role', {}).items():
                        barb_roles[role][0] += n
                        barb_roles[role][1] += inport
                    for role, (n, _) in c.get('sea_role_moved', {}).items():
                        barb_roles[role][2] += n
                    for key, (n, s) in c.get('sea', {}).items():
                        acc_b = fights  # barbarian census goes with the fights' table
                        acc_b['barb_sea_' + key] += s
                continue
            for cls, (n, cost) in c.get('ship_built', {}).items():
                add(p, 'built_%s_n' % cls, n)
                add(p, 'built_%s_cost' % cls, cost)
                G['ships'] += n
            for reach, (n, _) in c.get('ship_built_reach', {}).items():
                add(p, 'built_reach_%s' % reach, n)
            for kind, (n, cost) in c.get('hammers', {}).items():
                add(p, 'hammers_' + kind, cost)
            for key, (n, s) in c.get('sea', {}).items():
                add(p, 'sea_' + key, s)
            for key, (n, s) in c.get('sea_cargo', {}).items():
                add(p, 'cargo_' + key, s)
            for role, (n, inport) in c.get('sea_role', {}).items():
                roles[role][0] += n
                roles[role][1] += inport
            for role, (n, _) in c.get('sea_role_moved', {}).items():
                roles[role][2] += n
            for key, (n, s) in c.get('sea_raid', {}).items():
                add(p, 'raid_turns', n if key == 'blockading' else 0)
                add(p, 'raid_' + key, s)
            for key, (n, _) in c.get('sea_load', {}).items():
                add(p, 'load_' + key, n)
            for key, (n, _) in c.get('sea_unload', {}).items():
                add(p, 'unload_' + key, n)
            for key, (n, _) in c.get('sea_trip', {}).items():
                add(p, 'trips_started', n)
            for key, (n, s) in c.get('sea_imp_lost', {}).items():
                imp_lost[key] += n
                imp_lost[key + '_on_resource'] += s
                add(p, 'sea_imp_lost', n)
            # player-turns at war (the land census counts them)
            for key, (n, _) in c.get('army', {}).items():
                if key == 'war_city':
                    add(p, 'war_turns', n)

        ev = g.events
        battles = [e for e in ev if e.get('e') == 'battle']
        falls = [e for e in ev if e.get('e') == 'city_fell']
        for e in ev:
            k = e.get('e')
            p = e.get('p')
            if k == 'naval_first' and p in grp:
                firsts['all'][e['kind']].append(e['t'])
                firsts[grp[p]][e['kind']].append(e['t'])
            elif k == 'sea_fight':
                G['sea_fights'] += 1
                ba, bd = e.get('barb_att'), e.get('barb_def')
                civ_civ = p in civs and e.get('vs') in civs
                pair = 'civ_vs_civ' if civ_civ else ('animal' if e.get('animal_def') else ('civ_vs_barb' if (ba or bd) else 'other'))
                G['civ_sea_fights'] += civ_civ
                fights['n'] += 1
                fights['pair_' + pair] += 1
                fights['att_%s_def_%s' % (e['att'], e['def'])] += 1
                fights['res_' + e['res']] += 1
                fights['undefended'] += bool(e.get('undefended'))
                fights['waters_' + e.get('waters', '?')] += 1
                fights['odds_sum'] += e.get('odds', 0)
                fights['drowned'] += e.get('drowned_att', 0) + e.get('drowned_def', 0)
                if (e['def'] == 'transport' and e['res'] == 'win' and e.get('drowned_def', 0) > 0) or \
                        (e['att'] == 'transport' and e['res'] == 'loss' and e.get('drowned_att', 0) > 0):
                    fights['transports_sunk_with_cargo'] += 1
                for side, q, cls in (('att', p, e['att']), ('def', e.get('vs'), e['def'])):
                    if q in grp and cls != 'land':
                        add(q, 'fight_%s' % side)
                        lost = (e['res'] == 'loss') if side == 'att' else (e['res'] == 'win')
                        add(q, 'ships_lost', 1 if lost else 0)
            elif k == 'trip' and p in grp:
                add(p, 'trip_%s_%s' % (e['kind'], e['outcome']))
                add(p, 'trips_ended')
                if e['kind'] == 'military':
                    if e.get('left_port') is None and e['outcome'] in ('port', 'other'):
                        add(p, 'trip_military_never_left')
                    elif e.get('left_port') is not None:
                        waits[grp[p]].append(e['left_port'] - e['start'])
                        waits['all'].append(e['left_port'] - e['start'])
            elif k == 'sea_pillage':
                victim = e.get('vs')
                fights['pillage_by_' + ('barb' if e.get('barb') else 'civ')] += 1
                fights['pillage_on_resource'] += bool(e.get('bonus'))
                if victim in grp:
                    add(victim, 'sea_pillaged')
            elif k == 'found' and p in grp and e.get('overseas'):
                G['overseas_founded'] += 1
                add(p, 'overseas_founded')
                lost = any(a.get('e') == 'acquire' and a.get('from') == p and a['x'] == e['x'] and a['y'] == e['y']
                           and a['t'] >= e['t'] for a in ev)
                if not lost:
                    G['overseas_kept'] += 1
                    add(p, 'overseas_kept')

        # landings: units put ashore on a landmass where the owner had no city, by (player, turn, plot)
        ops = {}
        for e in ev:
            if e.get('e') == 'unload' and e.get('where') == 'landing' and e['p'] in grp:
                key = (e['p'], e['t'], e['x'], e['y'])
                op = ops.setdefault(key, {'kinds': set(), 'n': 0, 'cities': e.get('cities', [])})
                op['kinds'].add(e['kind'])
                op['n'] += 1
        for (p, t, x, y), op in ops.items():
            kind = 'military' if 'military' in op['kinds'] else ('settler' if 'settler' in op['kinds'] else 'other')
            add(p, 'landing_%s' % kind)
            add(p, 'landing_%s_units' % kind, op['n'])
            if kind != 'military':
                continue
            G['landing_ops'] += 1
            targets = {(c[0], c[1]) for c in op['cities'] if c[3] <= LANDING_CITY_DIST and c[4] and c[2] in civs | {q for q in g.players if g.barb(q)}}
            if not targets:
                add(p, 'landing_no_target')
                continue
            assaulted = any(b['p'] == p and t <= b['t'] <= t + LANDING_ASSAULT_TURNS and b.get('city') and (b['x'], b['y']) in targets
                            for b in battles)
            captured = any(f.get('to') == p and t <= f['t'] <= t + LANDING_CAPTURE_TURNS and (f['x'], f['y']) in targets for f in falls)
            add(p, 'landing_assaulted', assaulted)
            add(p, 'landing_captured', captured)
            G['landing_assaulted'] += assaulted
            G['landing_captured'] += captured

        # cities fallen (civilizations' cities, any captor)
        for f in falls:
            p = f['p']
            if p not in grp:
                continue
            G['falls'] += 1
            by_sea = f.get('captor_near_by_sea', 0) > 0
            G['falls_by_sea'] += by_sea
            add(p, 'fell')
            add(p, 'fell_coastal', bool(f.get('coastal')))
            add(p, 'fell_by_sea', by_sea)
            add(p, 'fell_captor_no_city_on_land', f.get('captor_land_city') is False)
            to = f.get('to')
            if to in grp:
                add(to, 'took')
                add(to, 'took_by_sea', by_sea)
                add(to, 'took_overseas', f.get('captor_land_city') is False)

        # wars between civilizations
        peaces = [e for e in ev if e.get('e') == 'peace']
        fights_ev = [e for e in ev if e.get('e') in ('battle', 'sea_fight', 'combat')]
        for w in (e for e in ev if e.get('e') == 'war'):
            a, b = set(w.get('players', [])), set(w.get('vs_players', []))
            if not a or not b or not (a <= civs and b <= civs):
                continue
            pair = {w['team'], w['vs']}
            end = next((pe['t'] for pe in peaces if {pe['team'], pe['vs']} == pair and pe['t'] >= w['t']), g.final_turn)
            fought = any(w['t'] <= x['t'] <= end and ((x['p'] in a and x.get('vs') in b) or (x['p'] in b and x.get('vs') in a))
                         for x in fights_ev)
            landed = any(w['t'] <= o[1] <= end and (o[0] in a or o[0] in b) and any(
                c[2] in (b if o[0] in a else a) for c in op['cities']) for o, op in ops.items())
            land = 'shared_land' if w.get('share_land') else 'no_shared_land'
            wars[land] += 1
            wars[land + '_fought'] += fought
            wars[land + '_landed'] += landed
            if land == 'no_shared_land':
                G['wars_no_land'] += 1
                G['wars_no_land_fought'] += fought

        # coastal cities at thirds of the game
        samples = defaultdict(list)
        for e in ev:
            if e.get('e') == 'coast' and e['p'] in grp:
                samples[e['p']].append(e)
        for p, ss in samples.items():
            for i in range(3):
                want = g.final_turn * (i + 1) // 3
                s = min(ss, key=lambda e: abs(e['t'] - want))
                add(p, 'third%d_cities' % i, s['cities'])
                add(p, 'third%d_coastal' % i, s['coastal'])
                add(p, 'third%d_off_start' % i, s['off_start'])
        per_game.append(G)
    return {'cutoffs': {'LANDING_CITY_DIST': LANDING_CITY_DIST, 'LANDING_ASSAULT_TURNS': LANDING_ASSAULT_TURNS,
                        'LANDING_CAPTURE_TURNS': LANDING_CAPTURE_TURNS},
            'groups': {k: dict(v) for k, v in acc.items()}, 'roles': dict(roles), 'barb_roles': dict(barb_roles),
            'firsts': {k: dict(v) for k, v in firsts.items()}, 'military_trip_waits': waits, 'fights': dict(fights),
            'sea_imp_lost': dict(imp_lost), 'wars': dict(wars), 'games': per_game}


def print_naval(nav):
    c = nav['cutoffs']
    print('\n== Naval (civilizations; groups: isolated = no other civ started on its landmass, shared, late = joined without a start)')
    print('  landing = units put ashore from ships on a landmass where their owner had no city (by player, turn, plot); '
          'assault = an engagement on an enemy city within %d plot(s) within %d turns; capture = that city taken within %d turns'
          % (c['LANDING_CITY_DIST'], c['LANDING_ASSAULT_TURNS'], c['LANDING_CAPTURE_TURNS']))
    print('  %-8s %5s %6s %5s %6s %6s %6s %7s %7s %7s %6s %7s %8s' % (
        'game', 'turns', 'water', 'lands', 'isol', 'ships', 'fights', 'civ-civ', 'landing', 'assault', 'taken',
        'falls', 'by sea'))
    for G in nav['games']:
        print('  %-8s %5d %5s%% %5s %6s %6d %6d %7d %7d %7d %6d %7d %8d   overseas cities founded %d kept %d; '
              'wars without shared land %d (fought %d)' % (
                  G['game'][-8:], G['final_turn'], G.get('water_pct', '-'), G.get('start_landmasses', '-'),
                  G.get('isolated_civs', '-'), G['ships'], G['sea_fights'], G['civ_sea_fights'], G['landing_ops'],
                  G['landing_assaulted'], G['landing_captured'], G['falls'], G['falls_by_sea'],
                  G['overseas_founded'], G['overseas_kept'], G['wars_no_land'], G['wars_no_land_fought']))
    groups = [k for k in NAVAL_GROUPS if nav['groups'].get(k, {}).get('civs')]
    print('\n  %-40s' % '' + ''.join('%19s' % k for k in groups))

    def row(label, f):
        print('  %-40s' % label + ''.join('%19s' % f(nav['groups'][k], k) for k in groups))

    def per_civ(key, fmt='%.2f'):
        return lambda v, k: fmt % (v.get(key, 0) / v['civs']) if v.get('civs') else '-'

    row('civs (player-games)', lambda v, k: v.get('civs', 0))
    row('final: alive', lambda v, k: pct(v.get('final_alive', 0), v['civs']).strip())
    row('final: cities / civ', per_civ('final_cities'))
    row('final: techs / civ', per_civ('final_techs', '%.1f'))
    row('final: legacy score / civ', per_civ('final_score', '%.0f'))
    row('final: power / civ', per_civ('final_power', '%.0f'))
    for i, name in enumerate(('1st third', '2nd third', 'end')):
        row('cities coastal / all, %s' % name, lambda v, k, i=i: '%d/%d' % (v.get('third%d_coastal' % i, 0), v.get('third%d_cities' % i, 0)))
    row('cities off the start landmass, end', lambda v, k: v.get('third2_off_start', 0))
    print('  ships built (count, hammers):')
    for cls in SEA_CLS:
        row('  ' + cls, lambda v, k, cls=cls: '%d (%d)' % (v.get('built_%s_n' % cls, 0), v.get('built_%s_cost' % cls, 0)))
    row('  able to enter every water terrain', lambda v, k: v.get('built_reach_ocean', 0))
    row('  ships\' share of unit+building hammers', lambda v, k: pct(
        v.get('hammers_ship', 0), v.get('hammers_ship', 0) + v.get('hammers_unit', 0) + v.get('hammers_building', 0)).strip())
    print('  first turn with (share of civs, median turn):')
    for kind in ('ship', 'warship', 'ocean_ship', 'ocean_transport'):
        row('  ' + kind, lambda v, k, kind=kind: '%s t%s' % (
            pct(len(nav['firsts'][k].get(kind, [])), v['civs']).strip(), median(nav['firsts'][k].get(kind, [])) or '-'))
    print('  sea census (unit-turns at the end of the owner\'s turn; port = in a city of its team, own = own waters, '
          'sea = elsewhere; still = spent no movement):')
    for cls in SEA_CLS:
        def cen(v, k, cls=cls):
            tot = {pl: sum(v.get('sea_%s_%s_%s' % (w, cls, pl), 0) for w in ('war', 'peace')) for pl in SEA_PLACES}
            still = sum(v.get('sea_%s_%s_%s_still' % (w, cls, pl), 0) for w in ('war', 'peace') for pl in SEA_PLACES)
            n = sum(tot.values())
            if not n:
                return '0'
            return '%d %s/%s/%s s%s' % (n, *(pct(tot[pl], n).strip()[:-3] for pl in SEA_PLACES), pct(still, n).strip()[:-3])
        row('  %s (n port/own/sea%% still%%)' % cls, cen)
    row('  transport-turns: military aboard', lambda v, k: sum(v.get('cargo_%s_military' % w, 0) for w in ('war', 'peace')))
    row('  transport-turns: settler aboard', lambda v, k: sum(v.get('cargo_%s_settler' % w, 0) for w in ('war', 'peace')))
    row('  transport-turns: empty', lambda v, k: sum(v.get('cargo_%s_empty' % w, 0) for w in ('war', 'peace')))
    print('  naval fights (a civ\'s ships):')
    row('  attacking / defending / ships lost', lambda v, k: '%d/%d/%d' % (v.get('fight_att', 0), v.get('fight_def', 0), v.get('ships_lost', 0)))
    print('  sea transport:')
    row('  loaded: military / settler / other', lambda v, k: '%d/%d/%d' % (v.get('load_military', 0), v.get('load_settler', 0), v.get('load_other', 0)))
    for kind in ('military', 'settler'):
        row('  unloaded %s: port/home/landing' % kind, lambda v, k, kind=kind: '%d/%d/%d' % tuple(
            v.get('unload_%s_%s' % (kind, w), 0) for w in ('port', 'home', 'landing')))
    row('  cargo died aboard', lambda v, k: v.get('unload_died_aboard', 0))
    row('  trips started / ended', lambda v, k: '%d/%d' % (v.get('trips_started', 0), v.get('trips_ended', 0)))
    for kind in ('military', 'settler'):
        row('  %s trips: landing/home/port' % kind, lambda v, k, kind=kind: '%d/%d/%d' % tuple(
            v.get('trip_%s_%s' % (kind, o), 0) for o in ('landing', 'home', 'port')))
        row('  %s trips: lost at sea/ship lost' % kind, lambda v, k, kind=kind: '%d/%d' % tuple(
            v.get('trip_%s_%s' % (kind, o), 0) for o in ('lost_at_sea', 'ship_lost')))
    row('  military trips: never seen out of port', lambda v, k: v.get('trip_military_never_left', 0))
    row('  military trips: turns in port, median', lambda v, k: median(nav['military_trip_waits'][k]) if nav['military_trip_waits'][k] else '-')
    row('  landings: military (units)', lambda v, k: '%d (%d)' % (v.get('landing_military', 0), v.get('landing_military_units', 0)))
    row('  landings: settler (units)', lambda v, k: '%d (%d)' % (v.get('landing_settler', 0), v.get('landing_settler_units', 0)))
    row('  military landings: no enemy city next', lambda v, k: v.get('landing_no_target', 0))
    row('  military landings: assault / capture', lambda v, k: '%d/%d' % (v.get('landing_assaulted', 0), v.get('landing_captured', 0)))
    print('  cities:')
    row('  fell (lost): coastal / by sea / all', lambda v, k: '%d/%d/%d' % (v.get('fell_coastal', 0), v.get('fell_by_sea', 0), v.get('fell', 0)))
    row('  fell, captor had no city on its land', lambda v, k: v.get('fell_captor_no_city_on_land', 0))
    row('  took: by sea / overseas / all', lambda v, k: '%d/%d/%d' % (v.get('took_by_sea', 0), v.get('took_overseas', 0), v.get('took', 0)))
    row('  overseas cities founded / kept', lambda v, k: '%d/%d' % (v.get('overseas_founded', 0), v.get('overseas_kept', 0)))
    print('  blockade and raiding (at war):')
    row('  player-turns at war', lambda v, k: v.get('war_turns', 0))
    row('  armed-ship-turns next to enemy city', lambda v, k: v.get('raid_near_enemy_city', 0))
    row('  armed-ship-turns on enemy sea resource', lambda v, k: v.get('raid_on_enemy_resource', 0))
    row('  armed-ship-turns blockading', lambda v, k: v.get('raid_blockading', 0))
    row('  own sea improvements pillaged / lost', lambda v, k: '%d/%d' % (v.get('sea_pillaged', 0), v.get('sea_imp_lost', 0)))
    f = nav['fights']
    print('\n  naval fights (every duel with a sea unit or on water): %d' % f.get('n', 0))
    if f.get('n'):
        print('    by sides: ' + ', '.join('%s %d' % (k[5:], v) for k, v in sorted(f.items()) if k.startswith('pair_')))
        print('    by classes: ' + ', '.join('%s %d' % (k, v) for k, v in sorted(f.items()) if k.startswith('att_')))
        print('    results: ' + ', '.join('%s %d' % (k[4:], v) for k, v in sorted(f.items()) if k.startswith('res_')) +
              '; undefended %d; mean odds %.0f%%' % (f.get('undefended', 0), f.get('odds_sum', 0) / f['n'] / 10.0))
        print('    where (plot owner): ' + ', '.join('%s %d' % (k[7:], v) for k, v in sorted(f.items()) if k.startswith('waters_')))
        print('    transports sunk with cargo %d, units drowned %d' % (f.get('transports_sunk_with_cargo', 0), f.get('drowned', 0)))
    print('  sea improvements pillaged: by civilizations %d, by barbarians %d (on a resource %d); owned sea improvements '
          'removed, by rules context: %s' % (f.get('pillage_by_civ', 0), f.get('pillage_by_barb', 0), f.get('pillage_on_resource', 0),
                                             ', '.join('%s %d' % kv for kv in sorted(nav['sea_imp_lost'].items())) or 'none'))
    bs = {k[len('barb_sea_'):]: v for k, v in f.items() if k.startswith('barb_sea_')}
    tot = sum(v for k, v in bs.items() if not k.endswith('_still'))
    still = sum(v for k, v in bs.items() if k.endswith('_still'))
    print('  barbarian sea units: %d unit-turns, still %s; by role (unit-turns, in port, moved): %s' % (
        tot, pct(still, tot).strip(), ', '.join('%s %d/%d/%d' % (r[7:], *v) for r, v in sorted(nav['barb_roles'].items())) or 'none'))
    print('  civilizations\' sea units by unit AI (unit-turns, in port, moved): ' + (', '.join(
        '%s %d/%d/%d' % (r[7:], *v) for r, v in sorted(nav['roles'].items(), key=lambda kv: -kv[1][0])) or 'none'))
    w = nav['wars']
    print('  wars between civilizations: sharing a landmass %d (with a fight %d, a landing %d); sharing none %d '
          '(with a fight %d, a landing %d)' % (
              w.get('shared_land', 0), w.get('shared_land_fought', 0), w.get('shared_land_landed', 0),
              w.get('no_shared_land', 0), w.get('no_shared_land_fought', 0), w.get('no_shared_land_landed', 0)))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--bin', help='build output directory holding ffh (or the ffh binary)')
    ap.add_argument('--seeds', default='1-6')
    ap.add_argument('--turns', type=int, default=300)
    ap.add_argument('--civs', type=int, default=6)
    ap.add_argument('--size', default='WORLDSIZE_STANDARD')
    ap.add_argument('--env', action='append', default=[], help='K=V for the games (repeatable)')
    ap.add_argument('--arg', action='append', default=[],
                    help="more `ffh stats` arguments, e.g. --arg='--sea-level SEALEVEL_HIGH' or --arg=--until-supremacy "
                         "(repeatable)")
    ap.add_argument('--jobs', type=int, default=os.cpu_count() or 4)
    ap.add_argument('--out', help='directory for the games played (with --bin)')
    ap.add_argument('--load', help='report from the games in this directory')
    ap.add_argument('--by', choices=['civ'], help='also per civilization')
    ap.add_argument('--json', help='write everything to this file')
    ap.add_argument('--sections', default=','.join(SECTIONS),
                    help='report sections, comma-separated (default all): ' + ', '.join(SECTIONS))
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
    sections = [s for s in args.sections.split(',') if s]
    for s in sections:
        if s not in SECTIONS:
            ap.error('unknown section %s (known: %s)' % (s, ', '.join(SECTIONS)))
    rep = analyse(games, by_civ)
    if 'naval' in sections:
        rep['naval'] = analyse_naval(games)
    print_report(rep, by_civ, sections)
    if 'naval' in sections:
        print_naval(rep['naval'])
    if args.json:
        with open(args.json, 'w') as f:
            json.dump(rep, f, indent=1, default=str)


if __name__ == '__main__':
    main()
