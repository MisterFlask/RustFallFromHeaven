"""python3 leader_vibe_prompts.py prompts [--write]

Build prompts/leaders_hd_vibes/<key>_v1..v4.txt from leaders_hd_vibes.md and the likeness part of
leaders_hd/<key>.txt (the description up to the first pose sentence, the one that says 'turned')."""
import re, sys
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
ANGEL = ("He is an angel, terrible and unknowable: his eyes are pure white light with no pupils, his face is smooth "
         "and still as a mask with no human feeling in it, a second and a third pair of wings unfold behind the first "
         "with open eyes among the feathers, and any people near him reach only to his chest, kneeling or shielding "
         "their eyes.")
EXTRA = {
    "basium": ANGEL, "sabathiel": ANGEL,
    "cassiel": ("What is left of the angel in him is uncanny: a face too perfect and too still for a mortal, a faint "
                "light under the skin."),
    "alexis": "Her gown is buttoned to the chin under the high lace collar.",
    "hannah": ("She is hollow, and something else looks out through her: her face is completely relaxed and empty, no "
               "smile, no frown, brows and mouth slack; her eyes have no catchlight and are fixed level, on nothing or "
               "straight through the viewer; she does not blink. She is perfectly still while everything around her "
               "moves. This emptiness overrides any drama in the style: her face shows no feeling at all."),
    "os_gabella": ("She is weary beyond anything mortal: alive since the first morning of the world and bored by all of "
                   "it, and it shows in her whole body: shoulders dropped, weight leaning on what is beside her, eyelids "
                   "half-closed, mouth slack, no tension anywhere. This weariness overrides any drama in the style: no "
                   "heroic pose, no wind in her hair, no triumphant light, no rim light."),
}
# whole likenesses written for the vibes, replacing the extracted ones (the owner's notes, 2026-11-05)
LIKENESS = {
    "hannah": ("HANNAH THE IRIN, the mutineer pirate queen born of a cult dreamer, leader of the Lanun in the fantasy "
               "game Fall from Heaven: human-seeming; female; about thirty; wiry. Wind-burned olive skin with a grey, "
               "drowned pallor, long black hair always wet and hanging straight and heavy, pale grey eyes like worn sea "
               "glass with no light in them, faint ring-shaped marks on her neck and wrists like the grip of suckers.",
               "a long plain sea-coat of waterlogged grey-black sailcloth with no braid, buttons or sash, over a yellowed "
               "high-necked linen shift like a burial shirt, salt crust and dried kelp at the hems.",
               "Her mother's NECKLACE OF HOLLOW BONE NEEDLES on a black cord, each needle tipped with a stub of whale-fat "
               "wax and a tiny wick, two or three still burning, wax dripped and hardened down her collarbone."),
    "os_gabella": ("OS-GABELLA, Queen of Storms, the first woman made by the gods, who rebelled against them, leader of "
                   "the Sheaim in the fantasy game Fall from Heaven: immortal human; woman; ageless, looking about "
                   "thirty-five; tall. Pale skin gone dull with tiredness, long black hair hanging limp and loose with a "
                   "few strands across her face, dark violet eyes with deep shadows under them.",
                   "a heavy, high-collared black gown over deep crimson underlayers; no armour.",
                   "Her pale white glowing JEWELS are always on her, set into whatever she wears (collar, girdle, rings)."),
}
# likeness sentences that carried the old scene's hand pose or state, loosened to the item itself
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
    likeness = f"{whole[0]} She wears {whole[1]} {whole[2]}" if whole else " ".join(sents[:cut])
    for old, new in FIX.get(key, []):
        assert old in likeness, (key, old)
        likeness = likeness.replace(old, new)
    colours = next((s for s in sents if s.startswith("Colours:")), "")
    if not write:
        print(f"{key:12} keeps {cut} sentences; pose starts: {sents[cut][:70]!r}")
        continue
    for i, (vibe, line, wears) in enumerate(vibes, 1):
        lk, extra = likeness, EXTRA.get(key, "")
        if wears:
            assert whole, f"{key}: a Wears: line needs a split likeness in LIKENESS"
            lk = f"{whole[0]} For this portrait she wears {wears} {whole[2]}"
        if (key, vibe) == ("sabathiel", "Hidden Angel"):  # the disguise: only his shadow shows the angel
            lk = lk.replace(", and great WHITE FEATHERED WINGS folded high behind him", "")
            extra = ("In the room he passes for a man, with a calm human face and grey eyes; only his shadow shows "
                     "what he is.")
        body = " ".join(x for x in (
            lk, extra,
            f"Pose, light and setting: {line[0].upper() + line[1:]}",
            f'Everything in the picture (the expression, where the eyes look, the hands, the light, the colours and the background) says "{vibe}".',
            colours) if x)
        body = re.sub(r"\s{2,}", " ", body)
        (out / f"{key}_v{i}.txt").write_text(f'VIBE: "{vibe}". {line[0].upper() + line[1:]}\n\n{body}\n', encoding="utf-8")
        n += 1
if write:
    print("wrote", n)
