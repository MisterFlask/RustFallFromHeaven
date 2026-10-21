#!/usr/bin/env python3
"""Summarize game telemetry (docs/TELEMETRY.md): what the players did, with what, and why.

  FFH_TELEMETRY=1 ffh stats --seed 1 ... > game.json
  python3 crates/ffh-cli/scripts/telemetry.py game.json [more.json ...] [--by side|civ|player]
      [--sections actions,combat,deaths,spells,wars,decisions,intents] [--top 12]

Inputs: `ffh stats` output (one JSON object, with "telemetry"), or a list of games as written by
h2h.py --out / runs of that shape ([{"stats": {...}}, ...]). Players are grouped by the AI that
played them (`--by side`, the default when both AIs played), by civilization or by player slot;
barbarians are left out. Numbers are per civilization and game (the group's civ-games), so groups
of different sizes compare.
"""
import argparse
import json
import sys
from collections import defaultdict

SECTIONS = ['actions', 'combat', 'deaths', 'spells', 'wars', 'decisions', 'intents']


def load_games(paths):
    games = []
    for path in paths:
        with open(path) as f:
            text = f.read().strip()
        try:
            data = json.loads(text)
        except json.JSONDecodeError:
            data = json.loads([l for l in text.split('\n') if l.startswith('{')][-1])
        items = data if isinstance(data, list) else [data]
        for it in items:
            st = it.get('stats', it)
            if 'telemetry' not in st:
                print('%s: no telemetry (run with FFH_TELEMETRY=1)' % path, file=sys.stderr)
                continue
            games.append(st)
    return games


def players_of(st):
    """player id (str) -> row of the last snapshot (civ, ctrl, barb, ...)."""
    snaps = st.get('snapshots', {})
    out = {}
    for t in sorted(snaps, key=int):
        for r in snaps[t]:
            out[str(r['p'])] = r
    return out


def short(name):
    for pre in ('MISSION_', 'MISSIONAI_', 'UNIT_', 'BUILDING_', 'BUILD_', 'PROMOTION_', 'SPELL_', 'TECH_', 'CIVIC_',
                'HURRY_', 'TRADE_', 'RELIGION_', 'PROJECT_', 'PROCESS_', 'WARPLAN_', 'CIVILIZATION_'):
        if name.startswith(pre):
            return name[len(pre):]
    return name


class Group:
    def __init__(self):
        self.civ_games = 0
        self.counts = defaultdict(lambda: defaultdict(lambda: [0, 0]))
        self.spells = defaultdict(lambda: defaultdict(int))
        self.spell_firsts = defaultdict(lambda: defaultdict(list))
        self.spell_reasons = defaultdict(lambda: defaultdict(int))
        self.spell_world = {}
        self.decisions = defaultdict(lambda: defaultdict(lambda: [0, 0]))
        self.whys = defaultdict(lambda: defaultdict(int))
        self.intent_time = defaultdict(lambda: defaultdict(int))
        self.intent_changes = defaultdict(int)
        self.wars = []

    def add_player(self, tp, final_turn):
        self.civ_games += 1
        for cat, items in tp.get('counts', {}).items():
            for k, (n, s) in items.items():
                c = self.counts[cat][k]
                c[0] += n
                c[1] += s
        for sp, st in tp.get('spells', {}).items():
            for k, v in st.items():
                if isinstance(v, (int, float)) and not isinstance(v, bool) and not k.startswith('first') and k != 'last_cast':
                    self.spells[sp][k] += v
            for k in ('first_known', 'first_castable', 'first_cast'):
                if st.get(k) is not None:
                    self.spell_firsts[sp][k].append(st[k])
            for r, n in st.get('reasons', {}).items():
                self.spell_reasons[sp][r] += n
            self.spell_world[sp] = st.get('world', False)
        for kind, items in tp.get('decisions', {}).items():
            for k, (n, v) in items.items():
                d = self.decisions[kind][k]
                d[0] += n
                d[1] += v
        for kind, items in tp.get('whys', {}).items():
            for k, n in items.items():
                self.whys[kind][k] += n
        for kind, line in tp.get('timelines', {}).items():
            self.intent_changes[kind] += len(line)
            for i, (t, v) in enumerate(line):
                end = line[i + 1][0] if i + 1 < len(line) else final_turn
                self.intent_time[kind][v] += max(0, end - t)


def median(xs):
    xs = sorted(xs)
    return xs[len(xs) // 2] if xs else None


def per(g, n):
    return n / max(1, g.civ_games)


def section_actions(groups, top):
    print('\n== Actions (per civ and game)')
    names = list(groups)
    rows = [
        ('techs', lambda g: per(g, g.counts['tech']['n'][0])),
        ('cities founded', lambda g: per(g, g.counts['city']['founded'][0])),
        ('cities taken (conquest)', lambda g: per(g, g.counts['city_taken']['conquest'][0])),
        ('cities taken from barbarians', lambda g: per(g, g.counts['city_taken']['from_barbarians'][0])),
        ('cities lost (conquest)', lambda g: per(g, g.counts['city_lost']['conquest'][0])),
        ('cities razed', lambda g: per(g, g.counts['city']['razed'][0])),
        ('hurries', lambda g: per(g, sum(v[0] for v in g.counts['hurry'].values()))),
        ('  gold spent hurrying', lambda g: per(g, sum(v[1] for v in g.counts['hurry'].values()))),
        ('  population spent hurrying', lambda g: per(g, sum(v[1] for v in g.counts['hurry_pop'].values()))),
        ('upgrades', lambda g: per(g, sum(v[0] for v in g.counts['upgrade_to'].values()))),
        ('  gold spent upgrading', lambda g: per(g, sum(v[1] for v in g.counts['upgrade_to'].values()))),
        ('promotions', lambda g: per(g, sum(v[0] for v in g.counts['promote'].values()))),
        ('improvements built', lambda g: per(g, sum(v[0] for v in g.counts['build'].values()))),
        ('units built', lambda g: per(g, sum(v[0] for v in g.counts['built_unit'].values()))),
        ('buildings built', lambda g: per(g, sum(v[0] for v in g.counts['built_building'].values()))),
        ('deals signed', lambda g: per(g, g.counts['deal']['n'][0])),
        ('revolutions', lambda g: per(g, g.counts['revolution']['n'][0])),
        ('  anarchy turns', lambda g: per(g, g.counts['revolution']['n'][1])),
        ('religion conversions', lambda g: per(g, sum(v[0] for v in g.counts['convert'].values()))),
        ('wars declared', lambda g: per(g, sum(v[0] for v in g.counts['war_declared'].values()))),
        ('caster unit-turns', lambda g: per(g, g.counts['casters']['unit_turns'][1])),
        ('  idle with a spell castable', lambda g: per(g, g.counts['casters']['idle_castable'][1])),
        ('missions pushed', lambda g: per(g, sum(v[0] for v in g.counts['mission'].values()))),
    ]
    print('%-32s' % '' + ''.join('%14s' % n for n in names))
    for label, f in rows:
        print('%-32s' % label + ''.join('%14.1f' % f(groups[n]) for n in names))
    for cat, label in [('mission', 'missions'), ('mission_ai', 'mission AI tags'), ('built_unit', 'units built'),
                       ('built_building', 'buildings built'), ('build', 'improvements'), ('promote', 'promotions'),
                       ('upgrade_to', 'upgraded to'), ('hurried', 'hurried'), ('deal_gave', 'deal items given'), ('civic', 'civics adopted')]:
        top_table(groups, label, lambda g, cat=cat: {k: v[0] for k, v in g.counts[cat].items()}, top)


def top_table(groups, label, getter, top, fmt='%14.1f', per_game=True):
    names = list(groups)
    vals = {n: getter(groups[n]) for n in names}
    keys = set()
    for n in names:
        keys |= set(vals[n])
    if not keys:
        return
    def score(k):
        return max(per(groups[n], vals[n].get(k, 0)) if per_game else vals[n].get(k, 0) for n in names)
    keys = sorted(keys, key=lambda k: -score(k))[:top]
    print('\n-- %s' % label)
    for k in keys:
        cells = ''.join(fmt % (per(groups[n], vals[n].get(k, 0)) if per_game else vals[n].get(k, 0)) for n in names)
        print('  %-30s' % short(k)[:30] + cells)


def section_combat(groups, top):
    print('\n== Combat (per civ and game)')
    names = list(groups)
    print('%-32s' % '' + ''.join('%14s' % n for n in names))
    def c(g, cat, k, i=0):
        return g.counts[cat][k][i]
    rows = [
        ('attacks', lambda g: per(g, sum(c(g, 'attack', k) for k in ('win', 'loss', 'withdraw')))),
        ('  won %', lambda g: 100 * c(g, 'attack', 'win') / max(1, sum(c(g, 'attack', k) for k in ('win', 'loss', 'withdraw')))),
        ('  withdrew %', lambda g: 100 * c(g, 'attack', 'withdraw') / max(1, sum(c(g, 'attack', k) for k in ('win', 'loss', 'withdraw')))),
        ('  on undefended units', lambda g: per(g, c(g, 'attack', 'undefended'))),
        ('  vs civilizations', lambda g: per(g, c(g, 'attack_vs', 'civ'))),
        ('    won %', lambda g: 100 * c(g, 'attack_vs', 'civ', 1) / max(1, c(g, 'attack_vs', 'civ'))),
        ('  vs barbarians', lambda g: per(g, c(g, 'attack_vs', 'barbarian'))),
        ('  vs animals', lambda g: per(g, c(g, 'attack_vs', 'animal'))),
        ('defenses', lambda g: per(g, sum(c(g, 'defend', k) for k in ('win', 'loss', 'withdraw')))),
        ('  held %', lambda g: 100 * c(g, 'defend', 'win') / max(1, sum(c(g, 'defend', k) for k in ('win', 'loss', 'withdraw')))),
        ('  in cities', lambda g: per(g, c(g, 'defend_in_city', 'yes'))),
        ('    lost in cities', lambda g: per(g, c(g, 'defend_in_city', 'yes', 1))),
    ]
    for label, f in rows:
        print('%-32s' % label + ''.join('%14.1f' % f(groups[n]) for n in names))
    print('\n-- attacks by odds (attacker\'s win chance as the battle began): n per game / expected % / won %')
    for b in ['%02d' % (10 * i) for i in range(10)]:
        cells = ''
        for n in names:
            g = groups[n]
            nn, won = g.counts['attack_odds'][b]
            exp = g.counts['attack_odds_exp'][b][1]
            cells += '%14s' % ('%.1f/%2.0f/%2.0f' % (per(g, nn), exp / 10 / max(1, nn), 100 * won / max(1, nn)) if nn else '-')
        print('  %-30s' % ('%s-%d%%' % (b, int(b) + 10)) + cells)
    top_table(groups, 'combats won, by unit', lambda g: {k: v[0] for k, v in g.counts['kill_by'].items()}, top)
    top_table(groups, 'combat units lost (combat, spells, killed)', lambda g: {k: v[0] for k, v in g.counts['lost_unit'].items()}, top)


def section_deaths(groups, top):
    top_table(groups, 'units gone, by cause (per civ and game)', lambda g: {k: v[0] for k, v in g.counts['died'].items()}, top)


def section_spells(groups, top):
    print('\n== Spells (per civ and game). known, castable: unit-turns at the start of the owner\'s turn;')
    print('   idle: unit-turns ending with the spell castable and nothing cast; outcome: per civ and game, summed over casts')
    for n, g in groups.items():
        rows = []
        for sp, st in g.spells.items():
            if st['known'] == 0 and st['casts'] == 0:
                continue
            rows.append((sp, st))
        rows.sort(key=lambda r: (-r[1]['casts'], -r[1]['known']))
        print('\n-- %s: %d spells known or cast; %d cast at least once; %.1f casts per civ and game' % (
            n, len(rows), sum(1 for _, st in rows if st['casts']), per(g, sum(st['casts'] for _, st in rows))))
        print('  %-26s %8s %8s %7s %7s %7s %7s  %-28s %s' % ('spell', 'known', 'castable', 'idle', 'casts', '1st kn', '1st cast', 'reasons', 'outcome'))
        for sp, st in rows[:top * 2]:
            fk = median(g.spell_firsts[sp]['first_known'])
            fc = median(g.spell_firsts[sp]['first_cast'])
            tot = sum(g.spell_reasons[sp].values()) or 1
            reasons = ','.join('%s %d%%' % (r, 100 * k // tot) for r, k in sorted(g.spell_reasons[sp].items(), key=lambda x: -x[1])[:3])
            out = ' '.join('%s %.1f' % (k, per(g, st[k])) for k in ('kills', 'damage', 'cursed', 'converted', 'others_buffed', 'summons', 'summon_kills', 'buffs', 'healed', 'own_lost') if st.get(k))
            print('  %-26s %8.1f %8.1f %7.1f %7.1f %7s %7s  %-28s %s' % (
                short(sp)[:26] + ('*' if g.spell_world.get(sp) else ''), per(g, st['known']), per(g, st['castable']), per(g, st['idle']), per(g, st['casts']),
                fk if fk is not None else '-', fc if fc is not None else '-', reasons[:28], out))
        never = [(sp, st) for sp, st in rows if st['casts'] == 0 and (st['castable'] > 0 or st['idle'] > 0)]
        never.sort(key=lambda r: -(r[1]['castable'] + r[1]['idle']))
        if never:
            print('  castable, never cast (idle unit-turns): ' + ', '.join('%s (%.0f)' % (short(sp), per(g, st['idle'])) for sp, st in never[:top]))


def section_wars(groups, top):
    print('\n== Wars between civilizations (the declaring side\'s view)')
    names = list(groups)
    print('%-32s' % '' + ''.join('%14s' % n for n in names))
    def stat(g, f):
        ws = [w for w in g.wars if not w['barb']]
        return f(ws) if ws else 0
    rows = [
        ('wars declared per civ-game', lambda g: stat(g, lambda ws: len(ws) / max(1, g.civ_games))),
        ('  mean length (turns)', lambda g: stat(g, lambda ws: sum(w['turns'] for w in ws) / len(ws))),
        ('  ended in peace %', lambda g: stat(g, lambda ws: 100 * sum(1 for w in ws if w['end'] is not None) / len(ws))),
        ('  cities taken by declarer', lambda g: stat(g, lambda ws: sum(w['cities'][0] for w in ws) / len(ws))),
        ('  cities taken by target', lambda g: stat(g, lambda ws: sum(w['cities'][1] for w in ws) / len(ws))),
        ('  combats won by declarer', lambda g: stat(g, lambda ws: sum(w['kills'][0] for w in ws) / len(ws))),
        ('  combats won by target', lambda g: stat(g, lambda ws: sum(w['kills'][1] for w in ws) / len(ws))),
        ('  gained a city %', lambda g: stat(g, lambda ws: 100 * sum(1 for w in ws if w['cities'][0] > 0) / len(ws))),
    ]
    for label, f in rows:
        print('%-32s' % label + ''.join('%14.1f' % f(groups[n]) for n in names))
    for n, g in groups.items():
        ws = [w for w in g.wars if not w['barb']]
        if not ws:
            continue
        by = defaultdict(lambda: [0, 0])
        for w in ws:
            r = (w.get('reason') or 'no reason given').split(' (')[0].split(';')[0]
            by[r][0] += 1
            by[r][1] += w['cities'][0]
        print('\n-- %s: why (cities taken by the declarer)' % n)
        for r, (k, cities) in sorted(by.items(), key=lambda x: -x[1][0])[:top]:
            print('  %-50s %4d wars, %4d cities' % (r[:50], k, cities))


def section_decisions(groups, top):
    for kind in sorted({k for g in groups.values() for k in g.whys}):
        top_table(groups, 'decisions "%s", by the rule that made them (per civ and game)' % kind, lambda g, kind=kind: dict(g.whys[kind]), top)
    for kind in sorted({k for g in groups.values() for k in g.decisions}):
        if kind in ('settle_target',):
            continue
        top_table(groups, 'decisions "%s", by choice (per civ and game)' % kind, lambda g, kind=kind: {k: v[0] for k, v in g.decisions[kind].items()}, top)


def section_intents(groups, top):
    print('\n== Plans (share of turns; changes per civ and game)')
    for kind in sorted({k for g in groups.values() for k in g.intent_time}):
        if kind == 'campaign':
            continue
        def share(g, kind=kind):
            tot = sum(g.intent_time[kind].values()) or 1
            return {v: 100.0 * t / tot for v, t in g.intent_time[kind].items()}
        top_table(groups, '"%s" (%% of turns)' % kind, share, top, per_game=False)
        print('  %-30s' % 'changes' + ''.join('%14.1f' % per(g, g.intent_changes[kind]) for g in groups.values()))
    if any(g.intent_changes.get('campaign') for g in groups.values()):
        print('\n-- campaign target changes per civ and game: ' + ', '.join('%s %.1f' % (n, per(g, g.intent_changes['campaign'])) for n, g in groups.items()))


def summarize(games, by=None, sections=SECTIONS, top=12):
    """Print the summary of `games` (`ffh stats` objects with "telemetry")."""
    if by is None:
        ctrls = {r.get('ctrl') for st in games for r in players_of(st).values() if not r.get('barb')}
        by = 'side' if len(ctrls) > 1 else 'civ'
    groups = defaultdict(Group)
    for st in games:
        rows = players_of(st)
        tel = st['telemetry']
        final_turn = st.get('final_turn') or st.get('turns') or 0
        key_of = {}
        for p, r in rows.items():
            if r.get('barb'):
                continue
            key_of[p] = {'side': r.get('ctrl', '?'), 'civ': short(r.get('civ', '?')), 'player': 'p' + p}[by]
        for p, tp in tel['players'].items():
            if p in key_of:
                groups[key_of[p]].add_player(tp, final_turn)
        for w in tel.get('wars', []):
            decl = w['a_players'][0] if w['a_players'] else None
            if decl is not None and str(decl) in key_of:
                groups[key_of[str(decl)]].wars.append(w)
    groups = dict(sorted(groups.items()))
    print('%d games; groups by %s: %s' % (len(games), by, ', '.join('%s (%d civ-games)' % (n, g.civ_games) for n, g in groups.items())))
    for s in sections:
        globals()['section_' + s](groups, top)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('files', nargs='+')
    ap.add_argument('--by', choices=['side', 'civ', 'player'], default=None)
    ap.add_argument('--sections', default=','.join(SECTIONS))
    ap.add_argument('--top', type=int, default=12)
    args = ap.parse_args()
    games = load_games(args.files)
    if not games:
        sys.exit('no games with telemetry')
    summarize(games, args.by, args.sections.split(','), args.top)


if __name__ == '__main__':
    main()
