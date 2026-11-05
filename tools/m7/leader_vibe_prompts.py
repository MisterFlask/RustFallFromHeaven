"""python3 leader_vibe_prompts.py prompts [--write]

Build prompts/leaders_hd_vibes/<key>_v1..v4.txt from leaders_hd_vibes.md (the vibes, with optional Wears:
outfits) and leaders_hd_likeness.json (body, usual costume, signature item, mood sentence); a leader not in
the JSON falls back to the likeness part of leaders_hd/<key>.txt (the description before its old pose)."""
import json, re, sys
from pathlib import Path
P = Path(sys.argv[1]); write = len(sys.argv) > 2 and sys.argv[2] == "--write"
md = (P / "leaders_hd_vibes.md").read_text()
HEAD = {"alexis": "Alexis", "amelanchier": "Amelanchier", "arendel": "Arendel", "arturus": "Arturus", "auric": "Auric",
        "barbarian": "Barbarian", "basium": "Basium", "beeri": "Beeri", "capria": "Capria", "cardith": "Cardith",
        "cassiel": "Cassiel", "charadon": "Charadon", "dain": "Dain", "decius": "Decius", "einion": "Einion",
        "ethne": "Ethne", "faeryl": "Faeryl", "falamar": "Falamar", "flauros": "Flauros", "garrim": "Garrim",
        "hannah": "Hannah", "hyborem": "Hyborem", "jonas": "Jonas", "kandros": "Kandros", "keelyn": "Keelyn",
        "mahala": "Mahala", "os_gabella": "Os-Gabella", "perpentach": "Perpentach", "random": "Random",
        "rhoanna": "Rhoanna", "sabathiel": "Sabathiel", "sandalphon": "Sandalphon", "sheelba": "Sheelba",
        "tasunke": "Tasunke", "tebryn": "Tebryn", "thessa": "Thessa", "valledia": "Valledia", "varn": "Varn"}
# whole likenesses written for the vibes (the owner's notes, 2026-11-05) live in leaders_hd_likeness.json:
# {key: {pronoun, body, usual costume, signature item, extra}}; a vibe's Wears: line replaces the costume.
# Leaders not in it fall back to the description in leaders_hd/<key>.txt (cut before its old pose) and EXTRA.
LIKENESS = json.loads((P / "leaders_hd_likeness.json").read_text()) if (P / "leaders_hd_likeness.json").exists() else {}
EXTRA = {
    "cassiel": ("What is left of the angel in him is uncanny: a face too perfect and too still for a mortal, a faint "
                "light under the skin."),
    "alexis": "Her gown is buttoned to the chin under the high lace collar.",
}
# likeness sentences that carried the old scene's hand pose or state, loosened to the item itself
# the owner's choice after the outline experiment (2026-11-05): the figure as bold, chunky pixel art with a
# thick black outline, against the soft painted background; appended after each description, as tested
OUTLINE = ("The character (not the background) is drawn as strongly chunky retro pixel art: very large square pixel "
           "blocks on the figure, only about 64 blocks across the whole picture, the face built from a few big clear "
           "blocks; a thick, solid, pure black outline three blocks wide around the entire silhouette, and bold black "
           "lines on every main shape inside it (eyes, nose, mouth, hands, folds and edges of the costume); flat colour "
           "areas with two or three shading steps, like a 16-bit sprite enlarged. The background stays soft, painterly "
           "and much less pixelated, so the figure stands out from it like a cut-out.")
FIX = {
    "keelyn": [("Both arms are wrapped tight around a small grinning red bat-winged IMP hugged against her cheek, a matching rose-pink bow tied on one of its horns.",
                "Her small grinning red bat-winged IMP, a matching rose-pink bow tied on one of its horns, is always with her.")],
    "sheelba": [("One fist clutches the lace against her chest.", "")],
    "jonas": [("In both hands, held at his chest like something precious and terrible, burns a small, fierce, undying flame.",
               "The small, fierce, undying FLAME of his goddess is always with him, in his hand or hovering at his shoulder.")],
    "capria": [(" beaded with cold sweat", "")],
    "amelanchier": [(" smudged with soot", "")],
}
sections = re.split(r"^## ", md, flags=re.M)[1:]
by_head = {}
for sec in sections:
    head = sec.split("\n", 1)[0].split(" (")[0].strip()
    by_head[head] = [(v, l, w or None) for v, l, w in
                     re.findall(r"^\d\. \*\*(.+?)\*\*: (.+)$(?:\n   Wears: (.+)$)?", sec, re.M)]
out = P / "leaders_hd_vibes"
if write:
    out.mkdir(exist_ok=True)
n = 0
for key, head in HEAD.items():
    vibes = by_head[head]
    assert len(vibes) == 4, (key, len(vibes))
    desc = (P / "leaders_hd" / f"{key}.txt").read_text().split("\n\n", 1)[1].strip()
    sents = re.split(r"(?<=[.])\s+(?=[A-Z])", desc)
    cut = next(i for i, s in enumerate(sents) if i > 0 and re.search(r"\bturned\b", s, re.I))
    whole = LIKENESS.get(key)  # (body, usual costume, signature) when written for the vibes
    he = (whole or {}).get("pronoun", "she")
    likeness = " ".join(sents[:cut])
    for old, new in FIX.get(key, []):
        assert old in likeness, (key, old)
        likeness = likeness.replace(old, new)
    colours = next((s for s in sents if s.startswith("Colours:")), "")
    if not write:
        print(f"{key:12} keeps {cut} sentences; pose starts: {sents[cut][:70]!r}")
        continue
    for i, (vibe, line, wears) in enumerate(vibes, 1):
        extra = whole["extra"] if whole else EXTRA.get(key, "")
        hidden = (key, vibe) == ("sabathiel", "Hidden Angel")  # the disguise: only his shadow shows the angel
        if whole:
            body_, costume, sig = whole["body"], whole["costume"], whole["signature"]
            dress = f"For this portrait {he} wears {wears}" if wears else f"{he.capitalize()} wears {costume}"
            lk = " ".join(x for x in (body_, dress, "" if hidden else sig) if x)
        else:
            assert not wears, f"{key}: a Wears: line needs a split likeness in LIKENESS"
            lk = likeness
        if hidden:
            extra = ("In the room he passes for a man, with a calm human face and grey eyes and no wings; only his "
                     "shadow shows what he is.")
        body = " ".join(x for x in (
            lk, extra,
            f"Pose, light and setting: {line[0].upper() + line[1:]}",
            f'Everything in the picture (the expression, where the eyes look, the hands, the light, the colours and the background) says "{vibe}".',
            "" if wears else colours, OUTLINE) if x)  # an outfit of its own carries its own colours
        body = re.sub(r"\s{2,}", " ", body)
        (out / f"{key}_v{i}.txt").write_text(f'VIBE: "{vibe}". {line[0].upper() + line[1:]}\n\n{body}\n', encoding="utf-8")
        n += 1
if write:
    print("wrote", n)
