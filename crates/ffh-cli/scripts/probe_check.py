#!/usr/bin/env python3
"""Did slot 0 play its probe's strategy? (docs/PROBES.md)

Reads `ai_metrics.py --out` files of games played with `--civ CIVILIZATION_X` (the civ in slot 0)
and FFH_TELEMETRY=1, and prints one row per game for player 0 plus the mean of each column, so a
probe's games can be set beside the standard AI's on the same seeds:

  probe      the probe the telemetry recorded (intent `probe`), '-' for the standard AI
  alive      alive at the end; c100/c200/cEnd cities at turns 100 and 200 and at the end
  share      score v2 at the end (basis points of the world, 0 when dead); rank among the civs
  first_war  turn of the first war its own AI declared; wars: wars its AI declared (a recorded
             reason), forced: declared for it by the rules (defensive pacts, council
             resolutions, a master's war), wars_on: declared on it, turns_war: turns at war with a
             civilization
  att_pct    attackers among the military units it trained (by the unit's default AI);
  mil_pct    military units' share of the hammers it put into units and buildings
  defs_city  city defenders alive per city at the end (units whose default AI is city defense)
  taken/lost civilization cities taken and kept / cities lost; killed: players it eliminated
  techs      techs known at the end

usage: probe_check.py FILE.json [FILE.json ..]     (one table per file)
"""
import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', 'tools'))
import jsonc  # noqa: E402

UNITS = {u['type']: u for u in jsonc.table('units')}
BUILDINGS = {b['type']: b for b in jsonc.table('buildings')}
ATTACK = {'UNITAI_ATTACK', 'UNITAI_ATTACK_CITY', 'UNITAI_COLLATERAL', 'UNITAI_COUNTER', 'UNITAI_PILLAGE'}
DEFENSE = {'UNITAI_CITY_DEFENSE', 'UNITAI_CITY_COUNTER'}
COLS = ['seed', 'civ', 'probe', 'alive', 'c100', 'c200', 'cEnd', 'share', 'rank', 'first_war', 'wars', 'forced', 'wars_on',
        'turns_war', 'att_pct', 'mil_pct', 'defs_city', 'taken', 'lost', 'killed', 'techs']


def kind(t):
    u = UNITS.get(t) or UNITS.get('UNIT_' + t, {})
    if u.get('combat', 0) <= 0 or u.get('found'):
        return 'civil'
    ai = u.get('default_unit_ai')
    return 'att' if ai in ATTACK else 'def' if ai in DEFENSE else 'other'


def row(g):
    final = str(max(int(k) for k in g['snapshots']))
    rows = g['snapshots'][final]
    r0 = [x for x in rows if x['p'] == 0][0]
    team = r0['team']
    tel = g.get('telemetry') or {}
    wars = [w for w in tel.get('wars', []) if not w['barb']]
    own = [w for w in wars if w['a'] == team and w.get('reason')]
    turns = set()
    for w in wars:
        if team in (w['a'], w['b']):
            turns.update(range(w['start'], w['end'] if w['end'] is not None else g['final_turn']))
    p0 = tel.get('players', {}).get('0', {})
    counts = p0.get('counts', {})
    n = {'att': 0, 'def': 0, 'other': 0, 'civil': 0}
    mil_h = all_h = 0
    for t, v in counts.get('built_unit', {}).items():
        k = kind(t)
        n[k] += v[0]
        h = (UNITS.get(t) or {}).get('cost', 0) * v[0]
        all_h += h
        mil_h += h if k != 'civil' else 0
    for t, v in counts.get('built_building', {}).items():
        all_h += BUILDINGS.get(t, {}).get('cost', 0) * v[0]
    probe = p0.get('timelines', {}).get('probe')
    defs = sum(v for t, v in (r0.get('unit_types') or {}).items() if kind(t) == 'def')

    def at(t):
        rs = g['snapshots'].get(str(t))
        return None if rs is None else [x for x in rs if x['p'] == 0][0]['cities']
    alive = r0['alive']
    return dict(seed=g['seed'], civ=r0['civ'], probe=probe[0][1] if probe else '-', alive=alive, c100=at(100), c200=at(200),
                cEnd=r0['cities'] if alive else 0, share=r0['score'] if alive else 0,
                rank=1 + sum(1 for x in rows if not x.get('barb') and x['p'] != 0 and x.get('score', 0) > (r0['score'] if alive else 0)),
                first_war=min((w['start'] for w in own), default=None), wars=len(own),
                forced=sum(1 for w in wars if w['a'] == team and not w.get('reason')), wars_on=sum(1 for w in wars if w['b'] == team),
                turns_war=len(turns), att_pct=round(100 * n['att'] / max(1, n['att'] + n['def'] + n['other'])),
                mil_pct=round(100 * mil_h / max(1, all_h)), defs_city=round(defs / max(1, r0['cities']), 1),
                taken=r0.get('taken_civ', 0), lost=r0.get('lost_civ', 0) + r0.get('lost_barb', 0), killed=r0.get('players_killed', 0),
                techs=r0.get('techs'))


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    for path in sys.argv[1:]:
        rows = [row(g) for g in json.load(open(path))]
        print('== %s: %d games' % (path, len(rows)))
        print(' '.join('%9s' % c for c in COLS))
        for r in rows:
            print(' '.join('%9s' % str(r[c])[:9] for c in COLS))

        def mean(c):
            v = [r[c] for r in rows if isinstance(r[c], (int, float)) and not isinstance(r[c], bool)]
            return '%.1f' % (sum(v) / len(v)) if v else '-'
        print(' '.join('%9s' % ('' if c in ('seed', 'civ', 'probe') else mean(c)) for c in COLS))
        print('alive %d/%d, first in share %d, declared a war %d' % (sum(r['alive'] for r in rows), len(rows), sum(r['rank'] == 1 for r in rows),
                                                                   sum(r['first_war'] is not None for r in rows)))


if __name__ == '__main__':
    main()
