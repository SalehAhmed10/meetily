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
| 1 | Import Transcript | Sidebar button imports `.txt`, `.md`, `.srt`, `.vtt` (and `.docx`, row 7) as a meeting. Subtitle cue times kept, VTT `<v Speaker>` tags become `Speaker:`. Google Meet / Tactiq `.txt` exports: leading `MM:SS` / `H:MM:SS` / `[00:05]` times are read, notes header kept untimed, list lines join the speaker's turn | 7 parser and database tests; imported real Teams-style VTT and Meet TXT | Issue #902, PR #903 (open) |
| 2 | GPU summaries | Built-in summary model runs on the GPU (Vulkan). llama-helper asks llama.cpp (`llama_params_fit`) how many layers fit instead of guessing: 33/33 layers on the RTX 5050 | Same 11k-token request 101.6 s -> 14.8 s; contract meeting summary 791.6 s (CPU build) -> 44.8 s | Comment on issue #552; PR not opened (maintainers plan to replace llama-helper, see #683) |
| 3 | Live summary progress | Spinner replaced by stage (reading N tokens / writing), estimated % (capped at 95% until done), characters, elapsed time and time left. Finished summaries show "Generated in Xs" | Helper, sidecar, query and time-left tests; seen in the app | Fork only |
| 4 | No [00:00] on untimed lines | Lines without a time (imported notes headers) show no time label | Seen in the app | Fork only |
| 5 | Projects | Assign meetings to a client/project from the picker in the transcript toolbar; create, rename, delete (meetings are kept). Sidebar groups meetings in project folders with a count | 2 repository tests on real migrations, 2 grouping tests; migration run on a copy of the real database | Fork only |
| 6 | Client Scope Call template | Summary template: agreed scope, out of scope, decisions, open questions, budget/timeline, client and developer action items, follow-up email draft | Template validated against the app's schema | Not in the repo: `%APPDATA%\Meetily\templates\client_scope_call.json` (shared with the official app) |
| 7 | Import `.docx` | Teams and Google Meet (Google Doc) transcript downloads import directly: one line per paragraph, then the same time and speaker reading as `.txt`. `Name   0:03` header lines followed by the words become `Name:` with that time; files of `start --> end` cue timings are read as VTT cues, with the speaker line after each timing becoming `Name:`. Layouts are from memory of Teams/Meet exports; both were imported in the app from generated sample files, not yet from a real export | 4 tests (Word XML with entities, tables and breaks; a real zipped `.docx`; cue layout; speaker-header layout, and notes left alone) | Fork only |
| 8 | Auto-file into projects | A meeting with no project goes into the project named in its title, as a whole word, ignoring case, longest name first ("Acme Mobile kickoff" -> Acme Mobile), then the one earlier in the title. Runs when a meeting is recorded, imported or renamed, including the title the AI summary gives it, and when a project is created or renamed (only meetings naming that project). A meeting that has a project is never moved. Choosing "No project" is not remembered: a later rename or summary can file it again | 5 tests, including rename on a one-connection database (no deadlock) and the import path; checked in the app (renaming a meeting to "Acme kickoff" filed it into Acme) | Fork only |
| 9 | Project action items | Checklist icon on a sidebar project folder opens one list of the project's action items, collected from every completed summary: the Action Items table (owner, task, due, transcript time) or any list under a heading with "action item" in it. The owner comes from the table, the heading ("Client Action Items"), a subheading inside the section, or a bold "**Saleh:**" before the item; nested bullets join their item. Translated summaries are read from their English original. Grouped by meeting, newest first; tick to mark done, "Show done" brings them back. Ticks are stored in a new `action_item_status` table keyed by meeting and task text, so a regenerated summary with reworded tasks starts unticked | 6 Rust tests (real table shape incl. a quoted `|` in a cell, lists, subheadings and bold owners, translated fallback, duplicates per owner, ticks on the real migrations) and 3 frontend tests; reviewed by a read-only agent, its 7 proven issues fixed | Fork only |
| 10 | One-line sidebar titles | Meeting titles in the sidebar stay on one line with "..." and show in full on hover. The edit/delete buttons float over the row end on hover or keyboard focus instead of reserving a third of the width, which is what made titles wrap onto 3-4 lines. Focus rings on the sidebar and action-items controls; action-items heading no longer has a label above it | tsc, frontend tests, Impeccable design detector (no findings in changed rows); checked in the app | Fork only |
| 11 | Summary to PDF | PDF button in the summary toolbar: the summary (with unsaved edits) becomes an A4 page with the meeting title and date, tables kept; you choose where to save and Edge/Chrome prints it in the background. The page may load nothing (Content-Security-Policy, no images, browser started with DNS disabled), since the summary is model output from possibly imported text. If the background print writes no file, the in-app print window opens instead (choose Save as PDF) The "PDF saved" message has an Open button | 2 Rust tests (one renders a real PDF with Edge on this PC), HTML test | Fork only |
| 12 | Email follow-up | Email button opens your mail app with the summary's "Follow-up Email Draft" section (or the whole summary) as the body and the meeting title as subject; you add the recipient and send. The full text is also copied, since mail links cut off near 2,000 characters. Links now open through `rundll32 url.dll,FileProtocolHandler` instead of `cmd /C start`, which split URLs at `&` | 4 frontend tests (draft section, encoding of `&`, long-body cut) | Fork only |
| 13 | Project timeline, emails and stages | The project button on a sidebar folder opens a project page with two tabs. **Timeline**: meetings and emails in date order, grouped by stage ("Round 1", "Phase 2"); a stage chip on each row sets it, offering stages used before. **Action items**: the list from row 9. **Add email**: paste an email or load a saved .eml/.txt; subject, sender and date are read from its headers (incl. encoded subjects, quoted-printable/base64, multipart, Outlook's copied From/Sent/Subject block); it is stored as a meeting of kind email, so search, summaries and action items work on it. Migration adds `meetings.kind` and `meetings.stage` | 3 Rust tests (emails and stages on the real migrations, search snippet next to multi-byte characters), 9 frontend tests (email parsing, stage grouping); reviewed by a read-only agent, its 5 proven and 2 likely issues fixed | Fork only |

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
| 2026-11-03 | `d756acf` | feat(projects): picker inline in the transcript toolbar, folder icon and count badge in the sidebar; this build sheet |
| 2026-11-03 | `ef29f5e` | feat(import): `.docx` transcripts; feat(projects): auto-file meetings into the project named in their title |
| 2026-11-03 | `b32a5c6` | feat(projects): creating or renaming a project files existing meetings that name it; fix(import): `Name:` after the speaker in Teams cue layout |
| 2026-11-03 | `d3f357b`, `305b728` | feat(projects): action items page per project, ticked off and remembered |
| 2026-11-03 | `2331cf7` | polish(ui): one-line sidebar titles, floating row actions, focus rings |
| 2026-11-03 | `55acccf` | feat(summary): PDF export and email follow-up; fix: open links containing `&` on Windows; fix(projects): "Pending" due dates read as none |
| 2026-11-03 | `15a6480` | fix(security): PDF export page cannot fetch anything (CSP, images dropped, no DNS); found by the automated security review |
| 2026-11-03 | `d8fcc72` | feat(projects): project page with timeline, emails and stages |
| 2026-11-03 | `4b02d40` | fix: review of the timeline: pasted first line kept, picked date kept, attachments skipped, HTML entities decoded; transcripts without times keep their order; search no longer panics next to curly quotes/em dashes |
| 2026-11-03 | `7b6aa77` | fix(summary): PDF falls back to the in-app print window (Save as PDF) when the background Edge print writes no file; that happened in the installed app (exit 0, no output) but not when Edge was started any other way; cause not yet found, error now records it |
| 2026-11-03 | (this commit) | feat(summary): "Open" button on the PDF saved message (direct save confirmed working in the app) |

## Next

- **Ask a project:** ask questions across one client's meetings, answered by the local model with the call and timestamp each point came from. Needs Projects (done).
- **Project brief and next-round prep:** the local model reads a project's summaries and emails in order and writes status, stage outcomes and a prep checklist. Pairs with Ask a project (fresh session).
- Later: speaker names per project, meeting hours per project.
