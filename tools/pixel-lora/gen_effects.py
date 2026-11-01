"""Generate pixel-art effect source images with the OpenAI Images API.

Usage: python gen_effects.py OUT_DIR [--only name1,name2] [--model MODEL]
Reads OPENAI_API_KEY from the environment. Writes OUT_DIR/<name>.png (1024px, transparent).
"""
import argparse, base64, json, os, sys, urllib.request
from concurrent.futures import ThreadPoolExecutor

STYLE = (
    "16-bit console-era pixel art game effect sprite, chunky visible square pixels on a coarse grid "
    "(as if drawn at about 96x96 pixels and scaled up with nearest-neighbor), limited palette of roughly "
    "24 colors, hand-placed pixel clusters, selective dark outlines on the outer edge, stepped cel shading, "
    "no anti-aliasing, no blur, no gradients, no soft glow or bloom around the effect. Single effect centered, "
    "isolated on a flat solid pure black (#000000) background, no ground, no shadow, no border, no frame, no text."
)

EFFECTS = {
    "fireball": "a roaring fireball flying to the right, orange and yellow core, red trailing flames",
    "explosion": "a big round explosion burst, white-yellow core, orange fire, dark smoke puffs at the edges",
    "lightning_bolt": "a vertical jagged lightning bolt striking down, pale cyan and white with violet edges",
    "ice_shards": "a cluster of sharp ice crystal shards bursting upward, pale blue, white highlights",
    "frost_nova": "a ring of frost and ice spikes radiating outward, top-down three-quarter view",
    "heal_sparkles": "a rising column of green and gold healing sparkles and plus-shaped glints",
    "holy_light": "a beam of holy golden light descending from above with small radiant stars",
    "poison_cloud": "a bubbling toxic poison cloud, sickly green and purple with skull-free bubbles",
    "shadow_bolt": "a swirling dark purple and black shadow orb with tendrils, flying to the right",
    "arcane_circle": "a glowing magic summoning circle with runes, seen from a three-quarter top-down angle, blue-violet",
    "slash_arc": "a white crescent sword slash arc with motion streaks, sharp and fast",
    "impact_hit": "a comic-style hit spark impact star, white and yellow with red tips",
    "dust_puff": "a puff of tan dust and small pebbles kicked up from the ground",
    "smoke_column": "a rising column of grey-brown smoke with darker curls",
    "water_splash": "a water splash with droplets flying outward, blue and white",
    "earth_spike": "jagged brown rock spikes erupting from cracked ground chunks",
    "wind_gust": "a swirling white-green wind gust with leaves caught in it, moving to the right",
    "magic_missile": "three small glowing violet magic missiles with sparkling trails, flying right",
    "fire_pillar": "a tall pillar of fire erupting upward, red base, yellow-white top",
    "lava_burst": "molten lava splashing up in blobs, bright orange with dark crusted edges",
    "blood_drain": "a stream of dark red life-draining wisps spiraling inward",
    "teleport_flash": "a vertical teleport flash of blue-white light with a ring of particles",
    "shield_bubble": "a translucent hexagon-patterned magic shield bubble, light blue edges",
    "buff_aura": "an upward swirling golden aura ring with arrow-like sparks",
    "debuff_curse": "a sinister downward spiral of purple and black curse glyphs",
    "coin_burst": "a burst of spinning gold coins flying outward with glints",
    "level_up_stars": "a fountain of small yellow and white stars bursting upward",
    "meteor": "a flaming meteor rock falling diagonally down-left with a long fire trail",
    "vine_whip": "green thorny vines lashing out and curling",
    "acid_splash": "a splash of bright green acid with sizzling droplets",
}


def gen(name, prompt, out_dir, model):
    path = os.path.join(out_dir, f"{name}.png")
    if os.path.exists(path):
        return name, "exists"
    body = json.dumps({
        "model": model, "prompt": f"{prompt}. {STYLE}", "size": "1024x1024",
        "output_format": "png", "quality": "high", "n": 1,
    }).encode()
    req = urllib.request.Request("https://api.openai.com/v1/images/generations", data=body, headers={
        "Authorization": f"Bearer {os.environ['OPENAI_API_KEY']}", "Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=300) as r:
            data = json.load(r)
    except urllib.error.HTTPError as e:
        return name, f"error {e.code}: {e.read().decode()[:300]}"
    with open(path, "wb") as f:
        f.write(base64.b64decode(data["data"][0]["b64_json"]))
    return name, "ok"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out_dir")
    ap.add_argument("--only")
    ap.add_argument("--model", default="gpt-image-2.5-sunburst")
    a = ap.parse_args()
    os.makedirs(a.out_dir, exist_ok=True)
    names = a.only.split(",") if a.only else list(EFFECTS)
    with ThreadPoolExecutor(4) as ex:
        for name, status in ex.map(lambda n: gen(n, EFFECTS[n], a.out_dir, a.model), names):
            print(name, status, flush=True)


if __name__ == "__main__":
    main()
