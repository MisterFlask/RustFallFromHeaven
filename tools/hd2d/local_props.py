"""The HD-2D mode's designed props from a local image model (docs/HD2D.md §5b, §10): every entry of
tools/hd2d/props.json through a ComfyUI server (FLUX on the owner's PC: free to run), each
candidate kept, a contact sheet to choose from, and the chosen ones made cards by tools/hd2d/art.py
prop (then `index`), replacing the code-drawn cards of the same name (tools/hd2d/draw.py).

  python tools/hd2d/local_props.py --workflow W.json [--comfy http://127.0.0.1:8188]
         [--only hut_1,ruin_3] [--seeds 3] [--out target/hd2d-local] [--pick hut_1=2,ruin_1=0]
  python tools/hd2d/local_props.py --cards [--out ..] [--pick ..]   # carding only, from what is there

W.json is a text-to-image workflow saved from ComfyUI in API format ("Save (API)"). The script
writes the prompt into its positive text node (the CLIPTextEncode -- or any node with a `text`
input -- whose text is not the negative; the first one when it cannot tell), the negative into the
one wired to a sampler's `negative` input if there is one, the seed into every `seed` and
`noise_seed` input, and 1024 x 1024 into the latent's `width` and `height`; everything else (the
model, sampler, steps) stays as the workflow has it. Candidates go to OUT/NAME_SEEDINDEX.png and
OUT/sheet.png shows them all, numbered; --pick chooses one by index (default 0) for each card.
Std only, plus Pillow and numpy for the carding (ComfyUI's bundled Python has both):
  C:/ComfyUI/python_embeded/python.exe tools/hd2d/local_props.py --workflow flux_klein_api.json
"""
import argparse
import json
import subprocess
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
PROPS = json.loads((HERE / "props.json").read_text(encoding="utf-8"))


def post(url, body):
    req = urllib.request.Request(url, data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.load(r)


def get(url, raw=False):
    with urllib.request.urlopen(url, timeout=60) as r:
        return r.read() if raw else json.load(r)


def fill(workflow, prompt, negative, seed):
    """The workflow with the prompt, negative, seed and size written in (a copy)."""
    wf = json.loads(json.dumps(workflow))
    # The text nodes wired to a sampler's negative input.
    neg_ids = set()
    for node in wf.values():
        neg = node.get("inputs", {}).get("negative")
        if isinstance(neg, list) and neg:
            neg_ids.add(str(neg[0]))
    texts = [k for k, n in wf.items() if isinstance(n.get("inputs", {}).get("text"), str)]
    pos = [k for k in texts if k not in neg_ids] or texts[:1]
    for k in texts:
        wf[k]["inputs"]["text"] = negative if k in neg_ids else (prompt if k in pos else wf[k]["inputs"]["text"])
    for node in wf.values():
        ins = node.get("inputs", {})
        for key in ("seed", "noise_seed"):
            if key in ins and not isinstance(ins[key], list):
                ins[key] = seed
        if "width" in ins and "height" in ins and not isinstance(ins["width"], list):
            ins["width"] = ins["height"] = 1024
    return wf


def generate(comfy, workflow, prompt, negative, seed, out_png):
    res = post(f"{comfy}/prompt", {"prompt": fill(workflow, prompt, negative, seed), "client_id": "ffh-hd2d"})
    pid = res["prompt_id"]
    t0 = time.time()
    while True:
        hist = get(f"{comfy}/history/{pid}")
        if pid in hist and hist[pid].get("outputs"):
            break
        if time.time() - t0 > 900:
            sys.exit(f"ComfyUI did not finish {out_png.name} in 15 minutes")
        time.sleep(1.0)
    for node in hist[pid]["outputs"].values():
        for im in node.get("images", []):
            q = urllib.parse.urlencode({"filename": im["filename"], "subfolder": im.get("subfolder", ""), "type": im.get("type", "output")})
            out_png.write_bytes(get(f"{comfy}/view?{q}", raw=True))
            return
    sys.exit(f"ComfyUI returned no image for {out_png.name}")


def sheet(out, names, seeds):
    from PIL import Image, ImageDraw

    cell = 192
    im = Image.new("RGB", (cell * seeds + 160, cell * len(names)), (40, 40, 44))
    d = ImageDraw.Draw(im)
    for r, name in enumerate(names):
        d.text((cell * seeds + 8, r * cell + 8), name, fill=(240, 240, 240))
        for c in range(seeds):
            p = out / f"{name}_{c}.png"
            if p.exists():
                t = Image.open(p).convert("RGB")
                t.thumbnail((cell - 8, cell - 8))
                im.paste(t, (c * cell + 4, r * cell + 4))
                d.text((c * cell + 8, r * cell + cell - 20), str(c), fill=(255, 220, 80))
    im.save(out / "sheet.png")
    print(f"contact sheet: {out / 'sheet.png'}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--workflow")
    ap.add_argument("--comfy", default="http://127.0.0.1:8188")
    ap.add_argument("--only", default="")
    ap.add_argument("--seeds", type=int, default=3)
    ap.add_argument("--out", default=str(HERE.parents[1] / "target" / "hd2d-local"))
    ap.add_argument("--pick", default="")
    ap.add_argument("--cards", action="store_true", help="only card what is already generated")
    a = ap.parse_args()
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    only = {s for s in a.only.split(",") if s}
    props = [p for p in PROPS["props"] if not only or p["name"] in only]
    if not a.cards:
        if not a.workflow:
            sys.exit("--workflow W.json (a ComfyUI API-format workflow) is needed to generate")
        workflow = json.loads(Path(a.workflow).read_text(encoding="utf-8"))
        for p in props:
            for k in range(a.seeds):
                png = out / f"{p['name']}_{k}.png"
                if png.exists():
                    continue
                # Seeds fixed by the name: a rerun reproduces the same candidates.
                seed = sum(map(ord, p["name"])) * 1000 + k
                print(f"{p['name']} #{k} (seed {seed})", flush=True)
                generate(a.comfy.rstrip("/"), workflow, f"{PROPS['style']}. {p['prompt']}", PROPS["negative"], seed, png)
        sheet(out, [p["name"] for p in props], a.seeds)
    picks = dict(kv.split("=") for kv in a.pick.split(",") if kv)
    art = HERE / "art.py"
    for p in props:
        k = int(picks.get(p["name"], 0))
        src = out / f"{p['name']}_{k}.png"
        if not src.exists():
            print(f"{p['name']}: no candidate {k}, the code-drawn card stays")
            continue
        subprocess.run([sys.executable, str(art), "prop", str(src), p["name"], str(p["height"]), str(p.get("deepen", 1.0))], check=True)
    subprocess.run([sys.executable, str(art), "index"], check=True)


if __name__ == "__main__":
    main()
