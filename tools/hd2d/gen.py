"""Prop sheets for the HD-2D mode (docs/HD2D.md §5b) from OpenAI's image API (gpt-image-1): each
sheet is a 3 x 3 grid of single pixel-art objects on a transparent background, to be cut into one
source image per object (a `split` step for tools/hd2d/art.py, still to write: no sheet has been
generated yet -- the API account had no credit) and made cards by `prop`.

  python3 tools/hd2d/gen.py SHEET [OUT_DIR] [--quality high|medium|low]

SHEET names an entry of SHEETS below; the PNG and the API's usage (tokens, the dollar cost at
gpt-image-1's list prices) go to OUT_DIR (default target/hd2d-gen/), the usage appended to OUT_DIR/usage.jsonl.
Needs OPENAI_API_KEY (any value where a proxy injects the real one) and network access; std only.
"""
import base64
import json
import os
import sys
import urllib.request
from pathlib import Path

# The house style, shared by every sheet: the look target's (docs/art/hd2d-reference.png) small,
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

# gpt-image-1 list prices (USD per million tokens).
PRICE = {"text_in": 5.0, "image_in": 10.0, "image_out": 40.0}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    quality = "high"
    if "--quality" in sys.argv:
        quality = sys.argv[sys.argv.index("--quality") + 1]
        args = [a for a in args if a != quality]
    if not args or args[0] not in SHEETS:
        sys.exit(__doc__ + "\nsheets: " + ", ".join(SHEETS))
    name = args[0]
    out = Path(args[1] if len(args) > 1 else "target/hd2d-gen")
    out.mkdir(parents=True, exist_ok=True)
    body = {
        "model": "gpt-image-1",
        "prompt": STYLE + SHEETS[name],
        "size": "1024x1024",
        "quality": quality,
        "background": "transparent",
        "output_format": "png",
        "n": 1,
    }
    req = urllib.request.Request(
        "https://api.openai.com/v1/images/generations",
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {os.environ.get('OPENAI_API_KEY', 'sk-proxy')}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=600) as r:
        res = json.load(r)
    (out / f"{name}.png").write_bytes(base64.b64decode(res["data"][0]["b64_json"]))
    u = res.get("usage", {})
    det = u.get("input_tokens_details", {})
    cost = (det.get("text_tokens", u.get("input_tokens", 0)) * PRICE["text_in"] + det.get("image_tokens", 0) * PRICE["image_in"] + u.get("output_tokens", 0) * PRICE["image_out"]) / 1e6
    rec = {"sheet": name, "quality": quality, "usage": u, "usd": round(cost, 4)}
    with open(out / "usage.jsonl", "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(json.dumps(rec))


if __name__ == "__main__":
    main()
