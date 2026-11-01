"""Build the pixel-art style LoRA dataset.

Sprites are upscaled by an integer factor with nearest-neighbour (crisp pixels), centered on a
1024x1024 flat background, and given a caption file using the trigger word.
Usage: python -I build_dataset.py
"""
import os, random, re
from PIL import Image

TRIGGER = "pxlhd2d"
OUT = r"C:\ai-lora\pixel_hd2d\dataset"
HD2D = r"C:\dev\RustFallFromHeaven\.claude\worktrees\hd2d-phase1\crates\ffh-play\assets\app\r3d\hd2d\art"
UNITS = r"C:\dev\RustFallFromHeaven\tools\m7\roster\units"
EFFECTS = r"C:\ai-lora\pixel_hd2d\effects_raw\sunburst"
CANVAS = 1024
random.seed(7)


def words(stem):
    stem = re.sub(r"^(unit|town_lm|town)_", "", stem)
    stem = re.sub(r"_\d+$", "", stem)
    m = re.match(r"tree_(\w+?)(_snow)?$", stem)
    if m:
        stem = ("snowy_" if m.group(2) else "") + m.group(1) + "_tree"
    return stem.replace("_", " ")


def a(phrase):
    return ("an " if phrase[0] in "aeiou" else "a ") + phrase


def place(img, bg, fill=0.8):
    img = img.convert("RGBA")
    bbox = img.getchannel("A").getbbox()
    if bbox:
        img = img.crop(bbox)
    k = max(1, int(CANVAS * fill // max(img.size)))
    img = img.resize((img.width * k, img.height * k), Image.NEAREST)
    canvas = Image.new("RGB", (CANVAS, CANVAS), bg)
    canvas.paste(img, ((CANVAS - img.width) // 2, (CANVAS - img.height) // 2), img)
    return canvas


def save(name, img, caption):
    img.save(os.path.join(OUT, name + ".png"))
    with open(os.path.join(OUT, name + ".txt"), "w") as f:
        f.write(caption)


def main():
    os.makedirs(OUT, exist_ok=True)
    n = 0
    # Characters
    units = sorted(f for f in os.listdir(UNITS) if f.endswith(".png"))
    for f in random.sample(units, min(28, len(units))):
        save("unit_" + f[5:-4], place(Image.open(os.path.join(UNITS, f)), "white"),
             f"{TRIGGER}, {a(words(f[:-4]))}, full body game unit sprite on a plain white background")
        n += 1
    # Landmarks, buildings, trees, props
    art = sorted(f for f in os.listdir(HD2D) if f.endswith(".png") and not f.startswith("gnd_"))
    picks = [f for f in art if f.startswith("town_lm_")][:10]
    picks += [f for f in art if f.startswith("tree_")][::2]
    picks += random.sample([f for f in art if f.startswith("town_") and not f.startswith("town_lm_")], 6)
    picks += [f for f in art if re.match(r"(crystal|rock|bush|well|altar|obelisk|lantern|hut|ruin|lava_rock|falls)_1", f)]
    for f in picks:
        subj = a(words(f[:-4]) + (" fortress landmark building" if f.startswith("town_lm_") else ""))
        save("art_" + f[:-4], place(Image.open(os.path.join(HD2D, f)), "white"),
             f"{TRIGGER}, {subj}, isolated game sprite on a plain white background")
        n += 1
    # Effects (already pixel-styled, on black)
    if os.path.isdir(EFFECTS):
        for f in sorted(os.listdir(EFFECTS)):
            if f.endswith(".png"):
                img = Image.open(os.path.join(EFFECTS, f)).convert("RGB").resize((CANVAS, CANVAS), Image.NEAREST)
                save("fx_" + f[:-4], img, f"{TRIGGER}, {words(f[:-4])} spell effect, on a plain black background")
                n += 1
    print("images:", n)


if __name__ == "__main__":
    main()
