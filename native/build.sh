#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
native="$root/native"
build="$native/build"
output="$build/libjyoti-capture.so"

missing=()
for tool in cc pkg-config wayland-scanner; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    missing+=("$tool")
  fi
done
if [ ${#missing[@]} -gt 0 ]; then
  echo "build:native needs ${missing[*]} on PATH." >&2
  echo "Install a C compiler, pkg-config and wayland-scanner, then retry." >&2
  exit 1
fi

packages=(wayland-client egl gbm libdrm)
if ! flags="$(pkg-config --cflags --libs "${packages[@]}" 2>/dev/null)"; then
  echo "build:native needs the development packages: ${packages[*]}." >&2
  echo "On Arch: pacman -S wayland mesa libdrm" >&2
  exit 1
fi

mkdir -p "$build"
rm -f "$output"

generate() {
  local xml="$1"
  local name="$2"
  wayland-scanner client-header "$xml" "$build/$name-protocol.h"
  wayland-scanner private-code "$xml" "$build/$name-protocol.c"
}

generate "$native/protocol/wlr-screencopy-unstable-v1.xml" wlr-screencopy
generate "$native/protocol/xdg-output-unstable-v1.xml" xdg-output
generate "$native/protocol/linux-dmabuf-unstable-v1.xml" linux-dmabuf

# shellcheck disable=SC2086
cc -std=c11 -O2 -fPIC -shared -pthread -Wall -Wextra \
  -I "$build" -I "$native" \
  "$native/capture.c" \
  "$build/wlr-screencopy-protocol.c" \
  "$build/xdg-output-protocol.c" \
  "$build/linux-dmabuf-protocol.c" \
  -o "$output" $flags -lm

echo "built ${output#"$root"/}"
