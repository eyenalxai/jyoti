import { mkdirSync, rmSync } from "node:fs"
import path from "node:path"

const root = path.resolve(import.meta.dir, "..")
const native = path.join(root, "native")
const build = path.join(native, "build")
const source = path.join(native, "capture.c")
const output = path.join(build, "libjyoti-capture.so")

const requiredTools = ["cc", "pkg-config", "wayland-scanner"]
const missingTools = requiredTools.filter((tool) => Bun.which(tool) === null)
if (missingTools.length > 0) {
  console.error(`build:native needs ${missingTools.join(", ")} on PATH.`)
  console.error("Install a C compiler, pkg-config and wayland-scanner, then retry.")
  process.exit(1)
}

const packages = ["wayland-client", "egl", "gbm", "libdrm"]
const pkgConfig = Bun.spawnSync({
  cmd: ["pkg-config", "--cflags", "--libs", ...packages],
  stdout: "pipe",
  stderr: "pipe",
})
if (pkgConfig.exitCode !== 0) {
  const detail = pkgConfig.stderr.toString().trim()
  console.error(`build:native needs the development packages: ${packages.join(", ")}.`)
  if (detail.length > 0) {
    console.error(detail)
  }
  console.error("On Arch: pacman -S wayland mesa libdrm")
  process.exit(1)
}

const protocols: ReadonlyArray<readonly [string, string]> = [
  ["wlr-screencopy-unstable-v1.xml", "wlr-screencopy"],
  ["xdg-output-unstable-v1.xml", "xdg-output"],
  ["linux-dmabuf-unstable-v1.xml", "linux-dmabuf"],
]

mkdirSync(build, { recursive: true })
rmSync(output, { force: true })

const generated: string[] = []
for (const [file, name] of protocols) {
  const xml = path.join(native, "protocol", file)
  const header = path.join(build, `${name}-protocol.h`)
  const code = path.join(build, `${name}-protocol.c`)
  for (const [mode, target] of [
    ["client-header", header],
    ["private-code", code],
  ] as const) {
    const result = Bun.spawnSync({
      cmd: ["wayland-scanner", mode, xml, target],
      stdout: "pipe",
      stderr: "pipe",
    })
    if (result.exitCode !== 0) {
      console.error(`wayland-scanner failed for ${file}:\n${result.stderr.toString()}`)
      process.exit(1)
    }
  }
  generated.push(code)
}

const flags = pkgConfig.stdout.toString().trim().split(/\s+/u).filter(Boolean)
const compile = Bun.spawnSync({
  cmd: [
    "cc",
    "-std=c11",
    "-O2",
    "-fPIC",
    "-shared",
    "-pthread",
    "-Wall",
    "-Wextra",
    "-I",
    build,
    "-I",
    native,
    source,
    ...generated,
    "-o",
    output,
    ...flags,
    "-lm",
  ],
  stdout: "pipe",
  stderr: "pipe",
})
if (compile.exitCode !== 0) {
  console.error(compile.stderr.toString())
  process.exit(1)
}

console.log(`built ${path.relative(root, output)}`)
