# Session Handoff — Librarian Runtime Node

> Quick summary for an agent or human picking up where the last session left off.
> Root: `G:\OpenWork\librarian-runtime-node\`
> Updated: 2026-06-20

## Repo Identity

- **Project:** Librarian Runtime Node
- **Local path:** `G:\OpenWork\librarian-runtime-node\`
- **Remote:** `https://github.com/andrewdhannah/librarian-runtime-node.git`
- **Description:** Local model runtime custody node for The Librarian.

## Current Status

**8 completed sprints — foundation proven, roadmap established.**

### Completed Sprints (in order)

| # | Sprint | Result |
|---|--------|--------|
| 1 | RUNTIME-REPO-INIT-1 | Repo initialized |
| 2 | WIN-SERVICE-LIFECYCLE-1 | Windows service + router lifecycle proved |
| 3 | WIN-BACKEND-SERVICE-PROOF-1 | Service-started router launches backend, cleans up |
| 4 | WIN-ROUTER-HARDEN-1 | Router endpoints + failure cases verified |
| 5 | WIN-MODEL-CONTEXT-FIT-2 | RX 570 context fit tested |
| 6 | ROUTER-PORTABILITY-1 | Portable Router contract documented |
| 7 | REDUCED-OFFLOAD-FIT-1 | All 5 profiles verified at 4096 context on RX 570 |
| 8 | **WINDOWS-PC-PLAN-UPDATE-1** | Roadmap, startup sequence, sprint index established |

### Profile Verification Summary (RX 570 4 GB)

| Profile | Context | ngl | Status |
|---------|---------|-----|--------|
| phi-4 | 4096 | 99 | Verified safe |
| qwen-coder | 4096 | 99 | Verified safe |
| llama-3.2 | 4096 | 80 | Verified safe (reduced offload) |
| qwen3 | 4096 | 80 | Verified safe (reduced offload) |
| gemma-3 | 4096 | 80 | Verified safe (reduced offload) |

## Key Files

| Path | Description |
|------|-------------|
| `router/router.py` | Python router implementation |
| `config/model-profiles.json` | Deployed profile config (5 profiles) |
| `config/runtime-node.example.json` | Template for local config |
| `config/runtime-node.local.json` | **Local config — gitignored** |
| `docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md` | Three-layer Windows PC sprint roadmap |
| `docs/operations/WINDOWS-AGENT-STARTUP-SEQUENCE.md` | Mandatory agent startup checklist |
| `docs/architecture/RUNTIME-NODE-ARCHITECTURE.md` | Component architecture and contracts |
| `docs/architecture/ROUTER-PORTABILITY-CONTRACT.md` | Portable Router contract definition |
| `docs/sprints/` | Sprint records (8 completed) |
| `scripts/test-reduced-offload-fit.ps1` | Reusable reduced-offload test harness |
| `fixtures/windows-runtime-node/router-impl/` | Verified endpoint evidence (JSON fixtures) |
| `.gitignore` | Exclusion policy for models, logs, secrets |

## Next Sprint

### WIN-RUNTIME-PROFILES-CLEANUP-1 — Clean up profile config with verified data

**Purpose:** Update `config/model-profiles.json` to reflect verified safe routing
reality from REDUCED-OFFLOAD-FIT-1.

**Key tasks:**
- Mark phi-4 and qwen-coder as preferred/stable RX 570 profiles.
- Mark llama-3.2, qwen3, and gemma-3 with verified ngl=80, context=4096.
- Add `verified_context`, `verified_ngl`, `stability`, `requires_reduced_offload` fields.
- Avoid claiming unverified safety.
- Verify router still loads all profiles and endpoint matrix passes.

**Alternative:** WIN-RUNTIME-OPERATIONS-1 (operator toolkit scripts) could be
done first if tooling is more urgent.

## Known Issues

1. **SSH key not configured.** The remote uses HTTPS. Push requires
   authentication credentials.

2. **Swift test cannot run on Windows.** The Swift toolchain is unavailable.
   Any WIN-RUNTIME-INTEGRATION-1 validation was manual audit only.

## Boundaries

- No NSSM install or service mutation without elevation and owner approval.
- No model downloads.
- No GGUF/safetensors/LLM binary files committed.
- `LibrarianRunTimeNode` remains Manual startup unless explicitly changed.
- Do not kill unrelated processes.
- Always run the [startup sequence](docs/operations/WINDOWS-AGENT-STARTUP-SEQUENCE.md)
  before modifying files.
