#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/../out/scenes"
mkdir -p "$out"

chromium --headless=new --disable-gpu --hide-scrollbars --no-sandbox \
  --window-size=1920,1080 --screenshot="$out/document.png" \
  "file://$here/document.html" >/dev/null 2>&1

chromium --headless=new --disable-gpu --hide-scrollbars --no-sandbox \
  --window-size=1920,1080 --screenshot="$out/editor.png" \
  "file://$here/editor.html" >/dev/null 2>&1

ffmpeg -y -v error \
  -f lavfi -i "testsrc2=size=1920x1080:rate=30:duration=30" \
  -vf "hue=h='360*t/7'" \
  -c:v libx264 -preset veryfast -pix_fmt yuv420p "$out/dynamic.mp4"

ffmpeg -y -v error \
  -f lavfi -i "color=c=0xC0392B:s=1920x1080:rate=30:duration=30" \
  -vf "hue=h='360*t/30',drawbox=x=1344:y=0:w=576:h=1080:color=0x303030:t=fill,drawbox=x=0:y=980:w=1920:h=100:color=0x101010:t=fill" \
  -c:v libx264 -preset veryfast -pix_fmt yuv420p "$out/drift.mp4"

ffmpeg -y -v error \
  -f lavfi -i "testsrc2=size=1920x1080:rate=30:duration=24" \
  -vf "eq=brightness=-0.42:saturation=0.45" \
  -c:v libx264 -preset veryfast -pix_fmt yuv420p "$out/dark.mp4"

ffmpeg -y -v error \
  -f lavfi -i "color=c=0xF5F5F0:s=1920x1080:rate=30:duration=3" \
  -f lavfi -i "color=c=0xC0392B:s=1920x1080:rate=30:duration=3" \
  -f lavfi -i "color=c=0x2980B9:s=1920x1080:rate=30:duration=3" \
  -f lavfi -i "color=c=0x27AE60:s=1920x1080:rate=30:duration=3" \
  -f lavfi -i "color=c=0x101018:s=1920x1080:rate=30:duration=3" \
  -filter_complex "[0:v][1:v][2:v][3:v][4:v]concat=n=5:v=1:a=0[v]" \
  -map "[v]" -c:v libx264 -preset veryfast -pix_fmt yuv420p "$out/steps.mp4"

identify "$out/document.png" "$out/editor.png"
for video in "$out"/*.mp4; do
  printf '%s ' "$(basename "$video")"
  ffprobe -v error -show_entries format=duration -of csv=p=0 "$video"
done
