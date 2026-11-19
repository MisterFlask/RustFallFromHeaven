#!/usr/bin/env python3
"""Import the approved minor-leader portraits into the game's existing M7 roster.

Only exports existing art: full portraits use the roster's 256-color, no-dither
format, and face crops are square camera frames resized to 160 pixels. Run
`python3 tools/m7/m7.py export` afterward to update the client and sprite index.
"""
import json
from pathlib import Path

from PIL import Image

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SOURCE = ROOT / "docs/minor-leaders"
DEFAULT_FACE = [80, 0, 224]
# Camera-frame exceptions, in the approved 384-pixel portrait's coordinates.
FACE_CROPS = {
    "qasir_of_noon": [0, 0, 224],
    "naeviel_lasthymn": [0, 0, 224],
    "sella_nine_shares": [160, 0, 224],
    "leora_bellfound": [36, 0, 224],
    "hamira_stoneford": [56, 0, 224],
    "kamal_red_meridian": [32, 52, 224],
    "farah_quietwater": [56, 0, 224],
    "levan_pledge": [36, 0, 224],
    "meriel_bronze_dawn": [128, 0, 224],
    "dessa_quartzbell": [56, 0, 224],
    "ulna_shaleprint": [44, 0, 224],
    "mevara_rainsuture": [40, 0, 224],
    "thalan_ambervow": [40, 0, 224],
    "lethiel_hollowtone": [104, 0, 224],
    "berga_ironwedge": [106, 0, 224],
    "huldra_sevenvault": [44, 0, 224],
    "ursi_mothsilver": [110, 0, 224],
    "naresh_lintelmist": [52, 0, 224],
    "veda_secondwindow": [48, 0, 224],
    "dario_reefglass": [36, 0, 224],
    "carlon_mirrorsail": [32, 0, 224],
    "jeron_foghorn": [44, 0, 224],
    "tessa_inkclasp": [106, 0, 224],
    "irena_dregs": [50, 0, 224],
    "gavren_rook": [104, 0, 224],
    "sedi_horsethistle": [128, 0, 224],
    "meri_thunderchalk": [48, 0, 224],
    "bekka_tallfang": [106, 0, 224],
    "valira_thinveil": [40, 0, 224],
    "malrec_rubyhand": [132, 0, 224],
    "dianthe_nightjury": [56, 0, 224],
    "sivriel_cinnabar": [126, 0, 224],
    "aethi_vaultmoth": [60, 0, 224],
}


def palette(image):
    return image.quantize(256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)


def main():
    concepts = json.loads((SOURCE / "concepts.json").read_text())
    manifest_path = HERE / "roster/manifest.json"
    manifest = json.loads(manifest_path.read_text())
    target = HERE / "roster/leaders"
    prompts = HERE / "prompts/assets"
    target.mkdir(exist_ok=True)
    sources = {}
    for path in sorted((SOURCE / "art").glob("*.json")):
        atlas = json.loads(path.read_text())
        for entry in atlas["leaders"]:
            lid = entry["leader"]
            assert lid not in sources, f"Multiple source atlases for {lid}"
            sources[lid] = atlas
    count = 0
    for civ in concepts:
        for leader in civ["leaders"]:
            lid = leader["id"]
            key = f"leader_{lid}"
            source = SOURCE / "portraits" / f"{lid}.png"
            image = Image.open(source).convert("RGB")
            assert image.size == (384, 384), source
            x, y, side = FACE_CROPS.get(lid, DEFAULT_FACE)
            assert x >= 0 and y >= 0 and side > 0 and x + side <= 384 and y + side <= 384, lid
            palette(image).save(target / f"{key}.png", optimize=True)
            face = image.crop((x, y, x + side, y + side)).resize((160, 160), Image.Resampling.LANCZOS)
            palette(face).save(target / f"{key}_face.png", optimize=True)
            prompt = f"{leader['name']} — {leader['epithet']}\n\n{leader['portrait']}\n\n"
            atlas = sources[lid]
            source_prompt = atlas["prompt"]
            prompt += f"Source atlas generation prompt: docs/minor-leaders/{source_prompt}\n"
            prompt += (SOURCE / source_prompt).read_text().strip() + "\n"
            if background_prompt := atlas.get("background_edit_prompt"):
                prompt += f"\nFinal background-edit prompt: docs/minor-leaders/{background_prompt}\n"
                prompt += (SOURCE / background_prompt).read_text().strip() + "\n"
            width, height = atlas["sheet_size"]
            (prompts / f"{key}.txt").write_text(prompt)
            common = {
                "kind": "leader_portrait",
                "style": "hd",
                "source": f"../../docs/minor-leaders/portraits/{lid}.png",
                "prompt": f"prompts/assets/{key}.txt",
                "source_atlas": f"../../docs/minor-leaders/{atlas['source_copy']}",
                "gen_size": f"{width}x{height}",
            }
            manifest[key] = {**common, "file": f"leaders/{key}.png", "size": [384, 384]}
            manifest[f"{key}_face"] = {
                **common,
                "file": f"leaders/{key}_face.png",
                "size": [160, 160],
                "crop": [x, y, side],
            }
            count += 1
    manifest_path.write_text(json.dumps(manifest, indent=1) + "\n")
    print(f"Imported {count} approved minor leaders: full portraits, face crops, prompts and roster metadata.")


if __name__ == "__main__":
    main()
