"""The recipe's image calls on the local FLUX 3 Image Klein model with the house pixel-art LoRA (`pxlhd2d`,
docs/art/PIXEL_LORA.md), through a ComfyUI server on this machine: free to run, nothing leaves the PC.

Same contract as `gen.edit` (the OpenAI path): the attached images go in as Klein's references, in order (the
guided sheet template first, then the style roster), the prompt is the recipe's sheet prompt with the LoRA's trigger
word in front, and the answer is `n` images at the template's size, written as `<out stem>_<k>.png` (or `<out>` when
n is 1). Returns {"ok", "files", "seconds", "seed", "usd": 0, "error"}.

  import klein
  res = klein.edit(tag="anim/units", images=[template, roster], prompt=text, out="work/x/raw_strike.png", n=2)

The graph is BFL's `flux3_klein_int8_mi2i` workflow flattened (UNETLoader -> LoraLoaderModelOnly ->
ModelSamplingFlux; the prompt and an empty negative through ReferenceMultiLatent with the references; one latent
repeated `n` times so each variant gets its own noise), without the prompt upsampler (it drops the trigger word).
Environment: COMFYUI_URL (default http://127.0.0.1:8188), KLEIN_LORA (default pxlhd2d_v2_1500.safetensors, the pose LoRA),
KLEIN_LORA_STRENGTH (default 1.0), KLEIN_REF_MP (megapixels each reference is resized to; default 1.6, about the
template's own size, so its guides are not shrunk), KLEIN_SEED (a base seed; default from the prompt and the out path,
so a re-run of the same call reproduces it).
"""
import hashlib, json, os, time, urllib.error, urllib.parse, urllib.request, uuid
from pathlib import Path

from PIL import Image

BASE = os.environ.get("COMFYUI_URL", "http://127.0.0.1:8188")
LORA = os.environ.get("KLEIN_LORA", "pxlhd2d_v2_1500.safetensors")
TRIGGER = "pxlhd2d"


def _req(path, data=None, headers=None, timeout=60):
    req = urllib.request.Request(BASE + path, data=data, headers=headers or {})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.read()


def _post_json(path, body):
    return json.loads(_req(path, json.dumps(body).encode(), {"Content-Type": "application/json"}))


def _upload(path):
    """Uploads an image to ComfyUI's input folder under a unique name; returns that name."""
    name = f"anim_{uuid.uuid4().hex[:12]}_{Path(path).name}"
    boundary = uuid.uuid4().hex
    body = (f"--{boundary}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"{name}\"\r\n"
            "Content-Type: image/png\r\n\r\n").encode() + Path(path).read_bytes() + (
            f"\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"overwrite\"\r\n\r\ntrue\r\n--{boundary}--\r\n"
            ).encode()
    res = json.loads(_req("/upload/image", body, {"Content-Type": f"multipart/form-data; boundary={boundary}"}))
    return res["name"]


def graph(prompt, refs, width, height, n, seed, strength=None, ref_mp=None, steps=16, init=None, denoise=1.0):
    """The API-format graph: `refs` are uploaded input names, in reference order; `init` (an uploaded name) makes it
    image-to-image: the sampler starts from that image, encoded, at `denoise` (1.0 redraws everything)."""
    strength = float(os.environ.get("KLEIN_LORA_STRENGTH", "1.0")) if strength is None else strength
    ref_mp = float(os.environ.get("KLEIN_REF_MP", "1.6")) if ref_mp is None else ref_mp
    g = {
        "unet": {"class_type": "UNETLoader", "inputs": {"unet_name": "flux3-image-klein-int8-convrot.safetensors",
                                                         "weight_dtype": "default"}},
        "clip": {"class_type": "CLIPLoader", "inputs": {"clip_name": "qwen3vl_4b_int8_convrot.safetensors",
                                                         "type": "flux3", "device": "default"}},
        "vae": {"class_type": "VAELoader", "inputs": {"vae_name": "flux2-vae.safetensors"}},
        "shift": {"class_type": "ModelSamplingFlux", "inputs": {"max_shift": 2.772588722239781,
                                                                "base_shift": 2.772588722239781, "width": 1024,
                                                                "height": 1024, "model": ["lora", 0]}},
        "pos": {"class_type": "CLIPTextEncode", "inputs": {"text": prompt, "clip": ["clip", 0]}},
        "neg": {"class_type": "CLIPTextEncode", "inputs": {"text": "", "clip": ["clip", 0]}},
        "refs": {"class_type": "ReferenceMultiLatent", "inputs": {
            "positive": ["pos", 0], "negative": ["neg", 0], "vae": ["vae", 0],
            "resolution": "resize", "resolution.megapixels": ref_mp}},
        "empty": {"class_type": "EmptyFlux2LatentImage", "inputs": {"width": width, "height": height, "batch_size": 1}},
        "batch": {"class_type": "RepeatLatentBatch", "inputs": {"samples": ["empty", 0], "amount": n}},
        "sample": {"class_type": "KSampler", "inputs": {
            "seed": seed, "steps": steps, "cfg": 1.0, "sampler_name": "euler", "scheduler": "simple", "denoise": denoise,
            "model": ["shift", 0], "positive": ["refs", 0], "negative": ["refs", 1], "latent_image": ["batch", 0]}},
        "decode": {"class_type": "VAEDecode", "inputs": {"samples": ["sample", 0], "vae": ["vae", 0]}},
        "save": {"class_type": "SaveImage", "inputs": {"filename_prefix": "anim_klein", "images": ["decode", 0]}},
    }
    if strength:
        g["lora"] = {"class_type": "LoraLoaderModelOnly", "inputs": {"model": ["unet", 0], "lora_name": LORA,
                                                                     "strength_model": strength}}
    else:
        g["shift"]["inputs"]["model"] = ["unet", 0]
    if init:
        g["init"] = {"class_type": "LoadImage", "inputs": {"image": init}}
        g["enc"] = {"class_type": "VAEEncode", "inputs": {"pixels": ["init", 0], "vae": ["vae", 0]}}
        g["batch"]["inputs"]["samples"] = ["enc", 0]
        del g["empty"]
    for k, name in enumerate(refs):
        g[f"ref{k}"] = {"class_type": "LoadImage", "inputs": {"image": name}}
        g["refs"]["inputs"][f"images.image{k}"] = [f"ref{k}", 0]
    return g


def edit(tag=None, images=(), prompt="", out="raw.png", n=2, size=None, timeout=1800, init=None, denoise=1.0, ref_mp=None,
         **_ignored):
    """One call: the images as references, n variants at the first image's size (or `size`, "WxH"); with `init` (a
    path) the sampler starts from that image at `denoise`, which then sets the size."""
    out = Path(out)
    out.parent.mkdir(parents=True, exist_ok=True)
    t0 = time.time()
    text = prompt if prompt.lstrip().startswith(TRIGGER) else f"{TRIGGER}, {prompt}"
    out.with_suffix(".prompt.txt").write_text(text, encoding="utf-8")
    if size:
        width, height = (int(v) for v in str(size).lower().split("x"))
    else:
        width, height = Image.open(init or images[0]).size
    width, height = width // 16 * 16, height // 16 * 16
    seed = int(os.environ["KLEIN_SEED"]) if os.environ.get("KLEIN_SEED") else int(
        hashlib.sha256((text + str(out)).encode()).hexdigest()[:12], 16)
    try:
        refs = [_upload(p) for p in images]
        g = graph(text, refs, width, height, n, seed, ref_mp=ref_mp, init=_upload(init) if init else None, denoise=denoise)
        pid = _post_json("/prompt", {"prompt": g, "client_id": "anim-sprint"})["prompt_id"]
        while True:
            h = json.loads(_req(f"/history/{pid}"))
            if pid in h:
                h = h[pid]
                break
            if time.time() - t0 > timeout:
                raise TimeoutError(f"no answer from ComfyUI in {timeout} s")
            time.sleep(1.0)
        st = h.get("status", {})
        if st.get("status_str") == "error":
            msgs = [m for m in st.get("messages", []) if m[0] == "execution_error"]
            raise RuntimeError(json.dumps(msgs)[-1500:])
        ims = [im for node in h["outputs"].values() for im in node.get("images", [])]
        files = []
        for k, im in enumerate(ims[:n]):
            q = urllib.parse.urlencode({"filename": im["filename"], "subfolder": im["subfolder"], "type": im["type"]})
            p = out if n == 1 else out.with_name(f"{out.stem}_{k + 1}{out.suffix}")
            p.write_bytes(_req(f"/view?{q}"))
            files.append(str(p))
        return {"ok": bool(files), "files": files, "seconds": round(time.time() - t0, 1), "seed": seed, "usd": 0,
                "error": None, "out": str(out)}
    except (urllib.error.HTTPError,) as e:
        return {"ok": False, "files": [], "usd": 0, "error": f"HTTP {e.code}: {e.read().decode(errors='replace')[:1500]}",
                "out": str(out)}
    except Exception as e:  # noqa: BLE001 -- the recipe reads the error field
        return {"ok": False, "files": [], "usd": 0, "error": f"{type(e).__name__}: {e}", "out": str(out)}

