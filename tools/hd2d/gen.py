"""Sheets of pixel-art objects for the HD-2D mode (docs/HD2D.md §5b, §5c) from OpenAI's Images API,
`gpt-image-2.5-sunburst` pinned to its snapshot `gpt-image-2.5-sunburst-2026-09-08` (gpt-image-1 was
shut down on 2026-10-23). Each sheet is a grid of single objects on a transparent background, cut
into one source image per object by `tools/hd2d/art.py split` and made cards by `art.py prop`.

  python3 tools/hd2d/gen.py SHEET [OUT_DIR] [--quality Q] [--size WxH] [--ref IMG ...] [--seed N]

SHEET names a prop sheet of SHEETS below, or a town kit sheet of tools/hd2d/towns.json
(`town:<family>`, `landmark:<n>`; `--list` prints them all). --ref IMG (repeatable) sends images
the sheet must match in style (an accepted sheet of the kit): the call goes to the edits endpoint.
--seed N only names the output (SHEET_N.png), for re-rolls side by side. The PNG and the API's usage
(tokens, the dollar cost at the model's list prices) go to OUT_DIR (default target/hd2d-gen/), the
usage appended to OUT_DIR/usage.jsonl. Needs OPENAI_API_KEY (any value where a proxy injects the
real one) and network access; std only.

The API (developers.openai.com, model page and image generation guide, checked 2026-10-30): size
any WIDTHxHEIGHT (1024x1024, 1536x1024, 1024x1536, 2048x2048 ... 3840x2160) or auto; quality low,
medium, high, xhigh, max or auto; background opaque, transparent or auto (transparent needs png or
webp output); billed by token, prices in PRICE below.
"""
import base64
import json
import os
import sys
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path

MODEL = "gpt-image-2.5-sunburst-2026-09-08"
API = "https://api.openai.com/v1/images"
TOWNS = Path(__file__).resolve().parent / "towns.json"

# The house style, shared by every prop sheet: the look target's (docs/art/hd2d-reference.png) small,
# detailed 16-bit objects, muted and earthy, lit from the upper left.
STYLE = (
    "16-bit pixel art game sprites, crisp pixels, a dark one-pixel outline, detailed but readable at "
    "small size, muted earthy palette (deep olive greens, warm browns, slate greys; no neon, no lime), "
    "lit from the upper left, seen from a three-quarter top-down view as in a strategy game map. "
    "A 3 by 3 grid of nine separate objects, each one isolated and well apart from the others, "
    "nothing touching, no ground plane, no cast shadow, no text, no labels, transparent background. "
)

SHEETS = {
    "village": "The nine objects: a small thatched peasant hut, a second smaller thatched hut with a "
    "stone chimney, a short wooden rail fence section, a haystack, a stone well with a little roof, a "
    "wooden hand cart, a low broken stone wall ruin, a ring of three grey standing stones, a small "
    "wooden shrine on a post.",
    "meadow": "The nine objects: a tuft of tall grass, a second tuft of grass with seed heads, a "
    "clump of reeds, a small fern, a cluster of brown mushrooms, a fallen mossy log, a small round "
    "berry shrub, a patch of white and yellow wildflowers, a patch of blue and violet wildflowers.",
    "shore": "The nine objects: low clusters of dark wet shoreline rocks and pebbles, each a wide "
    "flat horizontal band much wider than tall, mossy on top, grey-brown, some with tiny tufts of "
    "grass; all nine are variations of a rocky riverbank or sea-shore edge.",
    "trees": "The nine objects: small trees for a dense forest seen from above: three different "
    "round broadleaf trees in deep olive green with lighter leaf tips, two slim dark cypress trees, "
    "two small dark green fir trees, a white birch tree, a small autumn-gold tree.",
    "corrupt": "The nine objects: a twisted black thorn bush, a cluster of glowing violet mushrooms, "
    "a small dark obelisk with violet runes, a dead grey tree stump, a pile of violet crystal shards, "
    "a black rock with a glowing violet crack, a withered purple bramble, a small cracked stone altar, "
    "a ring of dark standing stones.",
    "hell": "The nine objects: a smoking brimstone vent, a small pile of bones and skulls, a charred "
    "black tree stump, a jagged obsidian rock, a cluster of black rocks with glowing orange cracks, a "
    "small iron spike, a burning brazier, a charred thorn bush, a glowing ember-filled crater rim.",
    "elven": "The nine objects: a small elven lantern post with a glowing blue-white light, a white "
    "stone elven archway ruin, a slender silver birch, a round shrub with tiny glowing golden "
    "flowers, a small stone fountain, a patch of glowing white lilies, a mossy carved stone marker, "
    "a small elven garden pavilion, a tall slim pale-green tree.",
    "desert": "The nine objects: a dry desert shrub, a small cactus, a pile of sandstone rocks, a "
    "bleached animal skull, a dry grass tuft, a small palm sapling, a broken sandstone pillar, a "
    "half-buried clay pot, a low dune ridge of rippled sand.",
}

# gpt-image-2.5-sunburst list prices (USD per million tokens; developers.openai.com, 2026-10-30).
# Text output is not billed (the model returns images).
PRICE = {"text_in": 5.0, "text_in_cached": 1.25, "image_in": 8.0, "image_in_cached": 2.0, "image_out": 30.0}


def town_sheets():
    """The town kit's sheets (tools/hd2d/towns.json, docs/HD2D.md §5c): one a family (its houses,
    tower and wall pieces, in a 3 x 3 grid) and the civilizations' landmarks four to a sheet (2 x 2).
    Returns {name: (prompt, size, [piece names in the order asked])}."""
    kit = json.loads(TOWNS.read_text(encoding="utf-8"))
    out = {}
    for fam, f in kit["families"].items():
        pieces = f["pieces"]
        listed = " ".join(f"({k + 1}) {p['prompt']}." for k, p in enumerate(pieces))
        prompt = (
            f"{kit['style']} {kit['view']} {kit['owner']} "
            f"A 3 by 3 grid of {len(pieces)} separate buildings of one people, {f['look']} "
            f"Each one isolated, well apart from the others, the same scale and style. In reading order: {listed}"
        )
        out[f"town:{fam}"] = (prompt, kit.get("size", "1024x1024"), [f"town_{fam}_{p['name']}" for p in pieces])
    lms = list(kit["landmarks"].items())
    for s in range(0, len(lms), 4):
        group = lms[s:s + 4]
        listed = " ".join(f"({k + 1}) {lm['prompt']}." for k, (_, lm) in enumerate(group))
        prompt = (
            f"{kit['style']} {kit['view']} {kit['owner']} "
            f"A 2 by 2 grid of {len(group)} separate grand landmark buildings, each the tallest building "
            f"of its city, a tall composition rising well above a town's roofs, each one isolated, well "
            f"apart from the others. In reading order: {listed}"
        )
        out[f"landmark:{s // 4 + 1}"] = (prompt, kit.get("landmark_size", "1024x1536"), [lm["name"] for _, lm in group])
    return out


def multipart(fields, files):
    """A multipart/form-data body (std only): fields {name: value}, files [(name, path)]."""
    b = uuid.uuid4().hex
    parts = []
    for k, v in fields.items():
        parts.append(f'--{b}\r\nContent-Disposition: form-data; name="{k}"\r\n\r\n{v}\r\n'.encode())
    for k, p in files:
        parts.append(f'--{b}\r\nContent-Disposition: form-data; name="{k}"; filename="{Path(p).name}"\r\nContent-Type: image/png\r\n\r\n'.encode() + Path(p).read_bytes() + b"\r\n")
    parts.append(f"--{b}--\r\n".encode())
    return b"".join(parts), f"multipart/form-data; boundary={b}"


def cost(u):
    """The dollar cost of a call from its usage (input split into text and image tokens, each with a
    cached part when the API reports one)."""
    det = u.get("input_tokens_details", {})
    text = det.get("text_tokens", u.get("input_tokens", 0) - det.get("image_tokens", 0))
    image = det.get("image_tokens", 0)
    cached = det.get("cached_tokens", 0)
    # The cached tokens are reported as one number; count them against the image input first (the
    # reference images are what repeats between calls), then the text.
    ci = min(cached, image)
    ct = min(cached - ci, text)
    out = u.get("output_tokens_details", {}).get("image_tokens", u.get("output_tokens", 0))
    return ((text - ct) * PRICE["text_in"] + ct * PRICE["text_in_cached"] + (image - ci) * PRICE["image_in"] + ci * PRICE["image_in_cached"] + out * PRICE["image_out"]) / 1e6


def read_stream(r):
    """The finished image of a streamed call (server-sent events: partial images, then a
    `*.completed` event with the image and the usage) as {"data": [{"b64_json"}], "usage"}."""
    for line in r:
        line = line.decode().strip()
        if not line.startswith("data:"):
            continue
        ev = json.loads(line[5:])
        if ev.get("type", "").endswith(".completed"):
            return {"data": [{"b64_json": ev["b64_json"]}], "usage": ev.get("usage", {})}
        if ev.get("type") == "error" or "error" in ev:
            sys.exit(f"stream error: {ev}")
    sys.exit("the stream ended without an image")


def opt(name, default=None, many=False):
    vals = [sys.argv[k + 1] for k, a in enumerate(sys.argv[:-1]) if a == f"--{name}"]
    return vals if many else (vals[-1] if vals else default)


def main():
    flags_with_value = {"--quality", "--size", "--ref", "--seed"}
    args, skip = [], False
    for a in sys.argv[1:]:
        if skip:
            skip = False
        elif a in flags_with_value:
            skip = True
        elif not a.startswith("--"):
            args.append(a)
    sheets = {k: (STYLE + v, "1024x1024", None) for k, v in SHEETS.items()}
    if TOWNS.exists():
        sheets.update(town_sheets())
    if "--list" in sys.argv:
        for k, (_, size, names) in sheets.items():
            print(k, size, " ".join(names or []))
        return
    if "--prompt" in sys.argv and args and args[0] in sheets:
        print(sheets[args[0]][0])
        return
    if not args or args[0] not in sheets:
        sys.exit(__doc__ + "\nsheets: " + ", ".join(sheets))
    name = args[0]
    prompt, size, names = sheets[name]
    size = opt("size", size)
    quality = opt("quality", "high")
    refs = opt("ref", many=True)
    seed = opt("seed")
    out = Path(args[1] if len(args) > 1 else "target/hd2d-gen")
    out.mkdir(parents=True, exist_ok=True)
    if refs:
        prompt = ("The attached images are accepted sprites of the same game: draw the new objects in "
                  "exactly their pixel art style, pixel size, outline, palette, camera and lighting. ") + prompt
    # Streamed (with a partial image or two on the way): a long call -- high quality takes over a
    # minute -- keeps its connection busy, where an idle one is cut by a proxy (HTTP 502).
    fields = {"model": MODEL, "prompt": prompt, "size": size, "quality": quality, "background": "transparent",
              "output_format": "png", "n": 1, "stream": "true" if refs else True, "partial_images": 2}
    key = os.environ.get("OPENAI_API_KEY", "sk-proxy")
    if refs:
        body, ctype = multipart(fields, [("image[]", r) for r in refs])
        url = f"{API}/edits"
    else:
        body, ctype = json.dumps(fields).encode(), "application/json"
        url = f"{API}/generations"
    t0 = time.time()
    for attempt in range(3):
        req = urllib.request.Request(url, data=body, headers={"Authorization": f"Bearer {key}", "Content-Type": ctype})
        try:
            with urllib.request.urlopen(req, timeout=900) as r:
                res = read_stream(r)
            break
        except urllib.error.HTTPError as e:
            msg = e.read().decode(errors="replace")
            # Only a rate limit is retried (not an account out of credit, nor a failure upstream,
            # which may have been billed).
            if e.code == 429 and attempt < 2 and "insufficient_quota" not in msg:
                time.sleep(20 * (attempt + 1))
                continue
            sys.exit(f"{name}: HTTP {e.code}: {msg}")
    stem = f"{name.replace(':', '_')}{'_' + seed if seed else ''}"
    (out / f"{stem}.png").write_bytes(base64.b64decode(res["data"][0]["b64_json"]))
    u = res.get("usage", {})
    rec = {"t": int(t0), "sheet": name, "file": f"{stem}.png", "model": MODEL, "size": size, "quality": quality,
           "refs": [Path(r).name for r in refs], "pieces": names, "latency": round(time.time() - t0, 1), "usage": u, "usd": round(cost(u), 4)}
    with open(out / "usage.jsonl", "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(json.dumps({k: rec[k] for k in ("sheet", "file", "quality", "size", "latency", "usd")}))


if __name__ == "__main__":
    main()
