#!/usr/bin/env python3
"""Squads per tech level for the balance lab (docs/ENGAGEMENTS.md, "Balance lab"): each level's
generic units (each class's default unit), squads at an equal budget (early 480, mid 960, late
1920 hammers) within 10 slots (a large unit 2) and at most a national class's limit at the
normal price (the over-limit price is a deliberate sink, not balanced for): every unit alone,
front-line pairs, a front type with each support type (3:1, and 1:1 for the main ones), triples.

    battles_squads.py LEVEL [--compact]          the squads, `name|spec` (compact: without the 1:1
                                                 front-and-support pairs)
    battles_squads.py specs OUT LEVELS GROUNDS [ONLY.json] [--chunks N]
                                                 round-robin spec files for `ffh battles`
                                                 (GROUNDS from open,forest,hills,walls; ONLY: a
                                                 {level: [squad names]} file to restrict to)
    battles_squads.py casters OUT [GROUNDS] [--chunks N]
                                                 the casters' marginal lab (docs/UNIT_ROLES.md,
                                                 section 9): a core squad and two of each sphere's
                                                 caster, the bolt caster, a healer or the core
                                                 units, early and mid (GROUNDS: open,forest)

    python3 crates/ffh-cli/scripts/battles_squads.py specs OUT early,mid,late open
    for f in OUT/part*.txt; do tools/agents/slot.sh $B/ffh battles $f --runs 40 > ${f%.txt}.jsonl & done; wait
    python3 crates/ffh-cli/scripts/battles_meta.py meta mid OUT/*.jsonl
"""
import itertools, json, os, random, sys
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..')
sys.path.insert(0, os.path.join(ROOT, 'tools'))
import jsonc

U = {u['type'][5:]: u for u in jsonc.load(os.path.join(ROOT, 'data/json/units.jsonc'))}
C = {c['type']: c for c in jsonc.load(os.path.join(ROOT, 'data/json/unit_classes.jsonc'))}
COST = {k: u.get('cost', 0) for k, u in U.items()}
LARGE = {k for k, u in U.items() if u.get('large')}
CAP = {k: C[u['class']].get('max_player_instances', -1) for k, u in U.items()}
SLOTS = 10

SHORT = {
    'AXEMAN': 'axe', 'ARCHER': 'arch', 'HORSEMAN': 'horse', 'HUNTER': 'hunt', 'ADEPT': 'adept',
    'DISCIPLE_THE_ORDER': 'disc', 'CATAPULT': 'cat', 'CHARIOT': 'chariot', 'WAR_ELEPHANT': 'eleph',
    'SKIRMISHER': 'skirm', 'WARRIOR': 'war', 'SCOUT': 'scout', 'CHAMPION': 'champ', 'LONGBOWMAN': 'lb',
    'HORSE_ARCHER': 'ha', 'MAGE': 'mage', 'MAGE+FIRE1+FIRE2': 'fmage', 'PRIEST_OF_THE_ORDER': 'priest',
    'ASSASSIN': 'assn', 'RANGER': 'ranger', 'CRUSADER': 'crus', 'PHALANX': 'phal', 'IMMORTAL': 'imm',
    'KNIGHT': 'knight', 'CROSSBOWMAN': 'xbow', 'MARKSMAN': 'mksm', 'ARQUEBUS': 'arq',
    'ARCHMAGE': 'amage', 'ARCHMAGE+FIRE1+FIRE2': 'famage', 'HIGH_PRIEST_OF_THE_ORDER': 'hpriest',
    'CANNON': 'cannon', 'PALADIN': 'pal', 'BERSERKER': 'bers', 'BEASTMASTER': 'bmaster', 'DRUID': 'druid',
}
base = lambda s: s.split('+')[0]
size = lambda s: 2 if base(s) in LARGE else 1


def fill(template, budget):
    """Counts for `template` [(spec, share)], within the budget, the slots and national caps."""
    specs = [s for s, _ in template]
    share = dict(template)
    cap = lambda s: CAP[base(s)] if CAP[base(s)] > 0 else 99
    n = {s: min(cap(s), max(1, round(share[s] * budget / COST[base(s)]))) for s in specs}
    cost = lambda: sum(n[s] * COST[base(s)] for s in specs)
    slots = lambda: sum(n[s] * size(s) for s in specs)
    while cost() > budget or slots() > SLOTS:
        s = max((s for s in specs if n[s] > 1), key=lambda s: n[s] * COST[base(s)] / share[s], default=None)
        if s is None:
            break
        n[s] -= 1
    while True:
        ok = [s for s in specs if n[s] < cap(s) and cost() + COST[base(s)] <= budget and slots() + size(s) <= SLOTS]
        if not ok:
            break
        s = min(ok, key=lambda s: (n[s] * COST[base(s)] / share[s], specs.index(s)))
        n[s] += 1
    return n


# The rows a player would give an attacker the auto formation places worse (the auto formation is
# out of scope: docs/UNIT_ROLES.md): mounted units charge from the front. Only the attacker's
# formation follows these (`ffh battles`' `@row`); the defender's is the rules' auto formation.
ATTACK_ROWS = {'HORSEMAN': 'front', 'HORSE_ARCHER': 'front', 'KNIGHT': 'front', 'CHARIOT': 'front'}


def name_of(n):
    return '+'.join(f"{c}{SHORT[s]}" for s, c in sorted(n.items(), key=lambda kv: (-kv[1] * COST[base(kv[0])], kv[0])) if c)


def spec_of(n):
    row = lambda s: f"@{ATTACK_ROWS[base(s)]}" if base(s) in ATTACK_ROWS else ""
    return ', '.join(f"{c}*{s}{row(s)}" for s, c in n.items() if c)


LEVELS = {
    'early': dict(budget=480,
                  front=['AXEMAN', 'HORSEMAN', 'HUNTER', 'WAR_ELEPHANT', 'CHARIOT'],
                  support=['ARCHER', 'ADEPT', 'DISCIPLE_THE_ORDER', 'CATAPULT', 'SKIRMISHER'],
                  pure_only=['WARRIOR', 'SCOUT'],
                  triples=[('AXEMAN', 'ARCHER', x) for x in ['HORSEMAN', 'ADEPT', 'DISCIPLE_THE_ORDER', 'CATAPULT', 'SKIRMISHER', 'WAR_ELEPHANT']]
                  + [('HORSEMAN', 'ARCHER', x) for x in ['ADEPT', 'SKIRMISHER', 'WAR_ELEPHANT']],
                  half=['AXEMAN', 'HORSEMAN']),
    'mid': dict(budget=960,
                front=['CHAMPION', 'HORSE_ARCHER', 'RANGER', 'WAR_ELEPHANT', 'CRUSADER'],
                support=['LONGBOWMAN', 'MAGE', 'MAGE+FIRE1+FIRE2', 'PRIEST_OF_THE_ORDER', 'ASSASSIN', 'CATAPULT', 'ARCHER'],
                pure_only=['CHARIOT'],
                triples=[('CHAMPION', 'LONGBOWMAN', x) for x in ['HORSE_ARCHER', 'MAGE+FIRE1+FIRE2', 'ASSASSIN', 'PRIEST_OF_THE_ORDER', 'WAR_ELEPHANT', 'CATAPULT']]
                + [('HORSE_ARCHER', 'ASSASSIN', 'MAGE+FIRE1+FIRE2'), ('HORSE_ARCHER', 'LONGBOWMAN', 'MAGE+FIRE1+FIRE2')],
                half=['CHAMPION', 'HORSE_ARCHER']),
    'late': dict(budget=1920,
                 front=['PHALANX', 'IMMORTAL', 'KNIGHT', 'PALADIN', 'BERSERKER', 'BEASTMASTER'],
                 support=['CROSSBOWMAN', 'MARKSMAN', 'ARCHMAGE+FIRE1+FIRE2', 'ARCHMAGE', 'HIGH_PRIEST_OF_THE_ORDER', 'ARQUEBUS', 'CANNON', 'MAGE+FIRE1+FIRE2'],
                 pure_only=['CHAMPION', 'WAR_ELEPHANT'],
                 triples=[],
                 half=[]),
}


def squads(level):
    L = LEVELS[level]
    B = L['budget']
    out = {}

    def add(template):
        n = fill(template, B)
        out.setdefault(name_of(n), spec_of(n))

    for s in L['front'] + L['support'] + L['pure_only']:
        if CAP[base(s)] in (-1,) or CAP[base(s)] * COST[base(s)] >= B:
            add([(s, 1.0)])
    if level == 'late':
        # National classes: 4 each. Two front classes; a front and a support class; filler.
        for a, b in itertools.combinations(L['front'], 2):
            add([(a, 0.5), (b, 0.5)])
        for a in ['PHALANX', 'IMMORTAL', 'KNIGHT', 'BEASTMASTER', 'PALADIN']:
            for s in L['support']:
                add([(a, 0.5), (s, 0.5)])
        for a in ['PHALANX', 'KNIGHT', 'BEASTMASTER']:
            add([(a, 0.5), ('CHAMPION', 0.5)])
            add([(a, 0.5), ('WAR_ELEPHANT', 0.5)])
        add([('PHALANX', 0.25), ('KNIGHT', 0.25), ('CROSSBOWMAN', 0.25), ('MARKSMAN', 0.25)])
        add([('PHALANX', 0.25), ('KNIGHT', 0.25), ('MARKSMAN', 0.25), ('ARCHMAGE+FIRE1+FIRE2', 0.25)])
        add([('BEASTMASTER', 0.25), ('KNIGHT', 0.25), ('MARKSMAN', 0.25), ('CROSSBOWMAN', 0.25)])
        add([('PHALANX', 0.25), ('IMMORTAL', 0.25), ('MARKSMAN', 0.25), ('CROSSBOWMAN', 0.25)])
        return out
    for a, b in itertools.combinations(L['front'], 2):
        add([(a, 0.5), (b, 0.5)])
    for a in L['front']:
        for s in L['support']:
            add([(a, 0.75), (s, 0.25)])
            if a in L['half'] and not COMPACT:
                add([(a, 0.5), (s, 0.5)])
    for a, b, c in L['triples']:
        add([(a, 0.5), (b, 0.25), (c, 0.25)])
    if level == 'early':
        add([('WARRIOR', 0.75), ('ARCHER', 0.25)])
        add([('WARRIOR', 0.5), ('AXEMAN', 0.5)])
    return out


COMPACT = '--compact' in sys.argv
if COMPACT:
    sys.argv.remove('--compact')

GROUNDS = {'open': 'open', 'forest': 'forest', 'hills': 'hills', 'walls': 'walls fort:5 bombard:3'}


def specs(levels, grounds, only=None):
    """Round-robin lines `LEVEL:GROUND:A>B | A | B | ground` (every ordered pair, mirrors too)."""
    lines = []
    for lvl in levels:
        sq = squads(lvl)
        names = [n for n in sq if not only or n in only.get(lvl, [])]
        for g in grounds:
            for a, d in itertools.product(names, repeat=2):
                lines.append(f"{lvl}:{g}:{a}>{d} | {sq[a]} | {sq[d]} | {GROUNDS[g]}")
    return lines


SPHERES1 = ['AIR', 'BODY', 'CHAOS', 'DEATH', 'EARTH', 'ENCHANTMENT', 'ENTROPY', 'FIRE', 'ICE', 'LAW', 'LIFE',
            'METAMAGIC', 'MIND', 'NATURE', 'SHADOW', 'SPIRIT', 'SUN', 'WATER']
SPHERES2 = ['AIR', 'DEATH', 'EARTH', 'FIRE', 'ICE', 'LIFE', 'NATURE', 'SUN', 'WATER']


def caster_squads(level):
    """The marginal lab's squads: a core and two of X, by X's name."""
    if level == 'early':
        core = '4*AXEMAN, 2*ARCHER'
        xs = {s.lower(): f'ADEPT+{s}1' for s in SPHERES1}
        xs.update(bolt='ADEPT', axe='AXEMAN', arch='ARCHER', disc='DISCIPLE_THE_ORDER')
    else:
        core = '4*CHAMPION, 2*LONGBOWMAN'
        xs = {f'{s.lower()}2': f'MAGE+{s}1+{s}2' for s in SPHERES2}
        xs.update(bodyenchantment='MAGE+BODY1+ENCHANTMENT1', enchantmentfire='MAGE+ENCHANTMENT1+FIRE1',
                  lawentropy='MAGE+LAW1+ENTROPY1', bolt='MAGE', priest='PRIEST_OF_THE_ORDER', lb='LONGBOWMAN',
                  champ='CHAMPION')
    return {k: f'{core}, 2*{v}' for k, v in xs.items()}


def caster_specs(grounds):
    lines = []
    for lvl in ['early', 'mid']:
        sq = caster_squads(lvl)
        for g in grounds:
            for a, d in itertools.product(sq, repeat=2):
                lines.append(f"{lvl}:{g}:{a}>{d} | {sq[a]} | {sq[d]} | {GROUNDS[g]}")
    return lines


def write_chunks(out, lines, chunks):
    random.Random(1).shuffle(lines)  # even chunks
    os.makedirs(out, exist_ok=True)
    for i in range(chunks):
        with open(os.path.join(out, f"part{i}.txt"), "w") as f:
            f.write("\n".join(lines[i::chunks]) + "\n")
    print(f"{len(lines)} matchups in {chunks} files under {out}")


if __name__ == '__main__':
    if len(sys.argv) >= 2 and sys.argv[1] == 'specs':
        # specs OUT LEVELS GROUNDS [ONLY.json] [--chunks N]
        out, levels, grounds = sys.argv[2], sys.argv[3].split(','), sys.argv[4].split(',')
        rest = sys.argv[5:]
        chunks = 4
        if '--chunks' in rest:
            chunks = int(rest[rest.index('--chunks') + 1])
            rest = rest[:rest.index('--chunks')]
        only = json.load(open(rest[0])) if rest else None
        write_chunks(out, specs(levels, grounds, only), chunks)
    elif len(sys.argv) >= 2 and sys.argv[1] == 'casters':
        # casters OUT [GROUNDS] [--chunks N]
        rest = sys.argv[2:]
        chunks = 4
        if '--chunks' in rest:
            chunks = int(rest[rest.index('--chunks') + 1])
            rest = rest[:rest.index('--chunks')]
        grounds = rest[1].split(',') if len(rest) > 1 else ['open', 'forest']
        write_chunks(rest[0], caster_specs(grounds), chunks)
    else:
        for k, v in squads(sys.argv[1]).items():
            print(f"{k}|{v}")
