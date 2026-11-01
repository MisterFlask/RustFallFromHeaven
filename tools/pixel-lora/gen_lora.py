"""Render prompts through ComfyUI's FLUX 3 Klein workflow with a LoRA added.

Usage: python gen_lora.py OUT_DIR [--lora NAME] [--strengths 1.0,0] [--seed 1234]
Prompts are full captions sent without the upsampler (it would rewrite away the trigger word).
"""
import argparse, copy, json, os, shutil, time, urllib.request

TEMPLATE = r"C:\Users\ironl\.claude\skills\local-image-gen\assets\flux3_klein_t2i_api.json"
BASE = "http://127.0.0.1:8188"
COMFY_OUT = r"C:\ComfyUI\ComfyUI\output"

PROMPTS = {
    "beetle_knight": "pxlhd2d, a knight in green armor riding a giant armored rhinoceros beetle, full body game unit sprite on a plain white background",
    "steam_golem": "pxlhd2d, a hulking brass steam-powered golem with glowing furnace chest, full body game unit sprite on a plain white background",
    "frog_wizard": "pxlhd2d, a small frog wizard in a starry blue robe holding a crooked wand, full body game unit sprite on a plain white background",
    "sky_lighthouse": "pxlhd2d, a stone lighthouse on a small floating rock island with waterfalls pouring off the edge, isolated game sprite on a plain white background",
    "treasure_chest": "pxlhd2d, an open wooden treasure chest overflowing with gold coins and red gems, isolated game item sprite on a plain white background",
    "potion_sheet": "pxlhd2d, a sprite sheet of eight different potion bottles in two rows of four, red, blue, green, purple, isolated game item icons on a plain white background",
    "chain_lightning": "pxlhd2d, a chain lightning spell effect arcing between three glowing blue orbs, on a plain black background",
    "smoke_puffs": "pxlhd2d, a sprite sheet of six small grey smoke puff particle textures in a row, from small to large, on a plain black background",
}


def post(path, data):
    req = urllib.request.Request(BASE + path, data=json.dumps(data).encode(), headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req) as r:
        return json.load(r)


def build(template, prompt, seed, lora, strength):
    w = copy.deepcopy(template)
    w["28"]["inputs"]["value"] = prompt
    w["127:116"]["inputs"]["switch"] = False  # skip the prompt upsampler
    w["19"]["inputs"]["megapixels"] = 1.0
    w["19"]["inputs"]["aspect_ratio"] = "1:1 (Square)"
    w["128:107"]["inputs"]["seed"] = seed
    if strength:
        w["200"] = {"class_type": "LoraLoaderModelOnly",
                    "inputs": {"model": ["128:101", 0], "lora_name": lora, "strength_model": strength}}
        w["128:132"]["inputs"]["model"] = ["200", 0]
    return w


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out_dir")
    ap.add_argument("--lora", default="pxlhd2d_v1_2250.safetensors")
    ap.add_argument("--strengths", default="1.0,0")
    ap.add_argument("--seed", type=int, default=1234)
    ap.add_argument("--only")
    a = ap.parse_args()
    os.makedirs(a.out_dir, exist_ok=True)
    template = json.load(open(TEMPLATE))
    names = a.only.split(",") if a.only else list(PROMPTS)
    jobs = []
    for name in names:
        for s in [float(x) for x in a.strengths.split(",")]:
            w = build(template, PROMPTS[name], a.seed, a.lora, s)
            tag = f"{name}_lora{s:g}"
            w["9"]["inputs"]["filename_prefix"] = "lora_test/" + tag
            jobs.append((tag, post("/prompt", {"prompt": w})["prompt_id"], time.time()))
    for tag, pid, t0 in jobs:
        while True:
            with urllib.request.urlopen(f"{BASE}/history/{pid}") as r:
                h = json.load(r).get(pid)
            if h and h.get("status", {}).get("completed") is not None or (h and h["status"].get("status_str") == "error"):
                break
            time.sleep(1)
        imgs = [i for o in h["outputs"].values() for i in o.get("images", [])]
        if not imgs:
            print(tag, "error", json.dumps(h["status"])[:400], flush=True)
            continue
        src = os.path.join(COMFY_OUT, imgs[0]["subfolder"], imgs[0]["filename"])
        shutil.copy(src, os.path.join(a.out_dir, tag + ".png"))
        print(tag, "ok", flush=True)


if __name__ == "__main__":
    main()
