#!/usr/bin/env python3
"""Fact sheets for the entity inventory (docs/ENTITIES.md): one block per unit, building, spell
and project, from the vendored XML (data/xml), the rules' help texts (the `data` query of
`ffh-play agent`) and every place in crates/ffh-ai and crates/ffh-core that names the type.

The AI lines re-implement the AI's filters: production::role_ok (the production roles),
units::role_of (the field role), the fields production::generic_building_value reads,
magic::understood and casting::ffh_value's terms (spells), production::best_project (projects).
When those functions change, change them here too. Regenerate the sheets at two commits (each in
its own checkout, with that commit's ffh-play) and diff them: the entities whose sheet changed are
the entries of docs/entities/ to revisit.

Usage:
  B=$(tools/agents/build-rev.sh HEAD)
  tools/entity_facts.py --bin $B/ffh-play --out DIR      (or --data-json FILE: a saved `data` reply)

Writes DIR/{units,buildings,spells,projects}.md, DIR/slices/*.md (one per writing slice, by the
docs' sections) and DIR/facts.json.
"""
import json
import os
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from collections import defaultdict


def _opt(name):
    if name in sys.argv:
        i = sys.argv.index(name)
        if i + 1 < len(sys.argv):
            return sys.argv[i + 1]
    return None


REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = _opt("--out")
if not OUT or not (_opt("--bin") or _opt("--data-json")):
    sys.exit(__doc__)

X = os.path.join(REPO, "data/xml")


def strip(t):
    return t.split("}")[-1]


def items(path, tag):
    r = ET.parse(os.path.join(X, path)).getroot()
    return [e for e in r.iter() if strip(e.tag) == tag]


def txt(e, tag, default=""):
    for c in e:
        if strip(c.tag) == tag:
            return (c.text or "").strip()
    return default


def num(e, tag, default=0):
    t = txt(e, tag, "")
    try:
        return int(float(t)) if t != "" else default
    except ValueError:
        return default


def child(e, tag):
    for c in e:
        if strip(c.tag) == tag:
            return c
    return None


def pairs(e, tag):
    """A list container: [(first child text, second child text or '1')]."""
    c = child(e, tag)
    out = []
    if c is None:
        return out
    for it in c:
        sub = list(it)
        if not sub:
            t = (it.text or "").strip()
            if t:
                out.append((t, "1"))
            continue
        a = (sub[0].text or "").strip()
        b = (sub[1].text or "").strip() if len(sub) > 1 else "1"
        out.append((a, b))
    return out


def on_list(e, tag):
    return [a for a, b in pairs(e, tag) if b not in ("0", "", "NONE")]


def int_list(e, tag):
    """Yields/commerces: <YieldChanges><iYield>1</iYield>..."""
    c = child(e, tag)
    if c is None:
        return []
    return [int((x.text or "0").strip() or 0) for x in c]


if _opt("--data-json"):
    data = json.load(open(_opt("--data-json"), encoding="utf-8"))
else:
    res = subprocess.run([_opt("--bin"), "agent", "--no-game"], input='{"k":"q","q":"data"}\n', capture_output=True, text=True, check=True)
    # Split on newlines only: the texts hold characters str.splitlines() also splits on.
    data = json.loads(next(l for l in res.stdout.split("\n") if l.startswith('{"id"')))["r"]
T = data["tables"]
NAME = {}
for k, rows in T.items():
    for r in rows:
        if isinstance(r, dict) and "type" in r:
            NAME[r["type"]] = r.get("name") or r["type"]
ROW = {}
for k in ["UnitType", "BuildingType", "SpellType", "ProjectType"]:
    for r in T[k]:
        ROW[r["type"]] = r


def nm(t):
    return NAME.get(t, t)


ICON = {
    "STRENGTH": " str", "MOVES": " mv", "FOOD": " food", "PRODUCTION": " hammers", "COMMERCE": " commerce",
    "RESEARCH": " research", "GOLD": " gold", "CULTURE": " culture", "ESPIONAGE": " espionage", "HAPPY": " happy",
    "UNHAPPY": " unhappy", "HEALTHY": " health", "UNHEALTHY": " unhealthy", "GREATPEOPLE": " GPP", "TRADE": " trade",
    "DEFENSE": " defense", "RELIGION": " religion", "POWER": " power", "ANGRY_POP": " angry", "OCCUPATION": " occupation",
    "BULLET": "- ", "MANA": " mana", "STAR": "*",
}


def plain(s):
    if not s:
        return ""
    s = s.replace("[NEWLINE]", "\n").replace("[SPACE]", " ").replace("[TAB]", "").replace("[PARAGRAPH:1]", "\n")
    s = re.sub(r"\[ICON_([A-Z_]+)\]", lambda m: ICON.get(m.group(1), " " + m.group(1).lower()), s)
    s = re.sub(r"\[COLOR_[A-Z_]+\]|\[COLOR_REVERT\]|\[LINK=[^\]]*\]|\[\\LINK\]|\[/?[A-Z_]+\]", "", s)
    s = re.sub(r"[ \t]+", " ", s)
    return "\n".join(l.strip() for l in s.split("\n") if l.strip())


# ---------------------------------------------------------------------------------------------
# Code references: every quoted type name in ffh-ai and ffh-core (ffh-play's help excluded).
# ---------------------------------------------------------------------------------------------
REFS = defaultdict(list)
TYPE_RE = re.compile(r'"((?:UNIT|UNITCLASS|BUILDING|BUILDINGCLASS|SPELL|PROJECT|PROMOTION|CIVILIZATION|IMPROVEMENT|BONUS)_[A-Z0-9_]+)"')


def scan(root, label):
    for dp, dn, fn in os.walk(root):
        for f in sorted(fn):
            if not f.endswith(".rs"):
                continue
            p = os.path.join(dp, f)
            rel = os.path.relpath(p, REPO)
            in_tests = False
            for i, line in enumerate(open(p, encoding="utf-8"), 1):
                if "#[cfg(test)]" in line:
                    in_tests = True
                for m in TYPE_RE.finditer(line):
                    REFS[m.group(1)].append((label, rel, i, in_tests))


scan(os.path.join(REPO, "crates/ffh-ai/src"), "ai")
scan(os.path.join(REPO, "crates/ffh-core/src"), "core")


def refs_of(t, label):
    seen = []
    for lab, rel, i, tests in REFS.get(t, []):
        if lab != label or tests:
            continue
        seen.append(f"{rel}:{i}")
    return seen


# Python dispatch: "pyName" => rust_fn, and where each fn is defined.
PYFN = {}
FNLOC = {}
for f in ["spells.rs", "spells_a.rs", "spells_b.rs", "events.rs", "custom.rs", "dispatch.rs", "game_utils.rs"]:
    p = os.path.join(REPO, "crates/ffh-core/src/ffh", f)
    if not os.path.exists(p):
        continue
    for i, line in enumerate(open(p, encoding="utf-8"), 1):
        for m in re.finditer(r'"([A-Za-z0-9_]+)"\s*=>\s*([a-z0-9_:]+)', line):
            PYFN.setdefault(m.group(1), m.group(2).split("::")[-1])
        m = re.match(r"\s*(?:pub(?:\(crate\))? )?fn ([a-z0-9_]+)\(", line)
        if m:
            FNLOC.setdefault(m.group(1), f"crates/ffh-core/src/ffh/{f}:{i}")


def py_loc(expr):
    if not expr:
        return ""
    name = expr.split("(")[0].strip()
    fn = PYFN.get(name)
    loc = FNLOC.get(fn, "") if fn else ""
    return f"`{expr}` -> {loc or '(no Rust function found)'}"


# The rules' AI-only spell gates (docs/AI_ONLY_RULES.md, section A2 table).
AI_ONLY = {}
for line in open(os.path.join(REPO, "docs/AI_ONLY_RULES.md"), encoding="utf-8"):
    m = re.match(r"\| ([^|]+) \| (.+) \|$", line.strip())
    if m and not m.group(1).startswith("spell") and not m.group(1).startswith("---"):
        for n in m.group(1).split("/"):
            AI_ONLY[n.strip().lower()] = m.group(2).strip()

# ---------------------------------------------------------------------------------------------
# Tables
# ---------------------------------------------------------------------------------------------
units = {txt(e, "Type"): e for e in items("Units/CIV4UnitInfos.xml", "UnitInfo")}
uclasses = {txt(e, "Type"): e for e in items("Units/CIV4UnitClassInfos.xml", "UnitClassInfo")}
buildings = {txt(e, "Type"): e for e in items("Buildings/CIV4BuildingInfos.xml", "BuildingInfo")}
bclasses = {txt(e, "Type"): e for e in items("Buildings/CIV4BuildingClassInfos.xml", "BuildingClassInfo")}
spells = {txt(e, "Type"): e for e in items("Units/CIV4SpellInfos.xml", "SpellInfo")}
projects = {txt(e, "Type"): e for e in items("Gameinfo/CIV4ProjectInfo.xml", "ProjectInfo")}
promos = {txt(e, "Type"): e for e in items("Units/CIV4PromotionInfos.xml", "PromotionInfo")}
civs = {txt(e, "Type"): e for e in items("Civilizations/CIV4CivilizationInfos.xml", "CivilizationInfo")}
leaders = {txt(e, "Type"): e for e in items("Civilizations/CIV4LeaderHeadInfos.xml", "LeaderHeadInfo")}
techs = {txt(e, "Type"): e for e in items("Technologies/CIV4TechInfos.xml", "TechInfo")}
improvements = {txt(e, "Type"): e for e in items("Terrain/CIV4ImprovementInfos.xml", "ImprovementInfo")}

# The 19 playable civilizations and the two that join mid-game (Infernal, Mercurians).
PLAYABLE = [c for c in civs if c not in ("CIVILIZATION_RANDOM", "CIVILIZATION_BARBARIAN")]

# Civilization overrides.
CIV_UNIT = {}  # (civ, class) -> unit or NONE
CIV_BUILDING = {}
for c, e in civs.items():
    for a, b in pairs(e, "Units"):
        CIV_UNIT[(c, a)] = b
    for a, b in pairs(e, "Buildings"):
        CIV_BUILDING[(c, a)] = b


def civ_unit(c, cls):
    if (c, cls) in CIV_UNIT:
        return CIV_UNIT[(c, cls)]
    return txt(uclasses[cls], "DefaultUnit") if cls in uclasses else "NONE"


def civ_building(c, cls):
    if (c, cls) in CIV_BUILDING:
        return CIV_BUILDING[(c, cls)]
    return txt(bclasses[cls], "DefaultBuilding") if cls in bclasses else "NONE"


def short_civ(c):
    return nm(c)


def who_trains_unit(u):
    cls = txt(units[u], "Class")
    prereq_civ = txt(units[u], "PrereqCiv")
    who = [c for c in PLAYABLE if civ_unit(c, cls) == u and (not prereq_civ or prereq_civ == "NONE" or prereq_civ == c)]
    return who


def who_builds(b):
    cls = txt(buildings[b], "BuildingClass")
    prereq_civ = txt(buildings[b], "PrereqCiv")
    who = [c for c in PLAYABLE if civ_building(c, cls) == b and (not prereq_civ or prereq_civ == "NONE" or prereq_civ == c)]
    return who


def civ_phrase(who, total, barb=False):
    tail = " (and the barbarians)" if barb else ""
    return civ_phrase0(who, total) + tail


def civ_phrase0(who, total):
    if len(who) == 0:
        return "no civilization"
    if len(who) == total:
        return "every civilization"
    if len(who) >= total - 6:
        missing = [short_civ(c) for c in PLAYABLE if c not in who]
        return "every civilization except " + ", ".join(missing)
    return ", ".join(short_civ(c) for c in who)


def class_limits(e):
    g, t, p = num(e, "iMaxGlobalInstances", -1), num(e, "iMaxTeamInstances", -1), num(e, "iMaxPlayerInstances", -1)
    out = []
    if g > 0:
        out.append(f"world limit {g}")
    if t > 0:
        out.append(f"team limit {t}")
    if p > 0:
        out.append(f"player limit {p}")
    return ", ".join(out)


# ---------------------------------------------------------------------------------------------
# Units: the AI's production roles (production::role_ok) and field role (units::role_of).
# ---------------------------------------------------------------------------------------------
def u_ai(e):
    return set(on_list(e, "UnitAIs"))


def u_free(e):
    return set(on_list(e, "FreePromotions"))


def is_hero(e):
    return txt(e, "DefaultUnitAI") == "UNITAI_HERO" or "PROMOTION_HERO" in u_free(e)


def born_caster(e):
    return "PROMOTION_CHANNELING1" in u_free(e)


def siege_engine(e):
    return txt(e, "DefaultUnitAI") in ("UNITAI_ATTACK_CITY", "UNITAI_COLLATERAL") and (num(e, "iBombardRate") > 0 or num(e, "iCollateralDamage") > 0)


def roles(e):
    has = u_ai(e)
    combat = num(e, "iCombat")
    found = num(e, "bFound")
    dom = txt(e, "Domain")
    da = txt(e, "DefaultUnitAI")
    fighter = combat > 0 and not found and not is_hero(e) and da != "UNITAI_MAGE"
    out = []
    land = dom == "DOMAIN_LAND"
    sea = dom == "DOMAIN_SEA"
    if land:
        if fighter and (has & {"UNITAI_CITY_DEFENSE", "UNITAI_CITY_COUNTER", "UNITAI_ATTACK", "UNITAI_COUNTER"}) and da != "UNITAI_EXPLORE":
            out.append("Defense")
        if fighter and not num(e, "bOnlyDefensive") and (has & {"UNITAI_ATTACK", "UNITAI_ATTACK_CITY"}) and not siege_engine(e):
            out.append("Attack/CityAttack")
        if "UNITAI_WORKER" in has and combat == 0:
            out.append("Worker")
        if "UNITAI_SETTLE" in has and found:
            out.append("Settler")
        if is_hero(e) and (combat > 0 or born_caster(e)):
            out.append("Hero")
        if da == "UNITAI_MAGE" or "UNITAI_WARWIZARD" in has:
            out.append("Mage")
        if "UNITAI_EXPLORE" in has:
            out.append("Explore")
        if fighter and (has & {"UNITAI_ATTACK_CITY", "UNITAI_COLLATERAL"}) and (num(e, "iBombardRate") > 0 or num(e, "iCollateralDamage") > 0):
            out.append("Siege")
        if "UNITAI_MISSIONARY" in has and pairs(e, "ReligionSpreads"):
            out.append("Missionary")
    if sea:
        if "UNITAI_WORKER_SEA" in has and combat == 0:
            out.append("SeaWorker")
        if num(e, "iCargo") > 0 and txt(e, "DomainCargo") == "DOMAIN_LAND" and txt(e, "SpecialCargo") in ("", "NONE"):
            out.append("Transport")
        if combat > 0 and (has & {"UNITAI_EXPLORE_SEA", "UNITAI_ATTACK_SEA"}):
            out.append("SeaExplore")
        if is_hero(e) and combat > 0:
            out.append("SeaHero")
    return out


def field_role(e):
    da = txt(e, "DefaultUnitAI")
    dom = txt(e, "Domain")
    combat = num(e, "iCombat")
    if dom == "DOMAIN_SEA":
        if "UNITAI_WORKER_SEA" in u_ai(e) and combat == 0:
            return "SeaWorker (workers.rs)"
        if num(e, "iCargo") > 0 and txt(e, "DomainCargo") == "DOMAIN_LAND" and txt(e, "SpecialCargo") in ("", "NONE"):
            return "Transport (naval.rs: settlers overseas, amphibious assault)"
        return "Naval: holds (no naval combat logic; a sea explorer if trained as one)"
    if da == "UNITAI_HERO" and (num(e, "bOnlyDefensive") or combat == 0):
        return "Caster (a hero that cannot attack moves as a caster)"
    m = {
        "UNITAI_SETTLE": "Settler (expansion.rs)",
        "UNITAI_WORKER": "Worker (workers.rs)",
        "UNITAI_CITY_DEFENSE": "Garrison", "UNITAI_CITY_COUNTER": "Garrison", "UNITAI_CITY_SPECIAL": "Garrison",
        "UNITAI_EXPLORE": "Explorer",
        "UNITAI_MISSIONARY": "Missionary",
        "UNITAI_MAGE": "Caster (magic.rs)", "UNITAI_WARWIZARD": "Caster (magic.rs)", "UNITAI_TERRAFORMER": "Caster (magic.rs)",
        "UNITAI_MANA_UPGRADE": "Caster (magic.rs)", "UNITAI_MEDIC": "Caster (magic.rs)",
        "UNITAI_ANIMAL": "Animal (attacks what it can beat, else roams)",
    }
    for gp in ["PROPHET", "ARTIST", "SCIENTIST", "GENERAL", "MERCHANT", "ENGINEER"]:
        m["UNITAI_" + gp] = "Great person (shrine / wonder rush / lightbulb / join / golden age)"
    if da in m:
        return m[da]
    if num(e, "bFound"):
        return "Settler"
    if combat > 0 or num(e, "iCombatDefense") > 0:
        return "Army (military.rs, campaign.rs)" + (" -- or Garrison when production trained it as a defender (UNITAI_CITY_DEFENSE)" if "Defense" in roles(e) else "")
    if on_list(e, "Builds"):
        return "Worker"
    return "Other: holds"


# Spells by unit / class / free promotions.
def spell_prereqs(s):
    e = spells[s]
    return {
        "p1": txt(e, "PromotionPrereq1"), "p2": txt(e, "PromotionPrereq2"), "unit": txt(e, "UnitPrereq"),
        "cls": txt(e, "UnitClassPrereq"), "combat": txt(e, "UnitCombatPrereq"), "civ": txt(e, "CivilizationPrereq"),
        "rel": txt(e, "ReligionPrereq"), "staterel": txt(e, "StateReligionPrereq"), "bld": txt(e, "BuildingPrereq"),
        "tech": txt(e, "TechPrereq"), "bclass_owned": txt(e, "BuildingClassOwnedPrereq"),
    }


def spells_of_unit(u):
    e = units[u]
    cls = txt(e, "Class")
    free = u_free(e)
    out = []
    for s in spells:
        p = spell_prereqs(s)
        direct = p["unit"] == u or p["cls"] == cls
        if p["unit"] not in ("", "NONE") and p["unit"] != u:
            continue
        if p["cls"] not in ("", "NONE") and p["cls"] != cls:
            continue
        if p["combat"] not in ("", "NONE") and p["combat"] != txt(e, "Combat"):
            continue
        # A spell's ReligionPrereq is the caster's own religion (CvUnit::canCast).
        if p["rel"] not in ("", "NONE") and p["rel"] != txt(e, "ReligionType"):
            continue
        if txt(spells[s], "CivilizationPrereq") not in ("", "NONE") and txt(e, "PrereqCiv") not in ("", "NONE") and txt(spells[s], "CivilizationPrereq") != txt(e, "PrereqCiv"):
            continue
        need = [x for x in (p["p1"], p["p2"]) if x not in ("", "NONE")]
        if direct and all(x in free for x in need):
            out.append(s)
        elif need and all(x in free for x in need) and (direct or p["unit"] in ("", "NONE")):
            out.append(s)
    return out


SUMMONED_BY = defaultdict(list)
CONVERTED_BY = defaultdict(list)
BUILDING_BY_SPELL = defaultdict(list)
for s, e in spells.items():
    cu = txt(e, "CreateUnitType")
    if cu not in ("", "NONE"):
        SUMMONED_BY[cu].append(s + (" (permanent)" if num(e, "bPermanentUnitCreate") else " (temporary)"))
    cv = txt(e, "ConvertUnitType")
    if cv not in ("", "NONE"):
        CONVERTED_BY[cv].append(s)
    cb = txt(e, "CreateBuildingType")
    if cb not in ("", "NONE"):
        BUILDING_BY_SPELL[cb].append(s)

UPGRADES_FROM = defaultdict(list)
for u, e in units.items():
    for cls in on_list(e, "UnitClassUpgrades"):
        UPGRADES_FROM[cls].append(u)

ENABLES = defaultdict(list)  # building -> units it gates
for u, e in units.items():
    pb = txt(e, "PrereqBuilding")
    if pb not in ("", "NONE"):
        ENABLES[pb].append(u)
    pbc = txt(e, "PrereqBuildingClass")
    if pbc not in ("", "NONE"):
        ENABLES["CLASS:" + pbc].append(u)

LAIR_SPAWN = defaultdict(list)
for i, e in improvements.items():
    su = txt(e, "SpawnUnitType")
    if su not in ("", "NONE"):
        LAIR_SPAWN[su].append(i)

# Free promotions from buildings.
PROMO_FROM_BUILDING = defaultdict(list)
for b, e in buildings.items():
    fp = txt(e, "FreePromotion")
    if fp not in ("", "NONE"):
        PROMO_FROM_BUILDING[fp].append(b)

EVENTS_TEXT = open(os.path.join(X, "Events/CIV4EventInfos.xml"), encoding="utf-8").read() + open(os.path.join(X, "Events/CIV4EventTriggerInfos.xml"), encoding="utf-8").read()
GOODY_TEXT = open(os.path.join(X, "Gameinfo/CIV4GoodyInfo.xml"), encoding="utf-8").read()


def tech_list(e, one="PrereqTech", many="TechTypes"):
    out = []
    t = txt(e, one)
    if t not in ("", "NONE"):
        out.append(t)
    out += [x for x, _ in pairs(e, many) if x not in ("", "NONE")]
    return out


def unit_facts(u):
    e = units[u]
    cls = txt(e, "Class")
    ce = uclasses.get(cls)
    cost = num(e, "iCost")
    who = who_trains_unit(u)
    default_for = [c for c in PLAYABLE if civ_unit(c, cls) == u]
    replaces = ""
    dflt = txt(ce, "DefaultUnit") if ce is not None else ""
    if dflt and dflt != u and dflt != "NONE":
        replaces = f"replaces {nm(dflt)} (`{dflt}`) for its civilization(s)"
    lines = []
    lines.append(f"- class `{cls}`{' (' + class_limits(ce) + ')' if ce is not None and class_limits(ce) else ''}; combat type `{txt(e, 'Combat')}`; domain `{txt(e, 'Domain')}`; tier {num(e, 'iTier')}")
    s = f"- strength {num(e, 'iCombat')}/{num(e, 'iCombatDefense')} (attack/defense), moves {num(e, 'iMoves')}"
    dmg = pairs(e, "DamageTypeCombats")
    if dmg:
        s += ", extra damage " + ", ".join(f"{b} {a}" for a, b in dmg)
    for tag, label in [("iFirstStrikes", "first strikes"), ("iChanceFirstStrikes", "chance first strikes"), ("iWithdrawalProb", "withdraw %"), ("iCollateralDamage", "collateral"), ("iBombardRate", "bombard"), ("iCityAttack", "city attack %"), ("iCityDefense", "city defense %"), ("iCargo", "cargo"), ("iWorkRate", "work rate")]:
        if num(e, tag):
            s += f", {label} {num(e, tag)}"
    lines.append(s)
    if cost > 0:
        barb = civ_unit("CIVILIZATION_BARBARIAN", cls) == u
        lines.append(f"- cost {cost} hammers; trained by: {civ_phrase(who, len(PLAYABLE), barb)}{'; ' + replaces if replaces else ''}")
    else:
        lines.append(f"- not trainable (cost {cost}){'; ' + replaces if replaces else ''}")
    req = []
    tl = tech_list(e, "PrereqTech", "TechTypes")
    if tl:
        req.append("tech " + " + ".join(nm(t) for t in tl))
    for tag, lab in [("PrereqBuilding", "building"), ("PrereqBuildingClass", "building class"), ("BonusType", "resource"), ("PrereqReligion", "religion"), ("StateReligion", "state religion"), ("PrereqCiv", "civilization"), ("PrereqAlignment", "alignment"), ("PrereqCivic", "civic")]:
        v = txt(e, tag)
        if v not in ("", "NONE"):
            req.append(f"{lab} {nm(v)}")
    ob = [x for x, _ in pairs(e, "PrereqBonuses")]
    if ob:
        req.append("one of " + "/".join(nm(x) for x in ob))
    if num(e, "iMinLevel"):
        req.append(f"min level {num(e, 'iMinLevel')} (not trainable: reached by upgrade)")
    if req:
        lines.append("- requires: " + "; ".join(req))
    up = on_list(e, "UnitClassUpgrades")
    if up:
        lines.append("- upgrades to (class default; civilizations may replace it): " + ", ".join(nm(txt(uclasses[c], "DefaultUnit")) if c in uclasses else c for c in up))
    frm = UPGRADES_FROM.get(cls, [])
    if frm:
        lines.append("- upgraded from: " + ", ".join(nm(x) for x in frm))
    fp = sorted(u_free(e))
    if fp:
        lines.append("- free promotions: " + ", ".join(nm(x) for x in fp))
    sources = []
    if SUMMONED_BY.get(u):
        sources.append("summoned by " + ", ".join(nm(x.split(' ')[0]) + ' ' + x.split(' ', 1)[1] for x in SUMMONED_BY[u]))
    if CONVERTED_BY.get(u):
        sources.append("converted into by " + ", ".join(nm(x) for x in CONVERTED_BY[u]))
    if LAIR_SPAWN.get(u):
        sources.append("spawned by " + ", ".join(nm(x) for x in LAIR_SPAWN[u]))
    if f">{u}<" in EVENTS_TEXT:
        sources.append("events give it")
    if f">{u}<" in GOODY_TEXT:
        sources.append("goody huts give it")
    core = refs_of(u, "core")
    if core:
        sources.append("named in the rules (Python ports and C++): " + ", ".join(core[:8]) + (" ..." if len(core) > 8 else ""))
    if sources:
        lines.append("- other sources / rules references: " + "; ".join(sources))
    sp = spells_of_unit(u)
    if sp:
        def civnote(s):
            c = txt(spells[s], "CivilizationPrereq")
            return f" [{nm(c)} only]" if c not in ("", "NONE") else ""
        lines.append("- spells/abilities it has from birth: " + ", ".join(f"{nm(s)} (`{s}`){civnote(s)}" for s in sp))
    da = txt(e, "DefaultUnitAI")
    lines.append(f"- AI data: default AI `{da}`; UnitAIs {sorted(u_ai(e)) or '[]'}; iAIWeight {num(e, 'iAIWeight')}" + ("; bOnlyDefensive" if num(e, 'bOnlyDefensive') else "") + ("; animal" if num(e, 'bAnimal') else ""))
    r = roles(e)
    if cost <= 0:
        prod = "not trainable, so no production role"
    elif not who:
        prod = "no playable civ trains it"
    elif not r:
        prod = "NO production role passes (production never trains it except a civ-specific step naming it)"
    else:
        prod = "production roles it qualifies for: " + ", ".join(r)
    if num(e, "iMinLevel") > 0 and cost > 0:
        prod += " (but iMinLevel > 0: the AI cannot train it unless GAMEOPTION_AI_NO_MINIMUM_LEVEL and min level > 1)"
    lines.append("- AI production: " + prod)
    if SUMMONED_BY.get(u):
        sea = txt(e, "Domain") == "DOMAIN_SEA"
        lines.append("- AI field role: a summon is created on UNITAI_ATTACK (unit_cast_create_unit), so units::role_of makes it " + ("Naval: holds (a sea unit on that AI)" if sea else "Army (military.rs); temporary summons attack at once (casting::summons_attack, magic::summons_attack)"))
    else:
        lines.append("- AI field role (units::role_of on its default AI, for units the rules create; a unit production trains takes the step's AI type instead: Defense -> CITY_DEFENSE (Garrison), Attack/Siege -> ATTACK/ATTACK_CITY (Army), Explore -> EXPLORE, Mage -> MAGE, ...): " + field_role(e))
    ai = refs_of(u, "ai")
    if ai:
        lines.append("- named in ffh-ai: " + ", ".join(ai))
    if cls and refs_of(cls, "ai"):
        lines.append("- its class named in ffh-ai: " + ", ".join(refs_of(cls, "ai")))
    return lines


# ---------------------------------------------------------------------------------------------
# Buildings: the fields production::generic_building_value reads, and the rest.
# ---------------------------------------------------------------------------------------------
VALUED = {
    "YieldChanges": "yields", "YieldModifiers": "yield %", "GlobalYieldModifiers": "global yield %",
    "CommerceChanges": "commerce", "ObsoleteSafeCommerceChanges": "commerce", "CommerceModifiers": "commerce %",
    "GlobalCommerceModifiers": "global commerce %", "iHappiness": "happiness", "iAreaHappiness": "area happiness",
    "iGlobalHappiness": "global happiness", "BonusHappinessChanges": "happiness from resources",
    "iHealth": "health", "iAreaHealth": "area health", "iGlobalHealth": "global health", "BonusHealthChanges": "health from resources",
    "SpecialistCounts": "specialist slots", "iGreatPeopleRateChange": "great people points", "iGreatPeopleRateModifier": "great people %",
    "iFreeSpecialist": "free specialist", "iDefense": "city defense", "iExperience": "free experience",
    "UnitCombatFreeExperiences": "experience by unit combat", "DomainFreeExperiences": "experience by domain (land only)",
    "iMilitaryProductionModifier": "military production %", "iTradeRoutes": "trade routes", "iFoodKept": "food kept",
    "iWorkerSpeedModifier": "worker speed", "iMaintenanceModifier": "maintenance %", "iHealRateChange": "heal rate",
    "iAIWeight": "AI weight", "iStateReligionHappiness": "state religion happiness",
}
UNVALUED = {
    "FreeBonus": "free resource", "FreeBonus2": "free resource", "FreeBonus3": "free resource", "FreePromotion": "free promotion for units",
    "bApplyFreePromotionOnMove": "promotion to units passing by", "RemovePromotion": "removes a promotion", "FreeBuilding": "free building",
    "ReligionChanges": "spreads a religion", "GlobalReligionCommerce": "shrine commerce", "bGoldenAge": "golden age", "iFreeTechs": "free techs",
    "iGlobalExperience": "global experience", "iGlobalWarWearinessModifier": "global war weariness %", "iWarWearinessModifier": "war weariness %",
    "CommerceChangeDoubleTimes": "commerce doubling time", "CommerceHappinesses": "happiness from commerce",
    "StateReligionCommerces": "state religion commerce", "SpecialistYieldChanges": "specialist yields",
    "SpecialistCommerceChanges": "specialist commerce", "SpecialistExtraCommerces": "specialist extra commerce",
    "SeaPlotYieldChanges": "sea plot yields", "GlobalSeaPlotYieldChanges": "global sea plot yields",
    "BonusYieldModifiers": "yield % with resources", "BonusProductionModifiers": "production % with resources",
    "DomainProductionModifiers": "production % by domain", "iCrime": "crime", "iResistMagic": "magic resistance",
    "iGlobalResistModify": "global spell resistance", "iGlobalResistEnemyModify": "enemy spell resistance",
    "iModifyGlobalCounter": "Armageddon counter", "iPlotRadius": "city radius", "bNoUnhappiness": "no unhappiness",
    "bNoUnhealthyPopulation": "no unhealthy population", "bNoCivicAnger": "no civic anger", "bSeeInvisible": "sees invisible",
    "bHideUnits": "hides units", "bUnhappyProduction": "unhappy citizens produce", "iAllCityDefense": "defense in all cities",
    "iBombardDefense": "bombard defense", "iAirlift": "airlift", "iCoastalTradeRoutes": "coastal trade routes",
    "iTradeRouteModifier": "trade route %", "iGlobalSpaceProductionModifier": "space production %", "bForceTeamVoteEligible": "vote eligibility",
    "bEquipment": "equipment building", "iFreePromotionPick": "free promotion pick", "FreeSpecialistCounts": "free specialists by type",
    "iNumFreeBonuses": "free resource count", "bGovernmentCenter": "government center (no distance maintenance)",
    "HolyCity": "needs the holy city", "bCapital": "palace (moves the capital)", "iGreatGeneralRateModifier": "great general %",
    "iGlobalGreatPeopleRateModifier": "global great people %", "iAnarchyModifier": "anarchy %", "iGoldenAgeModifier": "golden age %",
    "iGlobalHurryModifier": "hurry cost %", "iGlobalFreeExperience": "global experience", "iAreaFreeSpecialist": "area free specialist",
    "iGlobalFreeSpecialist": "global free specialist", "iEnemyWarWearinessModifier": "enemy war weariness %",
    "iGlobalPopulationChange": "population in every city", "iGlobalTradeRoutes": "global trade routes", "iForeignTradeRouteModifier": "foreign trade route %",
    "RiverPlotYieldChanges": "river yields", "AreaYieldModifiers": "area yield %", "PowerYieldModifiers": "power yield %",
    "UnitCombatProductionModifiers": "production % by unit combat",
}
FAV = defaultdict(list)
BCLS_LIST = list(bclasses.keys())
BLD_LIST = list(buildings.keys())
for l, e in leaders.items():
    fw = txt(e, "FavoriteWonder")
    if fw not in ("", "NONE"):
        FAV[fw].append(nm(l) + " (+300)")
    for tag in ("iFavoriteEarlyWonder", "iFavoriteLateWonder"):
        v = txt(e, tag)
        if v in ("", "NONE"):
            continue
        if v in bclasses:
            cls = v
        elif v in buildings:
            idx = BLD_LIST.index(v)  # C++ quirk: the building's index read as a class index
            cls = BCLS_LIST[idx] if idx < len(BCLS_LIST) else None
        else:
            cls = None
        if cls:
            for b, be in buildings.items():
                if txt(be, "BuildingClass") == cls:
                    FAV[b].append(nm(l) + " (+400, class" + (" via the index quirk" if v not in bclasses else "") + ")")


def nonzero(e, tag):
    c = child(e, tag)
    if c is None:
        return False
    if len(list(c)):
        vals = []
        for it in c:
            sub = list(it)
            if sub:
                vals.append((sub[-1].text or "").strip())
            else:
                vals.append((it.text or "").strip())
        return any(v not in ("", "0", "NONE") for v in vals)
    t = (c.text or "").strip()
    if tag.startswith("i"):
        return t not in ("", "0")
    return t not in ("", "0", "NONE", "-1")


def show_tag(e, tag):
    c = child(e, tag)
    if c is None:
        return ""
    if len(list(c)):
        parts = []
        for i, it in enumerate(c):
            sub = list(it)
            if sub:
                a, b = (sub[0].text or "").strip(), (sub[-1].text or "").strip()
                if b not in ("0", "", "NONE"):
                    parts.append(f"{a}={b}" if a != b else a)
            else:
                v = (it.text or "").strip()
                if v not in ("0", ""):
                    parts.append(f"[{i}]={v}")
        return ", ".join(parts)
    return (c.text or "").strip()


def building_facts(b):
    e = buildings[b]
    cls = txt(e, "BuildingClass")
    ce = bclasses.get(cls)
    cost = num(e, "iCost")
    who = who_builds(b)
    lines = []
    kind = class_limits(ce) if ce is not None else ""
    lines.append(f"- class `{cls}`" + (f" ({kind})" if kind else "") + f"; cost {cost if cost > 0 else 'none (not buildable: ' + str(cost) + ')'}")
    dflt = txt(ce, "DefaultBuilding") if ce is not None else ""
    rep = f"; replaces {nm(dflt)} (`{dflt}`)" if dflt and dflt not in (b, "NONE") else ""
    barb = civ_building("CIVILIZATION_BARBARIAN", cls) == b
    lines.append(f"- built by: {civ_phrase(who, len(PLAYABLE), barb)}{rep}")
    req = []
    tl = tech_list(e, "PrereqTech", "TechTypes")
    if tl:
        req.append("tech " + " + ".join(nm(t) for t in tl))
    for tag, lab in [("PrereqReligion", "religion"), ("StateReligion", "state religion"), ("HolyCity", "holy city of"), ("PrereqCiv", "civilization"), ("PrereqTrait", "trait"), ("Bonus", "resource"), ("VictoryPrereq", "victory enabled"), ("ObsoleteTech", "obsolete with")]:
        v = txt(e, tag)
        if v not in ("", "NONE"):
            req.append(f"{lab} {nm(v)}")
    for tag, lab in [("PrereqBuildingClasses", "buildings"), ("BuildingClassNeededs", "buildings in the city"), ("PrereqBonuses", "one of the resources")]:
        v = show_tag(e, tag)
        if v:
            req.append(f"{lab}: {v}")
    for tag, lab in [("iCitiesPrereq", "cities"), ("iLevelPrereq", "unit level"), ("iMinAreaSize", "area size")]:
        if num(e, tag) > 0:
            req.append(f"{lab} {num(e, tag)}")
    if num(e, "bWater"):
        req.append("coastal")
    if num(e, "bRiver"):
        req.append("river")
    if req:
        lines.append("- requires: " + "; ".join(req))
    valued = [f"{VALUED[t]} ({show_tag(e, t)})" for t in VALUED if nonzero(e, t) and t != "DomainFreeExperiences"]
    unvalued = [f"{UNVALUED[t]} ({show_tag(e, t)})" for t in UNVALUED if nonzero(e, t)]
    if nonzero(e, "DomainFreeExperiences"):
        dom = [a for a, v in pairs(e, "DomainFreeExperiences") if v not in ("0", "")]
        if "DOMAIN_LAND" in dom:
            valued.append(f"experience by domain, land ({show_tag(e, 'DomainFreeExperiences')})")
        if any(x != "DOMAIN_LAND" for x in dom):
            unvalued.append(f"experience for other domains, not read ({show_tag(e, 'DomainFreeExperiences')})")
    cul = int_list(e, "ObsoleteSafeCommerceChanges")
    if len(cul) > 2 and cul[2] > 0:
        valued.append("(its flat culture is in ObsoleteSafeCommerceChanges: read at 4 a point; the new-city culture bonus, which reads CommerceChanges only, does not apply)")
    lines.append("- fields the generic building value reads: " + ("; ".join(valued) if valued else "NONE"))
    if unvalued:
        lines.append("- effects it ignores: " + "; ".join(unvalued))
    paths = []
    if num(e, "bCapital"):
        paths.append("production::building_allowed: never, unless the palace is lost")
    by_class = [x for x in ENABLES.get("CLASS:" + cls, []) if any(civ_building(c, cls) == b and civ_unit(c, txt(units[x], "Class")) == x for c in PLAYABLE)]
    gated = sorted(set(ENABLES.get(b, []) + by_class))
    if gated:
        parts = []
        for x in gated:
            r = roles(units[x])
            via = []
            if "Defense" in r or "Attack/CityAttack" in r:
                via.append("enabler_building (if a third better than the city's best defender/attacker)")
            if "Siege" in r:
                via.append("campaign::siege_building (when the siege operation wants engines)")
            uc = [c for c in ("CIVILIZATION_AMURITES", "CIVILIZATION_CLAN_OF_EMBERS", "CIVILIZATION_DOVIELLO", "CIVILIZATION_SVARTALFAR") if civ_unit(c, "UNITCLASS_ADEPT") == x]
            if "Mage" in r and uc:
                via.append("civs::unique_casters (" + ", ".join(nm(c) for c in uc) + ")")
            if x == "UNIT_GRIGORI_MEDIC":
                via.append("civs/grigori.rs (Infirmary for medics at war)")
            parts.append(f"{nm(x)} [{'; '.join(via) if via else 'no AI path builds it for this unit'}]")
        paths.append("gates units: " + ", ".join(parts))
    builders = [nm(u2) for u2, ue in units.items() if b in on_list(ue, "Buildings") or b in on_list(ue, "ForceBuildings")]
    if builders:
        paths.append("units that can construct it (MISSION_CONSTRUCT): " + ", ".join(builders) + " -- the AI's great people construct only shrines (holy city + GlobalReligionCommerce)")
    if BUILDING_BY_SPELL.get(b):
        paths.append("created by spell " + ", ".join(nm(x) for x in BUILDING_BY_SPELL[b]) + (" (bRequiresCaster: lost when no caster in the city)" if num(e, "bRequiresCaster") else ""))
    if FAV.get(b):
        paths.append("leader favourite: " + ", ".join(FAV[b]))
    if ce is not None and pairs(ce, "VictoryThresholds"):
        paths.append("victory building: never the AI's own goal (victory::plan_ours drops the Tower and the Altar, 3041631); a rival close to it is a war target")
    if txt(e, "GlobalReligionCommerce") not in ("", "NONE"):
        paths.append("a shrine: Great Prophets build it (units::great_person_build)")
    if paths:
        lines.append("- AI paths besides the generic value: " + "; ".join(paths))
    ai = refs_of(b, "ai")
    if ai:
        lines.append("- named in ffh-ai: " + ", ".join(ai))
    if cls and refs_of(cls, "ai"):
        lines.append("- its class named in ffh-ai: " + ", ".join(refs_of(cls, "ai")))
    core = refs_of(b, "core")
    if core:
        lines.append("- named in the rules (Python ports and C++): " + ", ".join(core[:10]) + (" ..." if len(core) > 10 else ""))
    return lines


# ---------------------------------------------------------------------------------------------
# Spells
# ---------------------------------------------------------------------------------------------
PY_VALUED = {"SPELL_CRUSH", "SPELL_PILLAR_OF_FIRE", "SPELL_DESTROY_UNDEAD", "SPELL_DOMINATION", "SPELL_HEAL", "SPELL_EARTHQUAKE", "SPELL_SPRING", "SPELL_SCORCH", "SPELL_VITALIZE", "SPELL_RECRUIT"}
WORLD_TIMING = {
    "SPELL_ARCANE_LACUNA": "at war with a civilization (expert moment; FfH: 7 typed mana nodes on the map)",
    "SPELL_ARDOR": "5+ great people born", "SPELL_DIVINE_RETRIBUTION": "at war with 2+ teams",
    "SPELL_MARCH_OF_THE_TREES": "at war with 2+ teams", "SPELL_SANCTUARY": "at war with 2+ teams",
    "SPELL_FOR_THE_HORDE": "barbarians have 60+ units", "SPELL_GIFTS_OF_NANTOSUELTA": "5+ cities", "SPELL_LEGENDS": "5+ cities",
    "SPELL_RALLY": "5+ cities", "SPELL_RIVER_OF_BLOOD": "5+ cities", "SPELL_STASIS": "5+ cities", "SPELL_INTO_THE_MIST": "40+ units",
    "SPELL_MOTHER_LODE": "10+ mines", "SPELL_RAGING_SEAS": "an enemy fleet worth more than 10",
    "SPELL_RELIGIOUS_FERVOR": "5 cities of the state religion, or 3 that are two thirds of ours (expert moment)",
    "SPELL_REVELRY": "total population 25+", "SPELL_VEIL_OF_NIGHT": "25+ units and at peace",
    "SPELL_WARCRY": "25+ units and at war", "SPELL_WILD_HUNT": "at war and 20+ units",
    "SPELL_WORLDBREAK": "Armageddon counter 20+ and at war with a civilization (expert moment; FfH: 50)",
}


def spell_facts(s):
    e = spells[s]
    p = spell_prereqs(s)
    lines = []
    who = []
    for k, lab in [("p1", "promotion"), ("p2", "promotion"), ("unit", "unit"), ("cls", "unit class"), ("combat", "unit combat"), ("civ", "civilization"), ("rel", "unit religion"), ("staterel", "state religion"), ("bld", "building in the city"), ("bclass_owned", "owns building"), ("tech", "tech")]:
        v = p[k]
        if v not in ("", "NONE"):
            who.append(f"{lab} {nm(v)}")
    for tag, lab in [("UnitInStackPrereq", "unit in stack"), ("PromotionInStackPrereq", "promotion in stack (caster lacks it)"), ("ImprovementPrereq", "improvement"), ("FeatureOrPrereq1", "feature"), ("FeatureOrPrereq2", "feature")]:
        v = txt(e, tag)
        if v not in ("", "NONE"):
            who.append(f"{lab} {nm(v)}")
    lines.append("- cast by / requires: " + ("; ".join(who) if who else "nothing fixed (Python requirement only)"))
    flags = []
    for tag in ["bGlobal", "bAbility", "bCausesWar", "bResistable", "bSacrificeCaster", "bInCityOnly", "bInBordersOnly", "bAdjacentToWaterOnly", "bBuffCasterOnly", "bImmuneTeam", "bImmuneNeutral", "bImmuneEnemy", "bImmuneNotAlive", "bPermanentUnitCreate", "bDispel", "bIgnoreHasCasted", "bCasterMustBeAlive", "bCasterNoDuration", "bPrereqSlaveTrade"]:
        if num(e, tag):
            flags.append(tag)
    for tag in ["iRange", "iDamage", "iDamageLimit", "iImmobileTurns", "iCost", "iChangePopulation", "iCreateUnitNum", "iCasterMinLevel", "iResistModify", "iDelay"]:
        if num(e, tag):
            flags.append(f"{tag} {num(e, tag)}")
    for tag in ["DamageType", "CreateUnitType", "CreateUnitPromotion", "ConvertUnitType", "CreateBuildingType", "CreateFeatureType", "CreateImprovementType", "AddPromotionType1", "AddPromotionType2", "AddPromotionType3", "RemovePromotionType1", "RemovePromotionType2", "RemovePromotionType3", "SpreadReligion"]:
        v = txt(e, tag)
        if v not in ("", "NONE"):
            flags.append(f"{tag} {nm(v)}")
    lines.append("- data: " + ", ".join(flags))
    pr, preq = txt(e, "PyResult"), txt(e, "PyRequirement")
    if pr:
        lines.append("- Python effect: " + py_loc(pr))
    if preq:
        lines.append("- Python requirement: " + py_loc(preq))
    allow = num(e, "bAllowAI")
    glob = num(e, "bGlobal")
    ab = num(e, "bAbility")
    understood = (not glob and not ab) and (num(e, "iDamage") != 0 or txt(e, "CreateUnitType") not in ("", "NONE") or txt(e, "AddPromotionType1") not in ("", "NONE") or txt(e, "RemovePromotionType1") not in ("", "NONE") or num(e, "iImmobileTurns") != 0 or num(e, "bDispel") or txt(e, "CreateBuildingType") not in ("", "NONE") or s in PY_VALUED)
    terms = []
    if txt(e, "CreateUnitType") not in ("", "NONE"):
        terms.append("summon (enemy within the summon's moves)")
    if num(e, "iDamage"):
        terms.append("damage on enemies in range")
    if any(txt(e, f"AddPromotionType{i}") not in ("", "NONE") for i in (1, 2, 3)):
        terms.append("promotions added (caster's promotion value)")
    if any(txt(e, f"RemovePromotionType{i}") not in ("", "NONE") for i in (1, 2, 3)):
        terms.append("promotions removed (minus)")
    if txt(e, "ConvertUnitType") not in ("", "NONE"):
        terms.append("unit conversion (new type's value - old)")
    if txt(e, "CreateBuildingType") not in ("", "NONE"):
        terms.append("building created (building value)")
    if txt(e, "CreateFeatureType") not in ("", "NONE"):
        terms.append("feature +10")
    if txt(e, "CreateImprovementType") not in ("", "NONE"):
        terms.append("improvement +10")
    if num(e, "bDispel"):
        terms.append("dispel")
    if num(e, "iChangePopulation"):
        terms.append("population x50")
    if num(e, "iCost"):
        terms.append("gold cost -4/gold")
    if num(e, "iImmobileTurns"):
        terms.append("immobility")
    if num(e, "bSacrificeCaster"):
        terms.append("caster sacrificed (minus level x worth)")
    if num(e, "bResistable"):
        terms.append("halved (resistable)")
    aiw = num(e, "iAIWeight")
    ai = []
    ai.append(f"bAllowAI {allow}" + (" -> the rules refuse every AI cast (unit_can_cast)" if not allow else ""))
    ai.append(f"iAIWeight {aiw}")
    if glob:
        ai.append("world spell: AI timing world_spells::ready = " + WORLD_TIMING.get(s, "none (always ready)"))
    if allow:
        if understood:
            ai.append("valued by magic.rs (effects on units / Python model" + (", Python modelled" if s in PY_VALUED else "") + ") before attacks and at turn end")
        else:
            ai.append("NOT modelled by magic.rs (world spell, ability or Python-only): FfH's chooseSpell terms: " + (", ".join(terms) if terms else "none") + f" + iAIWeight {aiw}")
            if not terms and aiw <= 0:
                ai.append("=> generic value 0: never cast by the generic choice")
    if s.lower() in AI_ONLY or nm(s).lower() in AI_ONLY:
        ai.append("rules' AI-only gate (docs/AI_ONLY_RULES.md A2): " + AI_ONLY.get(nm(s).lower(), AI_ONLY.get(s.lower(), "")))
    lines.append("- AI: " + "; ".join(ai))
    refs = refs_of(s, "ai")
    if refs:
        lines.append("- named in ffh-ai: " + ", ".join(refs))
    core = [r for r in refs_of(s, "core")]
    if core:
        lines.append("- named in the rules: " + ", ".join(core[:8]) + (" ..." if len(core) > 8 else ""))
    return lines


def project_facts(pj):
    e = projects[pj]
    lines = []
    lines.append(f"- cost {num(e, 'iCost')}; tech {nm(txt(e, 'TechPrereq'))}; civ {nm(txt(e, 'PrereqCivilization')) if txt(e, 'PrereqCivilization') not in ('', 'NONE') else 'any'}; limits: world {num(e, 'iMaxGlobalInstances', -1)}, team {num(e, 'iMaxTeamInstances', -1)}")
    other = []
    for c in e:
        tag = strip(c.tag)
        if tag in ("Type", "Description", "Civilopedia", "Strategy", "Button", "TechPrereq", "iCost", "iMaxGlobalInstances", "iMaxTeamInstances", "PrereqCivilization", "iAIWeight", "MovieDefineTag", "CreateSound", "MovieArtDef"):
            continue
        if nonzero(e, tag):
            other.append(f"{tag} {show_tag(e, tag)}")
    if other:
        lines.append("- data: " + "; ".join(other))
    aiw = num(e, "iAIWeight")
    excluded = pj in {"PROJECT_SAMHAIN", "PROJECT_THE_DRAW", "PROJECT_STIR_FROM_SLUMBER", "PROJECT_BANE_DIVINE", "PROJECT_GLORY_EVERLASTING", "PROJECT_PURGE_THE_UNFAITHFUL"}
    s = f"- AI: iAIWeight {aiw}; "
    if excluded:
        s += "excluded from production::best_project (a ritual left to dedicated logic)"
    elif aiw <= 0:
        s += "iAIWeight <= 0: best_project never picks it"
    else:
        s += "best_project picks it (weight per turn, <= 40 turns) only when no building has value left"
    lines.append(s)
    ai = refs_of(pj, "ai")
    if ai:
        lines.append("- named in ffh-ai: " + ", ".join(ai))
    core = refs_of(pj, "core")
    if core:
        lines.append("- named in the rules: " + ", ".join(core[:8]))
    return lines


def entry(t, facts):
    r = ROW.get(t, {})
    out = [f"### {nm(t)} (`{t}`)", ""]
    out += facts
    eff = plain(r.get("effects"))
    if eff:
        out.append("- help text (rules' help, ffh-play):")
        out += ["    " + l for l in eff.split("\n")]
    if r.get("help") and plain(r.get("help")) not in eff:
        out.append("- help: " + plain(r["help"]).replace("\n", " / "))
    if r.get("strategy"):
        out.append("- strategy (FfH text): " + plain(r["strategy"]).replace("\n", " "))
    out.append("")
    return "\n".join(out)


def unit_group(u):
    e = units[u]
    dom = txt(e, "Domain")
    da = txt(e, "DefaultUnitAI")
    combat = txt(e, "Combat")
    cls = txt(e, "Class")
    if num(e, "bAnimal"):
        return "animals"
    if num(e, "iCost") <= 0:
        return "summoned, spawned and special"
    if is_hero(e) or (cls in uclasses and class_limits(uclasses[cls])):
        return "heroes and limited units"
    if dom == "DOMAIN_SEA":
        return "naval"
    if da.startswith("UNITAI_") and da[7:] in ("PROPHET", "ARTIST", "SCIENTIST", "GENERAL", "MERCHANT", "ENGINEER"):
        return "great people"
    if combat in ("UNITCOMBAT_ADEPT",) or da == "UNITAI_MAGE":
        return "arcane"
    if combat == "UNITCOMBAT_DISCIPLE" or da == "UNITAI_MISSIONARY":
        return "disciples and religious"
    if combat in ("UNITCOMBAT_SIEGE",):
        return "siege"
    if combat in ("UNITCOMBAT_MOUNTED",):
        return "mounted"
    if combat in ("UNITCOMBAT_ARCHER",):
        return "archery"
    if combat in ("UNITCOMBAT_RECON",):
        return "recon"
    if combat in ("UNITCOMBAT_MELEE",):
        return "melee"
    return "civilian and other"


os.makedirs(OUT, exist_ok=True)
facts = {"units": {}, "buildings": {}, "spells": {}, "projects": {}}
with open(os.path.join(OUT, "units.md"), "w") as f:
    for u in units:
        fl = unit_facts(u)
        facts["units"][u] = {"name": nm(u), "group": unit_group(u), "facts": fl}
        f.write(f"<!-- group: {unit_group(u)} -->\n" + entry(u, fl) + "\n")
with open(os.path.join(OUT, "buildings.md"), "w") as f:
    for b in buildings:
        fl = building_facts(b)
        facts["buildings"][b] = {"name": nm(b), "facts": fl}
        f.write(entry(b, fl) + "\n")
with open(os.path.join(OUT, "spells.md"), "w") as f:
    for s in spells:
        fl = spell_facts(s)
        facts["spells"][s] = {"name": nm(s), "facts": fl}
        f.write(entry(s, fl) + "\n")
with open(os.path.join(OUT, "projects.md"), "w") as f:
    for pj in projects:
        fl = project_facts(pj)
        facts["projects"][pj] = {"name": nm(pj), "facts": fl}
        f.write(entry(pj, fl) + "\n")
json.dump(facts, open(os.path.join(OUT, "facts.json"), "w"), indent=1)
print({k: len(v) for k, v in facts.items()})

# ---------------------------------------------------------------------------------------------
# Document sections and writing slices.
# ---------------------------------------------------------------------------------------------
GP = {"UNITAI_" + x for x in ("PROPHET", "ARTIST", "SCIENTIST", "GENERAL", "MERCHANT", "ENGINEER")}
COMBAT_SECTION = {
    "UNITCOMBAT_MELEE": "Melee", "UNITCOMBAT_ARCHER": "Archery", "UNITCOMBAT_MOUNTED": "Mounted",
    "UNITCOMBAT_RECON": "Recon", "UNITCOMBAT_SIEGE": "Siege", "UNITCOMBAT_ADEPT": "Arcane",
    "UNITCOMBAT_DISCIPLE": "Disciples", "UNITCOMBAT_NAVAL": "Naval", "UNITCOMBAT_BEAST": "Beasts",
    "UNITCOMBAT_ANIMAL": "Animals", "NONE": "Settlers, workers and constructs",
}


def unit_section(u):
    e = units[u]
    if txt(e, "EquipmentPromotion") not in ("", "NONE"):
        return "Equipment"
    if txt(e, "DefaultUnitAI") in GP:
        return "Great people"
    if is_hero(e):
        return "Heroes"
    if num(e, "bAnimal") or txt(e, "Combat") == "UNITCOMBAT_ANIMAL" or txt(e, "DefaultUnitAI") == "UNITAI_ANIMAL":
        return "Animals"
    if num(e, "iCost") <= 0:
        return "Summons" if SUMMONED_BY.get(u) else "Barbarian, event and special units"
    if txt(e, "Domain") == "DOMAIN_SEA":
        return "Naval"
    return COMBAT_SECTION.get(txt(e, "Combat"), "Settlers, workers and constructs")


UNIT_SLICES = {
    "U1": ["Settlers, workers and constructs", "Melee", "Siege", "Beasts", "Great people", "Summons"],
    "U2": ["Archery", "Mounted", "Recon", "Naval"],
    "U3": ["Arcane", "Disciples", "Heroes"],
    "U4": ["Equipment", "Animals", "Barbarian, event and special units"],
}


def building_section(b):
    e = buildings[b]
    ce = bclasses.get(txt(e, "BuildingClass"))
    g = num(ce, "iMaxGlobalInstances", -1) if ce is not None else -1
    t = num(ce, "iMaxTeamInstances", -1) if ce is not None else -1
    p = num(ce, "iMaxPlayerInstances", -1) if ce is not None else -1
    if num(e, "iCost") <= 0:
        return "Buildings no city can build (palaces, granted and spell-made)"
    if g > 0:
        return "World wonders"
    if t > 0 or p > 0:
        return "National wonders and limited buildings"
    return "City buildings"


SPHERES = ("AIR", "BODY", "CHAOS", "DEATH", "EARTH", "ENCHANTMENT", "ENTROPY", "FIRE", "ICE", "LAW", "LIFE", "METAMAGIC", "MIND", "NATURE", "SHADOW", "SPIRIT", "SUN", "WATER", "CREATION", "FORCE", "DIMENSIONAL", "CHANNELING")


def spell_section(s):
    e = spells[s]
    if num(e, "bGlobal"):
        return "World spells"
    p1, p2 = txt(e, "PromotionPrereq1"), txt(e, "PromotionPrereq2")
    if any(re.fullmatch(r"PROMOTION_(%s)[123]" % "|".join(SPHERES), p or "") for p in (p1, p2)):
        return "Sphere spells (arcane)"
    if num(e, "bAbility"):
        return "Abilities"
    return "Unit, hero, religion and civilization spells"


SPELL_SLICES = {
    "S1": ["Sphere spells (arcane)"],
    "S2": ["World spells", "Unit, hero, religion and civilization spells"],
    "S3": ["Abilities:A-L"],
    "S4": ["Abilities:M-Z"],
}
BUILDING_SLICES = {
    "B1": ["City buildings", "Projects and rituals"],
    "B2": ["World wonders", "National wonders and limited buildings", "Buildings no city can build (palaces, granted and spell-made)"],
}

os.makedirs(os.path.join(OUT, "slices"), exist_ok=True)
ORDER = {}
for u in units:
    ORDER.setdefault(("unit", unit_section(u)), []).append(u)
for b in buildings:
    ORDER.setdefault(("building", building_section(b)), []).append(b)
for pj in projects:
    ORDER.setdefault(("building", "Projects and rituals"), []).append(pj)
for s in spells:
    ORDER.setdefault(("spell", spell_section(s)), []).append(s)


def facts_for(kind, t):
    if kind == "unit":
        return unit_facts(t)
    if kind == "spell":
        return spell_facts(t)
    if t in projects:
        return project_facts(t)
    return building_facts(t)


summary = {}
for slices, kind in ((UNIT_SLICES, "unit"), (BUILDING_SLICES, "building"), (SPELL_SLICES, "spell")):
    for name, sections in slices.items():
        n = 0
        with open(os.path.join(OUT, "slices", name + ".md"), "w") as f:
            for sec in sections:
                base, _, rng = sec.partition(":")
                ts = sorted(ORDER.get((kind, base), []), key=lambda t: nm(t).lower())
                if rng:
                    lo, hi = rng.split("-")
                    ts = [t for t in ts if lo.lower() <= nm(t)[0].lower() <= hi.lower()]
                f.write(f"## {base} ({len(ts)}{', ' + rng if rng else ''})\n\n")
                for t in ts:
                    f.write(entry(t, facts_for(kind, t)) + "\n")
                    n += 1
        summary[name] = {"sections": sections, "entries": n}
json.dump(summary, open(os.path.join(OUT, "slices", "summary.json"), "w"), indent=1)
print(json.dumps(summary, indent=1))
