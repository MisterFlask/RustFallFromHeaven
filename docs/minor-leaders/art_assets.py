#!/usr/bin/env python3
"""Prepare imagegen atlas prompts and mechanically export their five square leaderhead cells."""
import argparse
import json
import shutil
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
STYLE = """Create a LANDSCAPE 1536 by 1024 image containing a strict 3 COLUMN by 2 ROW grid of SIX EXACTLY EQUAL SQUARE CELLS, every cell 512 by 512. This is a production leaderhead atlas for Fall from Heaven fantasy strategy game. FIVE separate independent square character portraits fill first five cells in reading order; sixth bottom-right cell solid dark navy. No gutters, no borders, no captions, no letters, no numbers, no watermarks, no shared scene across cells. Each portrait keeps its entire head and shoulders safely inside its square. Figure occupies most of each square; face clearly visible, upper third; one expressive hand prop if specified. House art: strongly chunky retro HD pixel art character, visible square pixel blocks ~64 across each cell, thick solid black silhouette outlines and black lines on major internal shapes, confident 3-5 step shading, saturated jewel tones. Softer richly painted detailed fantasy backgrounds, atmospheric depth. All five distinct people, faces and costumes accurately follow descriptions."""
BACKGROUND = """Edit this 1536x1024 leaderhead atlas. Preserve the exact 3-column by 2-row square-cell layout, character identities, faces, poses, active hand tasks, distinct primary costume colors, outfits, and empty dark navy bottom-right sixth cell. The FIVE FOREGROUND CHARACTERS AND THEIR HANDHELD PROPS must stay strongly chunky pixel art with clearly visible large square pixel blocks, black outlines and 16-bit style stepped shading. Change ONLY THE BACKGROUNDS behind each character into HIGH-DEFINITION PAINTERLY ARTISTIC REALISM: smooth rich brushwork, fine realistic material detail, believable stone and wood, atmospheric perspective and soft natural realistic lighting. Backgrounds must have NO PIXEL GRID, NO CHUNKY SQUARES, NO PIXEL-ART TEXTURE. The contrast must be obvious: pixel-art character cutouts against continuous-tone realistic fantasy paintings like detailed oil-painted concept-art vistas. Preserve each background's setting and story-specific objects exactly. Do not pixelate the full image or add pixel texture to backgrounds. Do not smooth the foreground characters into realism. No text, no border, no watermark. Preserve square-cell crop boundaries at x512,x1024,y512."""


def run(*args):
    return subprocess.check_output(args, text=True).strip()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["prompt", "background-prompt", "export", "export-background"])
    parser.add_argument("concepts", type=Path)
    parser.add_argument("civilization", help="Full CIVILIZATION_* identifier or lower-case slug")
    parser.add_argument("source", nargs="?", type=Path)
    args = parser.parse_args()
    civs = json.loads(args.concepts.read_text())
    key = args.civilization.upper().removeprefix("CIVILIZATION_")
    civ = next(c for c in civs if c["civilization"] == "CIVILIZATION_" + key)
    slug = key.lower()
    art = HERE / "art"
    portraits = HERE / "portraits"
    art.mkdir(exist_ok=True)
    portraits.mkdir(exist_ok=True)
    if args.command == "background-prompt":
        prompt = BACKGROUND + f" Civilization: {civ['name']}. Retain these five specific active hand tasks in reading order: "
        prompt += "; ".join(f"{l['name']}: {l['hand_activity']}" for l in civ["leaders"])
        (art / f"{slug}-background.txt").write_text(prompt + "\n")
        print(prompt)
    elif args.command == "prompt":
        style = STYLE
        if slug == "balseraphs":
            style = style.replace("All five distinct people, faces and costumes", "First three portraits are three personas of the SAME man, Perpentach, with identical facial bone structure and wild black-and-silver hair, gold-flecked eyes and his white porcelain smiling carnival mask with a black tear. They have different moods, costumes, gestures and settings. Last two are different people. All faces and costumes")
        prompt = style + " Art direction is mandatory: each of the five portraits has a DIFFERENT DOMINANT PRIMARY COLOR SCHEME, a distinct clothing silhouette, a distinct emotional vibe, and a different specific HAND ACTIVITY. Show people actively engaged in their task, with both hands interacting naturally where the activity needs it. Avoid five people simply posing with an object held against the chest. Keep the task visible within the square bust composition; forearms, working surface or tools may enter the lower third. Do not homogenize the palette into red/brown/gold or dark robes across the set. The specified primary palette must visibly dominate the clothing and lighting of that cell."
        prompt += f" {civ['name']} civilization. Cells in exact reading order:\n"
        for i, l in enumerate(civ["leaders"]):
            prompt += f"{i+1}. {l['name']}, {l['epithet']}. PRIMARY PALETTE: {l['primary_palette']}. VIBE: {l['vibe']}. CLOTHING: {l['clothing']}. ACTIVE HAND TASK (supersedes any static prop pose in description): {l['hand_activity']}. APPEARANCE AND SETTING: {l['portrait']}\n"
        prompt = prompt.rstrip()
        (art / f"{slug}.txt").write_text(prompt + "\n")
        print(prompt)
    else:
        if not args.source:
            parser.error("export requires the generated source image path")
        previous = json.loads((art / f"{slug}.json").read_text()) if args.command == "export-background" else None
        dst = art / f"{slug}.png"
        shutil.copyfile(args.source, dst)
        w, h = map(int, run("magick", "identify", "-format", "%w %h", str(dst)).split())
        assert w % 3 == 0 and h % 2 == 0 and w // 3 == h // 2, f"Expected 3x2 square cells, got {w}x{h}"
        cell = w // 3
        entries = []
        for i, leader in enumerate(civ["leaders"]):
            x, y = (i % 3) * cell, (i // 3) * cell
            path = portraits / (leader["id"] + ".png")
            subprocess.run(["magick", str(dst), "-crop", f"{cell}x{cell}+{x}+{y}", "+repage", "-resize", "384x384", str(path)], check=True)
            entries.append({"leader": leader["id"], "cell": i + 1, "crop": [x, y, cell, cell], "portrait": str(path.relative_to(HERE)), "size": [384, 384]})
        metadata = {"generator": "image_gen.imagegen", "generated_source": str(args.source), "source_copy": f"art/{slug}.png", "prompt": f"art/{slug}.txt", "sheet_size": [w, h], "layout": "3 columns x 2 rows, row-major; sixth cell unused", "export": "Square cell extraction and resize only; no retouching", "leaders": entries}
        if previous:
            metadata["initial_generated_source"] = previous["generated_source"]
            metadata["background_edit_prompt"] = f"art/{slug}-background.txt"
            metadata["style"] = "Pixel-art characters and props against high-definition painterly realistic backgrounds"
        (art / f"{slug}.json").write_text(json.dumps(metadata, indent=2) + "\n")
        print(f"Exported {civ['name']}: five individual 384x384 portraits from {w}x{h} source sheet.")
