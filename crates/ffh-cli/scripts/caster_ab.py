#!/usr/bin/env python3
"""How the AIs use their arcane casters, on two sides of the same seeds (docs/AI.md, "Spellcasting").

Plays every seed on both sides with `ffh stats --every 50` and `FFH_AI_TRACE_CAST=1` (6 civs,
standard map, to a decision or turn 690), by default the same binary with the caster school off
on the base side (`FFH_AI_CASTER_SCHOOL_OFF=all`), and reports the arcane casters (unit combat
Adept, summons left out) of every civilization, summed over the games: made, lost (alone: outside
our cities with no unit of ours that fights; with their owner: eliminated), upgraded, caster-turns
(ready: Adepts of a Mage's level waiting for the upgrade; Mages: Channeling II, Archmages:
Channeling III) and their casts -- by the tier of the
Channeling the spell needs, by context (`attack`: before a strike, `turn`: the end of the turn)
and by kind (a summon, damage, a promotion, a city spell, Python) -- with the rates per 1000
caster-turns; the strikes of players with a Mage and how many had one within two plots (where it
casts before the strike), and the casts before strikes per city the caster civilizations took. Win rates are not its business (ai_metrics.py, trait_ab.py, civ_ab.py).

Games diverge, and a longer game has more caster-turns and more casters lost with an eliminated
owner: `--common` compares each seed's two games up to the last 50-turn checkpoint both reached.

usage:
  caster_ab.py --bin DIR --seeds 7901-7940 --out-dir D [--base-bin DIR] [--base-env K=V ..]
               [--new-env K=V ..] [--common] [--civ] [--load]
"""
import argparse
import collections
import json
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, '..', '..', '..'))
UNITS = os.path.join(ROOT, 'data', 'xml', 'Units')
FIELDS = ['arcane_made', 'arcane_lost', 'arcane_lost_alone', 'arcane_lost_with_owner', 'arcane_lost_levels',
          'arcane_upgraded', 'arcane_turns', 'arcane_ready_turns', 'arcane_mage_turns', 'arcane_archmage_turns']
CAST = re.compile(r'\[cast t(\d+) p(\d+)\] civ=(\S+) unit=(\S+) spell=(\S+) ctx=(\S+)')
STRIKE = re.compile(r'\[strike t(\d+) p(\d+)\] civ=(\S+) near=(\d+) near_mages=(\d+) mages=(\d+)')


def parse_seeds(s):
    out = []
    for part in s.split(','):
        a, _, b = part.partition('-')
        out.extend(range(int(a), int(b or a) + 1))
    return out


def tables():
    """Arcane unit names; each spell's Channeling tier (0: none) and kind."""
    units = open(os.path.join(UNITS, 'CIV4UnitInfos.xml')).read()
    arcane = set()
    for m in re.finditer(r'<UnitInfo>(.*?)</UnitInfo>', units, re.S):
        if '<Combat>UNITCOMBAT_ADEPT</Combat>' in m.group(1):
            arcane.add(re.search(r'<Type>UNIT_(.*?)</Type>', m.group(1)).group(1))
    promos = open(os.path.join(UNITS, 'CIV4PromotionInfos.xml')).read()
    tier = {}
    for m in re.finditer(r'<PromotionInfo>(.*?)</PromotionInfo>', promos, re.S):
        a = re.search(r'<PromotionPrereqAnd>PROMOTION_CHANNELING(\d)</PromotionPrereqAnd>', m.group(1))
        if a:
            tier[re.search(r'<Type>(.*?)</Type>', m.group(1)).group(1)] = int(a.group(1))
    spells = open(os.path.join(UNITS, 'CIV4SpellInfos.xml')).read()
    stier, kind = {}, {}
    for m in re.finditer(r'<SpellInfo>(.*?)</SpellInfo>', spells, re.S):
        b = m.group(1)
        t = re.search(r'<Type>SPELL_(.*?)</Type>', b).group(1)
        p1 = re.search(r'<PromotionPrereq1>(.*?)</PromotionPrereq1>', b)
        stier[t] = tier.get(p1.group(1), 0) if p1 else 0
        kind[t] = ('summon' if '<CreateUnitType>' in b else 'damage' if re.search(r'<iDamage>[1-9]', b)
                   else 'city' if '<CreateBuildingType>' in b else 'promo' if '<AddPromotionType1>' in b else 'python')
    return arcane, stier, kind


def play(binary, env, seed, out_dir, side):
    js, cast = os.path.join(out_dir, '%s_%d.json' % (side, seed)), os.path.join(out_dir, '%s_%d.cast' % (side, seed))
    if os.path.exists(js) and os.path.getsize(js) > 0:
        return
    e = dict(os.environ, FFH_AI_TRACE_CAST='1', **env)
    cmd = [os.path.join(ROOT, 'tools', 'agents', 'slot.sh'), os.path.join(binary, 'ffh'), 'stats', '--seed', str(seed), '--civs', '6',
           '--size', 'WORLDSIZE_STANDARD', '--turns', '690', '--until-supremacy', '--every', '50']
    with open(js, 'w') as fo, open(cast, 'w') as fe:
        subprocess.run(cmd, stdout=fo, stderr=fe, env=e, check=False)


def env_list(kv):
    return dict(x.split('=', 1) for x in kv)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--bin', help='build dir holding ffh (the new side)')
    ap.add_argument('--base-bin', help='build dir for the base side (default: --bin)')
    ap.add_argument('--seeds', required=True)
    ap.add_argument('--out-dir', required=True)
    ap.add_argument('--base-env', action='append', default=None, help='K=V for the base side (default FFH_AI_CASTER_SCHOOL_OFF=all)')
    ap.add_argument('--new-env', action='append', default=[], help='K=V for the new side')
    ap.add_argument('--jobs', type=int, default=4)
    ap.add_argument('--load', action='store_true', help='report on games already in --out-dir')
    ap.add_argument('--common', action='store_true', help='compare each seed up to the last checkpoint both games reached')
    ap.add_argument('--civ', action='store_true', help='also per civilization')
    a = ap.parse_args()
    seeds = parse_seeds(a.seeds)
    os.makedirs(a.out_dir, exist_ok=True)
    if not a.load:
        base_env = env_list(a.base_env if a.base_env is not None else ['FFH_AI_CASTER_SCHOOL_OFF=all'])
        new_env = env_list(a.new_env)
        jobs = [(a.base_bin or a.bin, base_env, s, 'base') for s in seeds] + [(a.bin, new_env, s, 'new') for s in seeds]
        with ThreadPoolExecutor(a.jobs) as ex:
            list(ex.map(lambda j: play(j[0], j[1], j[2], a.out_dir, j[3]), jobs))
    arcane, stier, kind = tables()

    def load(side, s):
        path = os.path.join(a.out_dir, '%s_%d.json' % (side, s))
        return json.load(open(path)) if os.path.exists(path) and os.path.getsize(path) > 0 else None

    games = {s: (load('base', s), load('new', s)) for s in seeds}
    games = {s: g for s, g in games.items() if g[0] and g[1]}
    tot = {side: collections.Counter() for side in ('base', 'new')}
    civ_tot = {side: collections.defaultdict(collections.Counter) for side in ('base', 'new')}
    for s, pair in sorted(games.items()):
        ends = [max(int(t) for t in g['snapshots']) for g in pair]
        for side, g, end in zip(('base', 'new'), pair, ends):
            if a.common:
                end = min(ends) // 50 * 50
            rows = g['snapshots'].get(str(end))
            if rows is None:
                continue
            tot[side]['games'] += 1
            tot[side]['turns'] += end
            users = set()
            for r in rows:
                if r.get('barb'):
                    continue
                c = civ_tot[side][r['civ']]
                c['players'] += 1
                for f in FIELDS:
                    tot[side][f] += r.get(f, 0)
                    c[f] += r.get(f, 0)
                if r.get('arcane_turns', 0) > 0:
                    users.add(r['civ'])
                    tot[side]['taken_by_caster_civs'] += r.get('taken_civ', 0) + r.get('taken_barb', 0)
            with open(os.path.join(a.out_dir, '%s_%d.cast' % (side, s))) as f:
                for line in f:
                    st = STRIKE.match(line)
                    if st and int(st.group(1)) <= end and int(st.group(6)) > 0:
                        # A strike by a player with a Mage somewhere: was one within casting range?
                        tot[side]['strikes_with_mages'] += 1
                        tot[side]['strikes_mage_near'] += int(st.group(5)) > 0
                        tot[side]['strikes_caster_near'] += int(st.group(4)) > 0
                        continue
                    m = CAST.match(line)
                    if not m or m.group(3) == 'BARBARIAN' or m.group(4) not in arcane or int(m.group(1)) > end:
                        continue
                    sp, ctx = m.group(5), m.group(6)
                    t, k = stier.get(sp, 0), kind.get(sp, '?')
                    if ctx == 'attack' and m.group(3) in users:
                        tot[side]['casts_attack_caster_civs'] += 1
                    for c in (tot[side], civ_tot[side][m.group(3)]):
                        c['casts'] += 1
                        c['casts_t%d' % t] += 1
                        c['casts_' + ctx] += 1
                        c['casts_' + k] += 1
                        if t >= 2 and k in ('summon', 'damage'):
                            c['casts_t2+_fight'] += 1
    print('%d seeds%s (base %s, new %s)' % (len(games), ', equal game time' if a.common else '', a.base_env or 'FFH_AI_CASTER_SCHOOL_OFF=all', a.new_env or '-'))
    keys = ['games', 'turns'] + FIELDS + ['casts', 'casts_t0', 'casts_t1', 'casts_t2', 'casts_t3', 'casts_attack', 'casts_turn',
                                          'casts_summon', 'casts_damage', 'casts_promo', 'casts_city', 'casts_python', 'casts_t2+_fight',
                                          'strikes_with_mages', 'strikes_mage_near', 'strikes_caster_near', 'taken_by_caster_civs']
    for k in keys:
        b, n = tot['base'][k], tot['new'][k]
        print('%-24s base %8d  new %8d  %s' % (k, b, n, ('%+.0f%%' % (100.0 * (n - b) / b)) if b else ''))
    for side in ('base', 'new'):
        t = tot[side]
        per = lambda k: 1000.0 * t[k] / max(1, t['arcane_turns'])
        print('%-4s per 1000 caster-turns: lost %.1f (alone %.1f, with owner %.1f), upgraded %.1f, casts %.0f, before strikes %.1f, '
              'Channeling II+ summons and damage %.1f; Mage+Archmage share of caster-turns %.2f, levels per lost caster %.1f' % (
                  side, per('arcane_lost'), per('arcane_lost_alone'), per('arcane_lost_with_owner'), per('arcane_upgraded'), per('casts'),
                  per('casts_attack'), per('casts_t2+_fight'), (t['arcane_mage_turns'] + t['arcane_archmage_turns']) / max(1, t['arcane_turns']),
                  t['arcane_lost_levels'] / max(1, t['arcane_lost'])))
        print('%-4s strikes by players with a Mage: %d, a Mage within two plots in %.0f%% (any caster %.0f%%); casts before attacks '
              'per such strike %.2f, per city the caster civilizations took %.2f' % (
                  side, t['strikes_with_mages'], 100.0 * t['strikes_mage_near'] / max(1, t['strikes_with_mages']),
                  100.0 * t['strikes_caster_near'] / max(1, t['strikes_with_mages']), t['casts_attack_caster_civs'] / max(1, t['strikes_with_mages']),
                  t['casts_attack_caster_civs'] / max(1, t['taken_by_caster_civs'])))
    if a.civ:
        print('%-14s %7s %7s %7s %7s %7s %11s %11s %9s %9s %9s' % ('civ (base/new)', 'n', 'made', 'lost', 'alone', 'upg', 'turns', 'mage', 'arch', 'casts', 'II+fight'))
        for c in sorted(set(civ_tot['base']) | set(civ_tot['new'])):
            b, n = civ_tot['base'][c], civ_tot['new'][c]
            if not (b['arcane_turns'] or n['arcane_turns']):
                continue
            print('%-14s %3d/%-3d %3d/%-3d %3d/%-3d %3d/%-3d %3d/%-3d %5d/%-5d %5d/%-5d %4d/%-4d %4d/%-4d %4d/%-4d' % (
                c, b['players'], n['players'], b['arcane_made'], n['arcane_made'], b['arcane_lost'], n['arcane_lost'], b['arcane_lost_alone'],
                n['arcane_lost_alone'], b['arcane_upgraded'], n['arcane_upgraded'], b['arcane_turns'], n['arcane_turns'], b['arcane_mage_turns'],
                n['arcane_mage_turns'], b['arcane_archmage_turns'], n['arcane_archmage_turns'], b['casts'], n['casts'], b['casts_t2+_fight'],
                n['casts_t2+_fight']))


if __name__ == '__main__':
    sys.exit(main())
