# Minor leader portrait sources

Each civilization has a final generated 1536 by 1024 source atlas containing five square portraits and an unused sixth cell. The individual leaderheads in `../portraits/` are square cell exports resized to 384 by 384 pixels, without retouching.

The first generation establishes the pixel characters, different palettes, clothing and active hand tasks. A second imagegen edit preserves those characters and replaces the scenery with high-definition painterly realistic backgrounds. Both exact prompts and the final atlas are retained here. Metadata records the original generated workspace files and each export's cell coordinates.

| Civilization | Final atlas | Character prompt | Background edit prompt | Metadata |
| --- | --- | --- | --- | --- |
| Bannor | [PNG](bannor.png) | [Text](bannor.txt) | [Text](bannor-background.txt) | [JSON](bannor.json) |
| Malakim | [PNG](malakim.png) | [Text](malakim.txt) | [Text](malakim-background.txt) | [JSON](malakim.json) |
| Elohim | [PNG](elohim.png) | [Text](elohim.txt) | [Text](elohim-background.txt) | [JSON](elohim.json) |
| Mercurians | [PNG](mercurians.png) | [Text](mercurians.txt) | [Text](mercurians-background.txt) | [JSON](mercurians.json) |
| Luchuirp | [PNG](luchuirp.png) | [Text](luchuirp.txt) | [Text](luchuirp-background.txt) | [JSON](luchuirp.json) |
| Kuriotates | [PNG](kuriotates.png) | [Text](kuriotates.txt) | [Text](kuriotates-background.txt) | [JSON](kuriotates.json) |
| Ljosalfar | [PNG](ljosalfar.png) | [Text](ljosalfar.txt) | [Text](ljosalfar-background.txt) | [JSON](ljosalfar.json) |
| Khazad | [PNG](khazad.png) | [Text](khazad.txt) | [Text](khazad-background.txt) | [JSON](khazad.json) |
| Sidar | [PNG](sidar.png) | [Text](sidar.txt) | [Text](sidar-background.txt) | [JSON](sidar.json) |
| Lanun | [PNG](lanun.png) | [Text](lanun.txt) | [Text](lanun-background.txt) | [JSON](lanun.json) |
| Grigori | [PNG](grigori.png) | [Text](grigori.txt) | [Text](grigori-background.txt) | [JSON](grigori.json) |
| Hippus | [PNG](hippus.png) | [Text](hippus.txt) | [Text](hippus-background.txt) | [JSON](hippus.json) |
| Amurites | [PNG](amurites.png) | [Text](amurites.txt) | [Text](amurites-background.txt) | [JSON](amurites.json) |
| Doviello | [PNG](doviello.png) | [Text](doviello.txt) | [Text](doviello-background.txt) | [JSON](doviello.json) |
| Balseraphs | [PNG](balseraphs.png) | [Text](balseraphs.txt) | [Text](balseraphs-background.txt) | [JSON](balseraphs.json) |
| Clan of Embers | [PNG](clan_of_embers.png) | [Text](clan_of_embers.txt) | [Text](clan_of_embers-background.txt) | [JSON](clan_of_embers.json) |
| Svartalfar | [PNG](svartalfar.png) | [Text](svartalfar.txt) | [Text](svartalfar-background.txt) | [JSON](svartalfar.json) |
| Calabim | [PNG](calabim.png) | [Text](calabim.txt) | [Text](calabim-background.txt) | [JSON](calabim.json) |
| Sheaim | [PNG](sheaim.png) | [Text](sheaim.txt) | [Text](sheaim-background.txt) | [JSON](sheaim.json) |
| Infernals | [PNG](infernal.png) | [Text](infernal.txt) | [Text](infernal-background.txt) | [JSON](infernal.json) |
| Illians | [PNG](illians.png) | [Text](illians.txt) | [Text](illians-background.txt) | [JSON](illians.json) |

Prepare a new character prompt with `python3 docs/minor-leaders/art_assets.py prompt docs/minor-leaders/concepts.json <civilization>`. `background-prompt` prepares the background edit. Image creation and edits use the imagegen tool; the script only prepares prompts and exports square cells through ImageMagick. Use `export` for an initial generated atlas and `export-background` for its final background revision.
