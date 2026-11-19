#!/usr/bin/env python3
"""Validate the review concepts and rebuild the illustrated Markdown and local HTML catalog."""
import argparse
import html
import json
import re
import struct
from pathlib import Path

from art_assets import BATCH_SIZE

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def jsonc(path):
    text = path.read_text()
    return json.loads(re.sub(r"^\s*//.*$", "", text, flags=re.M))


def trait_name(trait):
    return trait.removeprefix("TRAIT_").replace("_", " ").title()


def validate(civs, require_images):
    source_civs = jsonc(ROOT / "data/json/civilizations.jsonc")
    expected = {c["type"] for c in source_civs} - {"CIVILIZATION_RANDOM", "CIVILIZATION_BARBARIAN"}
    assert len(civs) == 21 and {c["civilization"] for c in civs} == expected, "Expected the original 21 civilizations"
    known_traits = {t["type"] for t in jsonc(ROOT / "data/json/traits.jsonc")}
    inherited = {c["type"]: ([c["civ_trait"]] if c.get("civ_trait") else []) for c in source_civs}
    existing = jsonc(ROOT / "data/json/leader_heads.jsonc")
    for path in (ROOT / "data/json/cards").glob("*.jsonc"):
        if path.name not in {"zz_minor_leaders.jsonc", "zzz_minor_leaders.jsonc"}:
            existing += jsonc(path).get("leader_heads", [])
    existing_ids = {x["type"].removeprefix("LEADER_").lower() for x in existing}
    ids, names, palettes, activities = set(), set(), set(), set()
    for civ in civs:
        assert len(civ["leaders"]) == 2 * BATCH_SIZE, civ["name"]
        for leader in civ["leaders"]:
            lid, name = leader["id"], leader["name"]
            assert re.fullmatch(r"[a-z][a-z0-9_]*", lid), lid
            assert lid not in ids and lid not in existing_ids, f"Duplicate leader ID: {lid}"
            assert name not in names, f"Duplicate name: {name}"
            ids.add(lid)
            names.add(name)
            assert leader["alignment"] in {"Good", "Neutral", "Evil"}, name
            assert 1 <= len(leader["traits"]) <= 3, name
            assert len(set(leader["traits"])) == len(leader["traits"]), name
            assert set(leader["inherited_traits"]) == set(inherited[civ["civilization"]]), name
            assert not set(leader["traits"]) & set(leader["inherited_traits"]), name
            assert set(leader["traits"] + leader["inherited_traits"]) <= known_traits, name
            sentences = re.split(r"(?<=[.!?])\s+", leader["backstory"].strip())
            assert 3 <= len(sentences) <= 4, f"{name}: {len(sentences)} sentences"
            for field in ["epithet", "angle", "backstory", "portrait", "primary_palette", "hand_activity", "vibe", "clothing"]:
                assert leader[field].strip(), f"{name}: missing {field}"
            for field, values in [("primary_palette", palettes), ("hand_activity", activities)]:
                value = leader[field].strip().lower()
                assert value not in values, f"{name}: repeated {field}"
                values.add(value)
            portrait = HERE / "portraits" / f"{lid}.png"
            if require_images:
                raw = portrait.read_bytes()
                assert raw[:8] == b"\x89PNG\r\n\x1a\n", portrait
                w, h = struct.unpack(">II", raw[16:24])
                assert w == h and w >= 256, f"{portrait}: {w}x{h}"
    bals = next(c for c in civs if c["civilization"] == "CIVILIZATION_BALSERAPHS")
    assert sum("Perpentach" in x["name"] for x in bals["leaders"]) == 3, "Exactly three Perpentach versions required"
    demon = next(x for x in bals["leaders"] if x["id"] == "velisse_understudy")
    assert "demon" in demon["portrait"].lower() and "demon" in demon["backstory"].lower(), "Velisse must be a demon"
    return len(ids)


def render(civs):
    count = sum(len(civ["leaders"]) for civ in civs)
    intro = f"{count} minor leaders for the original {len(civs)} Fall from Heaven civilizations, with ten per civilization. Each has a portrait, three or four sentences of backstory, an alignment, and traits from the game. Three Balseraph leaders are rival facets of the same living Perpentach; another is a demon who escaped a summoned performance."
    status = "These leaders can enter play as Persons of Note or lead a breakaway faction. They take priority over major and legacy minor leaders when a faction forms. The game uses its seeded RNG to choose from the available preferred leaders."
    traits_note = "Personal traits are followed by inherited civilization traits. Alignment sets the faction's starting alignment; it does not remove civilization rules, races, unit restrictions, or inherited traits. Illian Agnostic and Infernal Barbarian, where listed, are personal traits matching their existing leaders. The Sundering retains its existing eligibility rules, including its Sheaim and Infernal exemptions; other city revolts and colonial releases use the same preferred leader pool."
    md = ["# Minor leaders", "", intro, "", status, "", "[Open the searchable illustrated catalog](catalog.html).", "", traits_note, "", "## Civilizations", ""]
    for c in civs:
        anchor = c["civilization"].removeprefix("CIVILIZATION_").lower()
        md.append(f"- [{c['name']}](#{anchor})")
    cards, nav = [], []
    esc = html.escape
    for c in civs:
        slug = c["civilization"].removeprefix("CIVILIZATION_").lower()
        md += ["", f'<a id="{slug}"></a>', "", f"## {c['name']}", ""]
        nav.append(f'<option value="{esc(c["name"])}">{esc(c["name"])}</option>')
        for l in c["leaders"]:
            personal = ", ".join(map(trait_name, l["traits"]))
            inherited = ", ".join(map(trait_name, l["inherited_traits"]))
            image = f"portraits/{l['id']}.png"
            trait_text = f"**{l['alignment']} · Personal traits:** {personal}."
            if inherited:
                trait_text += f" **Inherited:** {inherited}."
            art_text = f"**Portrait direction:** {l['vibe']}. Palette: {l['primary_palette']}. Activity: {l['hand_activity']}"
            md += [f"### {l['name']} — {l['epithet']}", "", f"![{l['name']} leaderhead portrait]({image})", "", trait_text, "", f"*{l['angle']}*", "", l["backstory"], "", art_text, ""]
            search = " ".join([c["name"], l["name"], l["epithet"], personal, inherited, l["alignment"], l["angle"], l["backstory"]])
            cards.append(f'''<article id="{l['id']}" data-civ="{esc(c['name'])}" data-alignment="{l['alignment']}" data-search="{esc(search.lower(), quote=True)}">
<img src="{image}" alt="{esc(l['name'])} leaderhead portrait" width="384" height="384" loading="lazy">
<div class="body"><div class="civ">{esc(c['name'])} <span class="alignment {l['alignment'].lower()}">{l['alignment']}</span></div>
<h2>{esc(l['name'])}</h2><p class="epithet">{esc(l['epithet'])}</p>
<p class="traits"><b>Personal traits:</b> {esc(personal)}{f'<br><b>Inherited:</b> {esc(inherited)}' if inherited else ''}</p>
<p class="angle">{esc(l['angle'])}</p><p>{esc(l['backstory'])}</p><details><summary>Portrait direction</summary><p><b>Vibe:</b> {esc(l['vibe'])}<br><b>Palette:</b> {esc(l['primary_palette'])}<br><b>Activity:</b> {esc(l['hand_activity'])}<br><b>Clothes:</b> {esc(l['clothing'])}</p></details></div></article>''')
    md += ["## Sources and assets", "", "Civilization identities and inherited traits: `data/json/civilizations.jsonc`. Existing leaders and personal traits: `data/json/leader_heads.jsonc` and `data/json/cards/doom_sundering.jsonc`. The runtime definitions and stories are in `data/json/cards/zz_minor_leaders.jsonc` and `data/json/cards/zzz_minor_leaders.jsonc`. Both come from `tools/sync-minor-leaders.py`. Trait definitions: `data/json/traits.jsonc`. Setting reference: `data/text/en.tsv`. Portrait style: `docs/ISO.md` and `tools/m7/prompts/assets/leader_*.txt`.", "", "Concept data and exact art briefs are in [concepts.json](concepts.json). Generated source sheets, prompts, and extraction metadata are in [art](art/). Each portrait is a separate PNG in [portraits](portraits/). The original generated sheets are preserved; export only splits their square cells and resizes them to the game's 384 pixel leaderhead size. The game exports also include 160 pixel face crops for diplomacy and other small portrait slots.", "", "Rebuild and validate this catalog with `python3 docs/minor-leaders/build_catalog.py`; check the runtime definitions with `python3 tools/sync-minor-leaders.py --check`.", ""]
    (HERE / "README.md").write_text("\n".join(md))
    page = '''<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Fall from Heaven minor leader concepts</title><style>
:root{color-scheme:dark;--bg:#101723;--panel:#1a2331;--gold:#e2bc70;--ink:#f3ecdd;--muted:#b8c2cf}*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--ink);font:16px/1.55 Georgia,serif}header,main{max-width:1440px;margin:auto;padding:24px}header{max-width:1050px;padding-top:48px}h1{font-size:clamp(30px,4vw,52px);line-height:1.12;margin:8px 0 24px;color:var(--gold)}header p{max-width:90ch}.label,.civ,select,input,button,.traits,.count{font-family:system-ui,sans-serif}.label{letter-spacing:.18em;text-transform:uppercase;font-size:12px;color:var(--muted)}.status{padding:14px 18px;border-left:3px solid var(--gold);background:var(--panel)}.notes{font-size:14px;color:var(--muted)}a{color:var(--gold)}.filters{display:flex;gap:10px;flex-wrap:wrap;position:sticky;top:0;background:#101723f5;padding:16px 0;z-index:2;align-items:center}label{display:grid;gap:4px;font:12px system-ui,sans-serif;color:var(--muted)}input,select,button{font-size:15px;background:var(--panel);color:var(--ink);border:1px solid #506075;border-radius:4px;padding:10px}input{width:270px}.count{margin-left:auto;font-size:13px;color:var(--muted)}.grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:24px}article{background:var(--panel);border:1px solid #364153;border-radius:6px;overflow:hidden;scroll-margin-top:130px}article[hidden]{display:none}article img{display:block;width:100%;height:auto;aspect-ratio:1;image-rendering:auto}.body{padding:20px}.civ{font-size:13px;color:var(--gold)}.alignment{float:right;font-size:11px;padding:2px 8px;border:1px solid;border-radius:20px}.good{color:#a9dbbd}.neutral{color:#d7caa7}.evil{color:#efadb4}h2{font-size:25px;line-height:1.2;margin:12px 0 3px}.epithet{margin:0 0 18px;color:var(--muted);font-style:italic}.traits{font-size:13px;line-height:1.6;border-top:1px solid #3a4554;border-bottom:1px solid #3a4554;padding:12px 0}.angle{font-style:italic;color:var(--gold)}.body p:last-child{margin-bottom:0}footer{padding:32px 0;color:var(--muted);font-size:14px}@media(max-width:1050px){.grid{grid-template-columns:repeat(2,minmax(0,1fr))}}@media(max-width:650px){header,main{padding:18px}.grid{grid-template-columns:1fr}.filters{position:static}input{width:100%}.count{margin-left:0}label:first-child{width:100%}}@media print{.filters{display:none}.grid{grid-template-columns:repeat(2,1fr)}article{break-inside:avoid}body{color:#111;background:white}.body,.status{background:white}.body p,.civ,h1,.angle{color:#111}}
</style></head><body><header><p class="label">Fall from Heaven · breakaway leaders</p><h1>Minor leaders</h1><p>INTRO</p><p class="status">STATUS</p><p class="notes">NOTES</p><p><a href="README.md">Full illustrated written list</a> · <a href="concepts.json">Concept data and portrait briefs</a></p></header><main><div class="filters"><label>Search names, traits, or stories<input id="search" type="search" placeholder="e.g. Perpentach, Arcane, sailors"></label><label>Civilization<select id="civ"><option value="">All CIV_COUNT civilizations</option>OPTIONS</select></label><label>Alignment<select id="alignment"><option value="">All alignments</option><option>Good</option><option>Neutral</option><option>Evil</option></select></label><span class="count" id="count" aria-live="polite">LEADER_COUNT leaders</span></div><div class="grid">CARDS</div><footer>Original concepts for the Rust Fall from Heaven repository. All portraits are generated concept art; personal traits use existing trait definitions. Preferred for breakaway factions; unavailable as starting leaders.</footer></main><script>
const cards=[...document.querySelectorAll('article')];const search=document.querySelector('#search');const civ=document.querySelector('#civ');const alignment=document.querySelector('#alignment');function filter(){let shown=0;const q=search.value.toLowerCase().trim();for(const c of cards){const yes=(!q||c.dataset.search.includes(q))&&(!civ.value||c.dataset.civ===civ.value)&&(!alignment.value||c.dataset.alignment===alignment.value);c.hidden=!yes;if(yes)shown++}document.querySelector('#count').textContent=`${shown} of ${cards.length} leaders`}for(const input of [search,civ,alignment])input.addEventListener('input',filter);
</script></body></html>'''
    for key, value in {"INTRO": esc(intro), "STATUS": esc(status), "NOTES": esc(traits_note), "CIV_COUNT": str(len(civs)), "LEADER_COUNT": str(count), "OPTIONS": "".join(nav), "CARDS": "\n".join(cards)}.items():
        page = page.replace(key, value)
    (HERE / "catalog.html").write_text(page)
    art_index = [
        "# Minor leader portrait sources",
        "",
        "Each civilization has two 1536 by 1024 source atlases. Each atlas contains five square portraits and an unused sixth cell. The individual leaderheads in `../portraits/` are square cells resized to 384 by 384 pixels.",
        "",
        "Both batches pair pixel-art characters with painterly backgrounds. The original batch used a second imagegen pass to refine its scenery. The second batch generated both layers in one pass. The prompts and metadata record each atlas and its export coordinates.",
        "",
        "| Civilization | Batch | Final atlas | Character prompt | Background edit prompt | Metadata |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for c in civs:
        slug = c["civilization"].removeprefix("CIVILIZATION_").lower()
        for batch in range(1, len(c["leaders"]) // BATCH_SIZE + 1):
            atlas = slug if batch == 1 else f"{slug}-{batch}"
            background = f"{atlas}-background.txt"
            edit_link = f"[Text]({background})" if (HERE / "art" / background).exists() else "—"
            art_index.append(f"| {c['name']} | {batch} | [PNG]({atlas}.png) | [Text]({atlas}.txt) | {edit_link} | [JSON]({atlas}.json) |")
    art_index += [
        "",
        "Run `python3 docs/minor-leaders/art_assets.py prompt docs/minor-leaders/concepts.json <civilization> --batch 2` to prepare a second-batch prompt. Use `export` with the same batch number to split a generated atlas into portraits.",
        "",
    ]
    (HERE / "art/README.md").write_text("\n".join(art_index))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--allow-missing-images", action="store_true")
    args = parser.parse_args()
    concepts = json.loads((HERE / "concepts.json").read_text())
    count = validate(concepts, not args.allow_missing_images)
    render(concepts)
    print(f"Validated {len(concepts)} civilizations, {count} leaders, all traits, 3–4 sentence backstories, and three Perpentach versions.")
