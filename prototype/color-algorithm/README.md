# Prototype: choose the screen-sync colour algorithm (JYO-23)

Throwaway comparison for [JYO-23](https://linear.app/utbetdix/issue/JYO-23/choose-the-color-algorithm). It answers one question: which of three candidates turns a real screen frame into a bulb colour that feels like the screen, without turning a white document blue?

The artifact is [`report.html`](report.html): eight scenes, 262 captured frames, the three candidates, and the measured trade-offs. `report.json` is the raw data behind it.

## Candidates

- **Chroma-weighted OKLab dominant** — k-means (k = 5) on the OKLab grid, score `population^0.8 * (0.2 + 0.8 * min(C / 0.15, 1))`, clusters under 3% population dropped, winner's hue kept. Warm-started from the previous tick's centroids.
- **Saliency background** — least-salient decile of the frame averaged in OKLab, following the Philips/Thomson-style background reading.
- **Hue histogram** — 36 × 6 × 6 chroma-weighted histogram, mode bin, 450 ms EMA.

All three share the same neutral gate: if the 90th-percentile OKLCH chroma of the cropped frame is below 0.03, emit the bulb's temperature mode at 4000 K instead of an RGB colour.

## Run it

The capture path expects Hyprland and a headless output named `HEADLESS-1` at 1920 × 1080. It does not touch the visible monitors.

```sh
bun install
./scenes/make-scenes.sh                 # render document/editor pages, generate test videos
bun run src/capture.ts --scene movie    # one scene at 1.5 Hz into out/frames/
bun run src/analyze.ts                  # writes out/report.json
bun run src/report.ts                   # writes out/report.html
```

The movie scene reads `/home/ulezot/Downloads/kling_20260917_VIDEO_show_me_my_427_0.mp4`; the desktop scene reads a local T3 Code recording. Replace those paths to re-run elsewhere. `src/bulb-test.ts` is the one-shot device check; it restores 100% / 4000 K and never prints tokens.

## Result

Chroma-weighted OKLab dominant wins. Saliency background is rejected: on the slow-drift scene it locks onto the static grey bar and reports chroma 0.00 while the coloured field sweeps the hue wheel; on the movie it averages the dark background (41° mean hue error). Hue histogram is viable but second: it tracks the drift, is cheaper, but its 10° bins raise the movie's p90 hue error from 13° to 21° and it can sit on one bin through a dim scene.

The derived colour is emitted at fixed OKLab L = 0.65, with chroma capped at 0.13; the scene's mean OKLab lightness is published separately as a brightness signal for the UX ticket. The neutral fallback is 4000 K, with a 0.03 gate and a 0.045 exit threshold. The JYO-21 deadband (ΔE_OK 0.02) and the research smoothing constants (450 ms colour, 900 ms brightness) stand.

This branch is a prototype: it is not merged, and the production implementation belongs under `src/lib/screen/`.
