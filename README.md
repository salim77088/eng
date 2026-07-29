<div align="center">

# LUMINA ENGINE

**A lightweight 2D/3D game engine with an integrated editor and an easy scripting language.**

[![CI](https://github.com/salim77088/eng/actions/workflows/ci.yml/badge.svg)](https://github.com/salim77088/eng/actions/workflows/ci.yml)
[![Release](https://github.com/salim77088/eng/actions/workflows/release.yml/badge.svg)](https://github.com/salim77088/eng/actions/workflows/release.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-1.97%2B-orange.svg)](https://www.rust-lang.org)

</div>

---

## What is Lumina?

Lumina is a small, fast, cross-platform game engine built in Rust. It ships as a
single binary that contains both the **runtime** and the **editor** — no separate
tools to install, no project setup wizards. Just run the binary and start building.

- **2D + 3D rendering** on top of `wgpu` (Vulkan / Metal / DX12 / WebGPU)
- **Integrated editor** built with `egui` — hierarchy, inspector, viewport, asset
  browser, and console
- **LuminaScript** — an easy, JS-like scripting language (powered by `rhai`) with
  hot-reload on file save
- **CPU particle system** with configurable emitters, color curves, and gravity
- **Audio** via `kira` (with a null fallback for headless environments)
- **ECS** via `hecs` — tiny, fast, no boilerplate
- **Cross-platform** — builds for Windows, Linux, and macOS from a single codebase

## Quick Start

### Prerequisites

- **Rust 1.97+** (install via [rustup](https://rustup.rs))
- **Linux**: `libasound2-dev` (for audio), and the usual X11/Wayland dev packages

### Build & Run

```bash
git clone https://github.com/salim77088/eng.git
cd eng
cargo run --release
```

The editor window opens with a demo scene: a textured cube on a ground plane,
a drifting 2D logo sprite, and a particle fountain. Press **Play** in the top
bar to enter play mode.

### Build Without Audio

On systems without ALSA dev headers (e.g. minimal CI containers):

```bash
cargo build --release --no-default-features
```

The engine runs with a no-op audio backend in this mode.

## LuminaScript

LuminaScript is a JavaScript-like language. Define `init()`, `update(dt)`, and
`shutdown()` functions in a `.lumi` file and the engine calls them at the right
times. Scripts hot-reload — edit the file, save, and the engine re-evaluates it
on the next frame.

```js
// demo.lumi
let phase = 0.0;

fn init() {
    print("LuminaScript ready");
}

fn update(dt) {
    phase += dt * 2.0;
    let x = 200.0 * sin(phase);
    // engine reads `x` back to position entities (v0.2 feature)
}
```

## Project Layout

```
lumina/
├── crates/
│   ├── lumina-core/       # Math, time, logging, ECS, input, scene
│   ├── lumina-graphics/   # wgpu renderer: 2D sprites, 3D meshes, cameras
│   ├── lumina-audio/      # kira backend + null fallback
│   ├── lumina-script/     # LuminaScript (rhai) with hot-reload watcher
│   ├── lumina-particles/  # CPU particle system
│   ├── lumina-editor/     # egui editor UI
│   └── lumina-engine/     # Main binary - ties everything together
├── assets/                # Default assets + demo script
└── .github/workflows/     # CI + release pipelines
```

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  Lumina Binary                   │
│  ┌───────────┐  ┌───────────┐  ┌─────────────┐ │
│  │  Editor   │  │  Runtime  │  │  Scripting  │ │
│  │  (egui)   │  │ (ECS+gfx) │  │ (LuminaScr) │ │
│  └─────┬─────┘  └─────┬─────┘  └──────┬──────┘ │
│        │              │               │         │
│        └──────────────┴───────────────┘         │
│                       │                         │
│              ┌────────┴────────┐                │
│              │   wgpu (GPU)    │                │
│              │  Vulkan/Metal/  │                │
│              │   DX12/WebGPU   │                │
│              └─────────────────┘                │
└─────────────────────────────────────────────────┘
```

## Download Releases

Pre-built binaries for Windows, Linux, and macOS are published on every tagged
release. See the [Releases page](https://github.com/salim77088/eng/releases).

## License

Dual-licensed under **MIT** or **Apache-2.0**, at your option. See
[`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE).

## Contributing

This is v0.1 — a foundation. The roadmap for v0.2 includes:

- Full glTF model import (currently minimal OBJ only)
- Per-texture sprite batching
- 2D physics (AABB collisions, raycasting)
- Editor viewport gizmos (move/rotate/scale)
- LuminaScript → entity control (spawn, move, destroy)
- Scene serialization (save/load `.lumina` scene files)
- Asset pipeline (texture atlasing, mesh baking)

Pull requests welcome. Open an issue first for major changes.
