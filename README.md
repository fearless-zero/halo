# Halo

Open-source, lightweight AI notetaking app — like Granola, but fully local and private. Halo captures the audio of your meetings, lectures and conversations (online **or** offline), transcribes it on-device, and writes clean notes for you. Nothing leaves your machine: transcription and note-writing both run locally.

Built with **Tauri** (Rust backend) + **React** + **TypeScript**.

## How it works

- **Audio capture** — records your microphone *and* your computer's system audio (the other participants) via [`cpal`], mixes them, and downsamples to 16 kHz mono.
- **Transcription** — [whisper.cpp] (Whisper base, MIT) runs on-device.
- **Note writing** — [llama.cpp] serving **Qwen3-4B-Instruct** (Apache-2.0), a fully-open model, turns the transcript into structured notes using your chosen style.
- **Storage** — notes live as plain JSON in your app-data directory. No database, no cloud, no account.
- **Integrations** — Markdown export, clipboard, and Notion; the integration layer is extensible.

Both inference engines are bundled with the app as sidecar binaries, and the models are downloaded once on first launch. After setup, Halo works completely offline.

## Prerequisites

- [Node.js 20+](https://nodejs.org/)
- [Rust](https://www.rust-lang.org/tools/install)
- **git**, **cmake** and a C/C++ compiler (to build the bundled inference engines)
- macOS 13+, Windows 10+, or Linux

## Setup

```bash
npm install

# Build the bundled llama.cpp + whisper.cpp sidecars for your platform.
# (Enables Metal on macOS; CPU elsewhere. Run once.)
./scripts/fetch-sidecars.sh
```

On first launch the app downloads the two models (~150 MB Whisper + ~2.5 GB Qwen3-4B) behind a progress screen.

## Development

```bash
npm run dev
```

## Build

```bash
npm run build   # tauri build → native installer in src-tauri/target/release/bundle/
```

## Checks & tests

```bash
npm run typecheck   # tsc --noEmit
npm run test        # vitest
cargo check --manifest-path src-tauri/Cargo.toml
```

## System-audio capture — platform notes

Halo captures system audio through a loopback/monitor input device:

- **Linux** (PulseAudio/PipeWire): the `*.monitor` source is detected automatically.
- **Windows**: enable **Stereo Mix** (or use a loopback device); it is picked up automatically.
- **macOS**: install a virtual loopback device such as [BlackHole](https://github.com/ExistentialAudio/BlackHole) (or create an Aggregate Device). Native ScreenCaptureKit capture is a planned follow-up so no virtual device is needed.

Microphone-only capture works everywhere with no extra setup. Both sources are toggleable in Settings.

## Project structure

- `src/` — React + TypeScript frontend
  - `types.ts` / `ipc.ts` — the typed Tauri command contract
  - `store.tsx` — app state and the record → transcribe → generate flow
  - `screens/`, `components/` — UI
- `src-tauri/src/` — Rust backend
  - `audio.rs` — capture & mixing · `transcribe.rs` — whisper.cpp · `llm.rs` — llama.cpp
  - `models.rs` — model downloads · `storage.rs` — local notes · `integrations.rs` — exports
  - `commands.rs` — Tauri command surface
- `scripts/fetch-sidecars.sh` — builds the bundled inference engines
- `.github/workflows/` — CI (typecheck, tests, frontend build, backend compile on 3 OSes)

## IDE setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- JetBrains IDEs (IntelliJ IDEA, WebStorm, RustRover) with the Rust plugin

[`cpal`]: https://github.com/RustAudio/cpal
[whisper.cpp]: https://github.com/ggml-org/whisper.cpp
[llama.cpp]: https://github.com/ggml-org/llama.cpp
