"""python3 leader_vibe_merge.py prompts key1,key2,...

Fold prompts/leaders_hd_vibes_parts/<key>.json (one leader's likeness and four vibes with outfits) into
leaders_hd_vibes.md (the vibe lines and their Wears: lines) and leaders_hd_likeness.json (pronoun, body,
usual costume, signature item, mood sentence), after checking it. Then run leader_vibe_prompts.py --write.
"""
import json, re, sys
from pathlib import Path

P = Path(sys.argv[1])
keys = [k for k in sys.argv[2].split(",") if k]
md_p, lk_p = P / "leaders_hd_vibes.md", P / "leaders_hd_likeness.json"
md = md_p.read_text()
likeness = json.loads(lk_p.read_text())
HEAD = {"os_gabella": "Os-Gabella"}


def section(key):
    head = HEAD.get(key, key.capitalize())
    m = re.search(r"^## " + re.escape(head) + r" \(.*\)\n((?:(?:\d\. |   Wears: ).*\n)+)", md, re.M)
    if not m:
        raise SystemExit(f"{key}: no section '## {head} (...)' in {md_p}")
    return m


problems = []
for key in keys:
    part = json.loads((P / "leaders_hd_vibes_parts" / f"{key}.json").read_text())
    m = section(key)
    old_names = re.findall(r"^\d\. \*\*(.+?)\*\*", m.group(1), re.M)
    vibes = part["vibes"]
    if len(vibes) != 4:
        problems.append(f"{key}: {len(vibes)} vibes")
    if [v["name"] for v in vibes] != old_names:
        problems.append(f"{key}: vibe names changed {old_names} -> {[v['name'] for v in vibes]}")
    for f in ("pronoun", "body", "costume", "signature"):
        if not part.get(f, "").strip():
            problems.append(f"{key}: empty {f}")
    for v in vibes:
        for f in ("line", "wears"):
            t = v.get(f, "").strip()
            if not t or "\n" in t or "—" in t:
                problems.append(f"{key} {v['name']}: bad {f}")
    if problems:
        continue
    body = "".join(f"{i}. **{v['name']}**: {v['line'].strip()}\n   Wears: {v['wears'].strip()}\n"
                   for i, v in enumerate(vibes, 1))
    md = md[:m.start(1)] + body + md[m.end(1):]
    likeness[key] = {f: part.get(f, "").strip() for f in ("pronoun", "body", "costume", "signature", "extra")}
if problems:
    raise SystemExit("not merged:\n" + "\n".join(problems))
md_p.write_text(md)
lk_p.write_text(json.dumps(likeness, indent=1, ensure_ascii=False) + "\n")
print("merged", ", ".join(keys))
