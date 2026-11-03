# meetily-plus build sheet

Custom features in this fork of [Zackriya-Solutions/meetily](https://github.com/Zackriya-Solutions/meetily), what changed, and how each was verified.

- **Fork:** `SalehAhmed10/meetily`, daily branch `feat/import-transcript` (based on upstream `main`, v0.4.1)
- **Installed as:** `meetily-plus` in `%LOCALAPPDATA%\meetily-plus`, next to the official `meetily`
- **Data:** `%APPDATA%\com.saleh.meetily` (own database). `models\` is a junction to the official app's models, so nothing is downloaded twice
- **Rebuild:** `git fetch upstream && git merge upstream/main`, then `./frontend/build-plus.sh --install` (close the app first)
- **Regenerate the PDF:** `python docs/meetily-plus/make_pdf.py`

> Keeping this current: every feature or fix added to the fork gets a row in **Features** and an entry in **Change log**, in the same commit as the code, and the PDF is regenerated.

## Features

| # | Feature | What it does | Verified by | Upstream |
|---|---|---|---|---|
| 1 | Import Transcript | Sidebar button imports `.txt`, `.md`, `.srt`, `.vtt` as a meeting. Subtitle cue times kept, VTT `<v Speaker>` tags become `Speaker:`. Google Meet / Tactiq `.txt` exports: leading `MM:SS` / `H:MM:SS` / `[00:05]` times are read, notes header kept untimed, list lines join the speaker's turn | 7 parser and database tests; imported real Teams-style VTT and Meet TXT | Issue #902, PR #903 (open) |
| 2 | GPU summaries | Built-in summary model runs on the GPU (Vulkan). llama-helper asks llama.cpp (`llama_params_fit`) how many layers fit instead of guessing: 33/33 layers on the RTX 5050 | Same 11k-token request 101.6 s -> 14.8 s; contract meeting summary 791.6 s (CPU build) -> 44.8 s | Comment on issue #552; PR not opened (maintainers plan to replace llama-helper, see #683) |
| 3 | Live summary progress | Spinner replaced by stage (reading N tokens / writing), estimated % (capped at 95% until done), characters, elapsed time and time left. Finished summaries show "Generated in Xs" | Helper, sidecar, query and time-left tests; seen in the app | Fork only |
| 4 | No [00:00] on untimed lines | Lines without a time (imported notes headers) show no time label | Seen in the app | Fork only |
| 5 | Projects | Assign meetings to a client/project from the picker in the transcript toolbar; create, rename, delete (meetings are kept). Sidebar groups meetings in project folders with a count | 2 repository tests on real migrations, 2 grouping tests; migration run on a copy of the real database | Fork only |
| 6 | Client Scope Call template | Summary template: agreed scope, out of scope, decisions, open questions, budget/timeline, client and developer action items, follow-up email draft | Template validated against the app's schema | Not in the repo: `%APPDATA%\Meetily\templates\client_scope_call.json` (shared with the official app) |

## Fork-only setup (not features)

| Change | Why |
|---|---|
| `productName` `meetily-plus`, identifier `com.saleh.meetily` | Own install folder, uninstall entry and database; never overwrites the official app |
| Updater endpoint points at this fork | "Update" can no longer install the official build over this one |
| `createUpdaterArtifacts` off; signing disabled at build time | Both need upstream's private keys |
| `frontend/build-plus.sh`, `frontend/build-llama-helper-vulkan.bat` | One-command Windows build: libclang 18 pin, Vulkan helper built with Ninja, installer |

## Build environment (this PC)

| Need | Detail |
|---|---|
| Rust, CMake, VS 2022 Build Tools (C++), LLVM, Vulkan SDK | Installed with winget |
| libclang 18 | `C:\Users\LMAhm\tools\libclang-18` (PyPI `libclang==18.1.1`). LLVM 23's libclang breaks whisper-rs-sys bindings |
| Build output | `C:\cargo-target\meetily` (drive H: was full) |
| Ninja for the Vulkan helper | MSBuild runs llama.cpp's shader-generator steps out of order on this machine |
| Rust tests | Set `ORT_DYLIB_PATH` to `frontend/src-tauri/binaries/onnxruntime/onnxruntime.dll`, or 7 VAD tests load the old system ONNX Runtime |

## Change log

| Date | Commit | Change |
|---|---|---|
| 2026-11-03 | `9d00667` | feat: import transcripts (.txt/.md/.srt/.vtt) as meetings |
| 2026-11-03 | `9dca0fb` | chore(fork): ship as meetily-plus beside the official app |
| 2026-11-03 | `ab3d1c1`, `fbccbaa` | fix(import): whitespace-only cue separators, no title in logs; README note |
| 2026-11-03 | `11a0548` | fix(llama-helper): let llama.cpp fit GPU offload to free VRAM |
| 2026-11-03 | `36e5a3c` | chore(fork): build llama-helper with Vulkan via Ninja |
| 2026-11-03 | `f12e6a7` | feat(summary): live progress and "Generated in Xs" |
| 2026-11-03 | `15636a0` | fix(import): read leading timestamps in plain-text transcripts |
| 2026-11-03 | `93740a9` | fix(transcript): no [00:00] label on lines without a time |
| 2026-11-03 | `b6241d8` | feat(projects): group meetings by client/project |
| 2026-11-03 | (this commit) | feat(projects): picker inline in the transcript toolbar, folder icon and count badge in the sidebar; this build sheet |

## Next

- **Ask a project:** ask questions across one client's meetings, answered by the local model with the call and timestamp each point came from. Needs Projects (done).
