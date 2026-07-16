# Halo — Lightweight AI Notetaking App

Building Halo: an open-source, fast, local-first AI notetaking app using Tauri + Rust + React.

Captures notes from meetings, lessons, lectures, conversations—online or offline.

## Architecture
- **Backend**: Rust in `src-tauri/` — core logic, file operations, AI integration
- **Frontend**: React + TypeScript in `src/` — UI, note editing, display
- **IPC**: Tauri bridge connects frontend→backend with type-safe commands

## Rules
- No stubs, no TODOs, no placeholder implementations
- Every function must be fully implemented and working
- Run tests after implementing: `npm run test`
- Fix any failures before finishing
- The pre-commit hook scans for secrets — do NOT bypass it with --no-verify
- At the end of each stint, if all tests pass:
  1. Stage all relevant files (NOT .env or secret files)
  2. Commit: `git commit -m "feat: [description]\n\nAutomated-By: Claude AI (Haiku 4.5) <noreply@anthropic.com>"`
  3. Push to origin master
- Output STINT_COMPLETE when fully done

## Code Style
- **Rust**: Type hints, async where needed, minimal comments, use `Result<T>` for errors
- **TypeScript**: Strict mode, interfaces for all types, no `any`
- Use existing libraries — don't reinvent wheels
- Handle errors at boundaries (file I/O, API calls)
- Keep it minimal — no unnecessary abstractions

## Development
- Run dev server: `npm run dev`
- Build: `npm run build`
- Type checking: `npm run typecheck`

## Security
- Never hardcode API keys, tokens, or credentials
- Use environment variables (see .env.example)
- The pre-commit hook blocks any detected secrets
