# Session Handoff — Librarian Runtime Node

> Quick summary for an agent or human picking up where the last session left off.
> Root: `G:\OpenWork\librarian-runtime-node\`
> Updated: 2026-06-22

## Repo Identity

- **Project:** Librarian Runtime Node
- **Local path:** `G:\OpenWork\librarian-runtime-node\`
- **Remote:** `https://github.com/andrewdhannah/librarian-runtime-node.git`
- **Description:** Local model runtime custody node for The Librarian.

## Companion Repo

- **TheLibrarian-main:** `G:\OpenWork\TheLibrarian-main\`
- **Description:** Mac-side Librarian app (Swift/macOS). Contains the Windows Runtime Node integration target and the integration proof + receipt writer/verifier.

## Runtime Node Roadmap

| Sprint | Status |
|--------|--------|
| WIN-RUNTIME-NETWORK-BOUNDARY-1 | ✅ **Sealed** (final commit `82b3a9d`) |
| WIN-RUNTIME-INTEGRATION-1 | ✅ **Sealed** (TheLibrarian-main commit `ecda805`) |
| WIN-RUNTIME-HARDEN-1A | ✅ **Sealed** (librarian-runtime-node commit `cef8581`) |
| WIN-RUNTIME-RECEIPTS-1 | ✅ **Sealed** (TheLibrarian-main commit `1e32002`, librarian-runtime-node hardened in `cef8581`) |
| **WIN-RUNTIME-SEAL-AND-ANTI-LOOP-1** | ← **Current sprint** (docs-only) |
| RUNTIME-NODE-QUALIFICATION-1 | ⏳ Future |

## Current State (repos)

### librarian-runtime-node
- **HEAD**: `cef8581` — `fix(router): harden Python runtime lifecycle and request handling` (WIN-RUNTIME-HARDEN-1A)
- **Working tree**: Clean (after this docs commit lands) ✅
- **Service**: `LibrarianRunTimeNode` is Stopped / Manual ✅
- **Port 9130**: Free ✅
- **llama-server orphans**: None ✅

### TheLibrarian-main
- **HEAD**: `1e32002` — `feat(runtime): add integration proof receipts and verifier` (WIN-RUNTIME-RECEIPTS-1)
- **Working tree**: Clean ✅
- **Existing integration**: `Sources/App/Services/WindowsRuntimeNodeGenerationBackend.swift` (advisory target, `GenerationBackend` protocol)
- **Integration proof**: `scripts/integration_proof.py` — emits JSON receipt to `receipts/runtime-integration/`
- **Receipt verifier**: `scripts/receipt_verifier.py` — 29 checks (schema, lifecycle, cleanup, secret-scan, evidence-derived overall)

## Receipt (current run)

- **Path**: `G:\OpenWork\receipts\runtime-integration\win-runtime-integration-2026-06-22T193351Z-qwen-coder.json`
- **Schema**: `win-runtime-receipt/v1` (declared at `receipts/runtime-integration/schema.json`)
- **Verifier result**: **29/29 checks passed** (good-receipt test)
- **Overall status**: `partial` — proof says all 7 authenticated endpoints passed and all 2 unauthorized checks returned 401, but the proof's cleanup check found 1 backend process still running after stop+retries (`backend_orphans_after_stop: 1`). This is honestly recorded, NOT edited to force pass.
- **Token safety**: temporary one-off token used for the proof; never persisted, logged, included in receipt, or committed.

## Process State (as of handoff)

| Process | Status |
|---------|--------|
| `rust-router.exe` | Not running ✅ |
| `llama-server.exe` | Not running ✅ |
| `python.exe` (router) | Not running ✅ |
| `LibrarianRunTimeNode` service | Stopped / Manual ✅ |

## Key Files

### Runtime Node
| Path | Description |
|------|-------------|
| `rust-router/src/` | Rust router source (core router) |
| `router/router.py` | Python router (fallback/reference) — now uses non-blocking lock pattern, explicit log path, 64KB body limit |
| `config/model-profiles.json` | 5 verified profiles (phi-4, qwen-coder, llama-3.2, qwen3, gemma-3) |
| `config/runtime-node.example.json` | Template for local config |
| `config/runtime-node.local.json` | **Local config — gitignored** |
| `docs/sprints/WIN-RUNTIME-NETWORK-BOUNDARY-1.md` | Network boundary sprint spec + closeout |
| `docs/operations/WINDOWS-AGENT-STARTUP-SEQUENCE.md` | Mandatory agent startup checklist + **anti-loop rules** |
| `scripts/test-rust-router-endpoints.ps1` | 15-test endpoint suite |
| `scripts/start-librarian-runtime-node.ps1` | Service launcher (Rust primary, Python fallback) |
| `.gitignore` | Exclusion policy for models, logs, secrets |

### TheLibrarian-main
| Path | Description |
|------|-------------|
| `Sources/App/Services/WindowsRuntimeNodeGenerationBackend.swift` | Existing Swift integration (advisory) |
| `scripts/integration_proof.py` | Governed Mac → Windows proof; emits JSON receipt to `receipts/runtime-integration/` |
| `scripts/receipt_verifier.py` | 29-check receipt verifier |
| `scripts/integration_proof_result.json` | Latest proof result (overwritten each run) |
| `G:\OpenWork\receipts\runtime-integration/schema.json` | Receipt schema v1 |
| `G:\OpenWork\receipts\runtime-integration/*.json` | Real receipt runs |

## Pre-Work Checklist (next session must run)

1. Verify both repos' current HEAD — do not assume `cef8581` / `1e32002`
2. Verify working tree is clean on both repos
3. Verify `LibrarianRunTimeNode` service is Stopped / Manual before testing
4. Verify port 9130 is free before testing
5. Verify no orphan `llama-server`, `rust-router`, or Python router processes
6. Do not weaken the network boundary
7. If auth/token setup blocks proof, generate a temporary local token (do not ask Owner to paste secrets)

## Network Boundary Policy (do not weaken)

- Default bind: `127.0.0.1`
- LAN bind requires explicit `ROUTER_HOST`
- Auth via `ROUTER_AUTH_TOKEN` / `ROUTER_REQUIRE_AUTH`
- No token leaks in logs
- Body size bounded by `ROUTER_MAX_BODY_BYTES` (Python router: 64KB)
- Receipts must not contain tokens, bearer strings, or authorization headers

## Known Issues

1. **SSH key not configured.** Remote uses HTTPS. Push requires authentication credentials.
2. **Swift test cannot run on Windows.** The Swift toolchain is unavailable. Any `TheLibrarian-main` integration work on Windows is manual audit / proof-script only.
3. **Ad-hoc `rust-router` orphan risk.** When running the router outside NSSM for a proof run, it must be killed after the proof. Always check after closing.

## Anti-Loop Rules (see `docs/operations/WINDOWS-AGENT-STARTUP-SEQUENCE.md`)

- Stop after two failed attempts at the same command, test, or code path.
- Do not broaden sprint scope after a timeout.
- Do not rewrite working code to fix unrelated failures.
- Before retrying, record: command, failure, hypothesis, smallest next action.
- If service state becomes ambiguous, restore first: stop service/router/backend, free port 9130, confirm no llama-server orphans.
- Never mutate both repos unless the sprint explicitly requires it.
- Never commit generated cache files (`__pycache__/`, `*.pyc`, `*.pyo`).
- Never sweep pre-existing dirty files into a sprint commit without explicit scope.
- If auth/token setup blocks proof, generate a temporary local token; do not ask the Owner to paste secrets.
- Receipts may report partial/fail; do not edit evidence to force pass.

## Boundaries

- No NSSM install or service mutation without elevation and owner approval.
- No model downloads.
- No GGUF/safetensors/LLM binary files committed.
- `LibrarianRunTimeNode` remains Manual startup unless explicitly changed.
- Do not kill unrelated processes.
- Always verify orphan processes before starting work.
- Network boundary must not be weakened.
- Anti-loop rules apply to all Windows/runtime sprints.
