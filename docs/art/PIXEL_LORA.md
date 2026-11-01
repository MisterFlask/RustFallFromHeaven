# Pixel-art style LoRA (`pxlhd2d`)

A LoRA for the local FLUX 3 Image Klein model that turns any prompt into crisp 16-bit pixel art in
this game's house style (dark outlines, warm limited palette, chunky unit proportions). Trained
2026-11-01 on this machine. Use it when generating new units, props, items, icons or effect /
particle textures with the local model instead of prompting the style from scratch.

![Each pair: plain Klein (left) vs the LoRA (right), same prompt and seed, none of these subjects
were in the training set](pixel-lora-compare.jpg)

## Where things are

These live outside the repository (they are large); the scripts and config are copied into
`tools/pixel-lora/` so the run can be reproduced.

| Thing | Path |
|---|---|
| LoRA in use (step 2250 of 2500) | `C:\ComfyUI\ComfyUI\models\loras\pxlhd2d_v1_2250.safetensors` |
| All checkpoints (every 250 steps, 153 MB each) | `C:\ai-lora\pixel_hd2d\output\pxlhd2d_v1\` |
| Training dataset (92 images + captions) | `C:\ai-lora\pixel_hd2d\dataset\` |
| Base model weights used for training | `C:\ai-lora\models\flux-3-image-klein-base\` |
| Trainer (ai-toolkit, own venv) | `C:\ai-toolkit\` |

## Using it

- **Trigger word `pxlhd2d`** at the start of the prompt. Captions followed the pattern
  `pxlhd2d, <a subject>, full body game unit sprite on a plain white background` (units),
  `..., isolated game sprite on a plain white background` (buildings, props, items) and
  `..., spell effect, on a plain black background` (effects). Prompts in the same shape work best.
- **Upsampler off.** Klein's prompt upsampler rewrites the prompt and drops the trigger word, so send
  the caption as written.
- **Model.** It was trained on Klein *Base* but works on the distilled int8 model ComfyUI already
  uses (`flux3-image-klein-int8-convrot.safetensors`, 16 steps, cfg 1): add a
  `LoraLoaderModelOnly` node between the UNET loader and `ModelSamplingFlux`, strength 1.0.
  `tools/pixel-lora/gen_lora.py` does exactly this against the `local-image-gen` skill's API
  template and renders each prompt with and without the LoRA (`--strengths 1.0,0`). About 13 s per
  1 MP image on the RTX 4080 Laptop.
- **Still post-process.** The output is pixel-art *style*, not an exact grid. Snap it with the
  `local-image-gen` skill's `sprite.py` (single objects) or `pixelate.py` (scenes) before it goes
  into `crates/ffh-play`.
- **License.** The LoRA is a derivative of FLUX 3 Image Klein and falls under the FLUX Kommunity
  License (non-commercial, with a commercial-output exception for qualifying users). Check it before
  shipping anything made with it.

## What it is good and bad at

Strong: characters and units, props, items and icon sheets (a 2x4 potion sheet came out clean),
buildings. Effects: good shapes (lightning, smoke puff sequences, spark bursts) but little variety
in one design (every "vortex" is the same spiral). Small details (held weapons, props) vary between
seeds. Steps 2000-2250 were the best balance; 2500 was not clearly better.

## How it was made

1. **Effects source art.** We had little effect art, so 30 spell effects were generated with the
   OpenAI Images API, model `gpt-image-2.5-sunburst` (the newest that drew clean pixel art; it does
   not support transparent backgrounds, so effects are on solid black): `tools/pixel-lora/gen_effects.py`.
2. **Dataset.** `tools/pixel-lora/build_dataset.py`: 28 unit sprites from `tools/m7/roster/units`,
   34 HD-2D buildings, landmarks, trees and props from the hd2d-phase1 branch's
   `crates/ffh-play/assets/app/r3d/hd2d/art`, plus the 30 effects. Sprites are upscaled by an integer
   factor with nearest-neighbour (crisp pixels) onto a 1024 px flat canvas, white for objects, black
   for effects, each with a caption file.
3. **Training.** ai-toolkit, arch `flux3`, Klein Base quantized to `convrot8`, LoRA rank 32,
   adamw8bit, lr 1e-4, 2500 steps at 512/768 px, cached latents and text embeddings:
   `tools/pixel-lora/train_v1.yaml`. Fits in 12 GB VRAM at ~2.7 s/step; ~2.5 h with sampling.
   Run from `C:\ai-toolkit`: `.\venv\Scripts\python.exe run.py <config>`.
   - The config needs `sample.neg: ""`: ai-toolkit defaults it to `False`, which crashes the FLUX 3
     text encoder when sample prompts are cached.
   - ai-toolkit's requirements miss `torchaudio`; install the build matching torch from the
     PyTorch cu128 index.

To retrain (more effects, particle sheets, a v2), add images and captions to the dataset, change the
config's `name`, and run it again.
