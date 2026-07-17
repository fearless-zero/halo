# Halo

Open-source, lightweight AI notetaking app — like Granola, but local-first and private. Halo captures the audio of your meetings, lectures and conversations (online **or** offline), transcribes it on-device, and writes clean notes for you. Transcription and note-writing always run locally on your machine; when you're online, Halo can additionally research each note's topics on the web and fold the findings in.

Built with **Tauri** (Rust backend) + **React** + **TypeScript**.

## How it works

- **Audio capture** — records your microphone *and* your computer's system audio (the other participants) via [`cpal`], mixes them, and downsamples to 16 kHz mono.
- **Import existing recordings** — point Halo at audio files from a voice recorder, USB stick or SD card (WAV, MP3, M4A, FLAC, OGG, AAC, Opus — decoded with [Symphonia]). Import a whole day of classes at once and Halo processes each recording into its own note.
- **Transcription** — [whisper.cpp] (Whisper base, MIT) runs on-device.
- **Note writing** — [llama.cpp] serving **Qwen3-4B-Instruct** (Apache-2.0), a fully-open model, turns the transcript into structured notes using your chosen style, and detects the topic to title each note.
- **Web research** — when online, Halo researches the note's title and section topics on Wikipedia (keyless, fully-open content) and appends a sourced *Background & Research* section. Toggle it off to stay fully offline.
- **Storage** — notes live as plain JSON in your app-data directory. No database, no account.
- **Integrations** — Markdown export, Obsidian, clipboard, Notion, Slack, a generic webhook, and Google / Apple / Microsoft calendars (via secret iCal feeds, used to auto-title recordings with the meeting that's happening). Notion supports **smart routing** — set `route` to `auto` and each note is filed in the workspace database that best matches its topic. The integration layer is extensible.

Both inference engines are bundled with the app as sidecar binaries, and the models are downloaded once on first launch. After setup, transcription and note-writing work completely offline; web research and cloud integrations use the network only when enabled.

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
npm run coverage    # vitest with v8 coverage (100% enforced)
cargo test --manifest-path src-tauri/Cargo.toml --lib
bash scripts/coverage.sh   # full frontend + backend coverage, fails below 100%
```

### Coverage

The project holds **100% test coverage**. The frontend (Vitest v8) enforces
100% statements, branches, functions and lines. The backend enforces 100% line
coverage via `cargo +nightly llvm-cov`; the code that can't run in CI (the
desktop event loop, cpal hardware capture, sidecar subprocess spawning and the
OS clipboard) is excluded at the function level with
`#[cfg_attr(coverage_nightly, coverage(off))]` and is exercised on real
devices. HTTP integrations and the model downloader are covered with a mock
server (`wiremock`) and Tauri's mock runtime.

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
  - `audio.rs` — capture & mixing · `import.rs` — audio-file import (Symphonia) · `transcribe.rs` — whisper.cpp · `llm.rs` — llama.cpp
  - `models.rs` — model downloads · `storage.rs` — local notes · `integrations.rs` — exports & Notion routing · `research.rs` — web research
  - `commands.rs` — Tauri command surface
- `scripts/fetch-sidecars.sh` — builds the bundled inference engines
- `.github/workflows/` — CI (typecheck, tests, frontend build, backend compile on 3 OSes)

## IDE setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- JetBrains IDEs (IntelliJ IDEA, WebStorm, RustRover) with the Rust plugin

[`cpal`]: https://github.com/RustAudio/cpal
[whisper.cpp]: https://github.com/ggml-org/whisper.cpp
[llama.cpp]: https://github.com/ggml-org/llama.cpp
[Symphonia]: https://github.com/pdeljanov/Symphonia
