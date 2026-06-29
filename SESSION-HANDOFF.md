# Session Handoff — Librarian Runtime Node

> Quick summary for an agent or human picking up where the last session left off.
> Root: `G:\OpenWork\librarian-runtime-node\`
> Updated: 2026-06-29

## Repo Identity

- **Project:** Librarian Runtime Node
- **Local path:** `G:\OpenWork\librarian-runtime-node\`
- **Remote:** `https://github.com/andrewdhannah/librarian-runtime-node.git`
- **Description:** Local model runtime custody node for The Librarian.

## Companion Repo

- **TheLibrarian-main:** `G:\OpenWork\TheLibrarian-main\`
- **Description:** Mac-side Librarian app (Swift/macOS). Contains the Windows Runtime Node integration target and the integration proof + receipt writer/verifier.

## Runtime Node Roadmap

For the full up-to-date roadmap see `docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md`.

**Current baseline:** librarian-runtime-node `08a8602`, TheLibrarian-main `1e32002`
**Last sealed sprint:** WIN-AGENT-HARNESS-ENV-BASELINE-1

| Sprint | Status |
|--------|--------|
| RUNTIME-REPO-INIT-1 | ✅ Done |
| WIN-SERVICE-LIFECYCLE-1 | ✅ Done |
| WIN-BACKEND-SERVICE-PROOF-1 | ✅ Done |
| WIN-ROUTER-HARDEN-1 | ✅ Done |
| WIN-MODEL-CONTEXT-FIT-2 | ✅ Done |
| ROUTER-PORTABILITY-1 | ✅ Done |
| REDUCED-OFFLOAD-FIT-1 | ✅ Done |
| WINDOWS-PC-PLAN-UPDATE-1 | ✅ Done |
| WIN-RUNTIME-INTEGRATION-1 | ✅ Done |
| WIN-RUNTIME-RECEIPT-CLEANUP-1 | ✅ Done |
| WIN-RUNTIME-RECEIPTS-2 | ✅ Done |
| WIN-RUNTIME-QUALIFICATION-1 | ✅ Done |
| WIN-RUNTIME-CONTROLLED-ACTIVATION-1 | ✅ Done |
| **WIN-AGENT-HARNESS-ENV-BASELINE-1** | **✅ Done (this sprint)** |
| **WIN-AGENT-HARNESS-PLAN-1** | **← Next** |

## Proof Chain Complete

The three-link runtime proof chain is sealed:

| Link | Evidence |
|------|----------|
| 1. Source HEAD proof | `e7cfe33` (runtime-node), `1e32002` (main) |
| 2. Artifact hash proof | SHA-256 `84EB797A715FDC4CDB634A85FDFEC5C81CB90F37FB700F6EA6C97576B9569DC9` |
| 3. Governed rebuild proof | Rebuild hash matches receipt; 38/38 qualification gate passed |

**Key infrastructure:**
- v2 receipt schema at `receipts/runtime-integration/schema-v2.json`
- 48-check receipt verifier at `scripts/verify-receipt.ps1`
- Integration proof v2 script at `scripts/run-integration-proof-v2.ps1`
- Qualification scripts at `scripts/run-runtime-qualification.ps1` and `scripts/verify-runtime-qualification.ps1`
- Full environment baseline at `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md`
- 8 findings recorded for follow-up sprints (see baseline report)

## Current State (repos)

### librarian-runtime-node
- **HEAD**: `08a8602` — `docs(sprint): close WIN-RUNTIME-CONTROLLED-ACTIVATION-1 — PROMOTE`
- **Working tree**: Clean ✅
- **Service**: `LibrarianRunTimeNode` is Stopped / Manual ✅
- **Port 9130**: Free ✅
- **llama-server orphans**: None ✅
- **Ahead of origin**: 20 commits (push pending)

### TheLibrarian-main
- **HEAD**: `1e32002` — `feat(runtime): add integration proof receipts and verifier`
- **Working tree**: Clean ✅
- **Existing integration**: `Sources/App/Services/WindowsRuntimeNodeGenerationBackend.swift` (advisory target, `GenerationBackend` protocol)
- **Integration proof**: `scripts/integration_proof.py` — emits JSON receipt to `receipts/runtime-integration/`
- **Receipt verifier**: `scripts/receipt_verifier.py` — 29 checks (legacy; v2 verifier is `scripts/verify-receipt.ps1`)

## Current Receipts

### Latest v2 Integration Receipt
- **Path:** `receipts/runtime-integration/win-runtime-integration-v2-20260622-232214-qwen-coder.json`
- **Schema:** `win-runtime-receipt/v2`
- **Overall:** `pass`
- **Artifact SHA-256:** `84EB797A715FDC4CDB634A85FDFEC5C81CB90F37FB700F6EA6C97576B9569DC9`
- **Cleanup:** listener_active=false, connectivity=refused, orphans=0

### Latest Qualification Record
- **Path:** `receipts/runtime-qualification/win-runtime-qualification-20260622-234015.json`
- **Gate:** 38/38 passed
- **Rebuild hash matches receipt:** true

### Historical v1 Receipts
- `receipts/runtime-integration/win-runtime-integration-2026-06-22T193351Z-qwen-coder.json` (overall=partial — reconciled)
- `receipts/runtime-integration/win-runtime-integration-2026-06-22T195000Z-qwen-coder-cleanup-proof.json` (overall=pass)

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
| `router/router.py` | Python router (fallback/reference) |
| `config/model-profiles.json` | 5 verified profiles (phi-4, qwen-coder, llama-3.2, qwen3, gemma-3) |
| `config/runtime-node.example.json` | Template for local config |
| `config/runtime-node.local.json` | **Local config — gitignored** |
| `docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md` | ✅ Updated through WIN-RUNTIME-QUALIFICATION-1 |
| `docs/sprints/*.md` | Individual sprint specs and closeouts |
| `docs/operations/WINDOWS-AGENT-STARTUP-SEQUENCE.md` | Mandatory agent startup checklist + **anti-loop rules** |
| `scripts/run-integration-proof-v2.ps1` | Automated lifecycle proof (v2 receipts) |
| `scripts/verify-receipt.ps1` | 48-check receipt verifier |
| `scripts/run-runtime-qualification.ps1` | Governed rebuild qualification |
| `scripts/verify-runtime-qualification.ps1` | Qualification gate verifier |
| `scripts/start-librarian-runtime-node.ps1` | Service launcher (Rust primary, Python fallback) |
| `scripts/test-rust-router-endpoints.ps1` | 15-test endpoint suite |
| `.gitignore` | Exclusion policy for models, logs, secrets |

### TheLibrarian-main
| Path | Description |
|------|-------------|
| `Sources/App/Services/WindowsRuntimeNodeGenerationBackend.swift` | Swift integration (advisory) |
| `scripts/integration_proof.py` | Governed Mac → Windows proof |
| `scripts/receipt_verifier.py` | 29-check v1 receipt verifier (legacy) |
| `scripts/integration_proof_result.json` | Latest proof result (overwritten each run) |

### Receipts
| Path | Description |
|------|-------------|
| `G:\OpenWork\receipts\runtime-integration/schema.json` | v1 schema (legacy) |
| `G:\OpenWork\receipts\runtime-integration/schema-v2.json` | v2 schema (active) |
| `G:\OpenWork\receipts\runtime-integration/*.json` | Integration receipts |
| `G:\OpenWork\receipts\runtime-qualification/*.json` | Qualification records |

## Pre-Work Checklist (next session must run)

1. Verify both repos' current HEAD — do not assume prior values
2. Verify working tree is clean on both repos
3. Verify `LibrarianRunTimeNode` service is Stopped / Manual before testing
4. Verify port 9130 is free before testing
5. Verify no orphan `llama-server`, `rust-router`, or Python router processes
6. Do not weaken the network boundary
7. If auth/token setup blocks proof, generate a temporary local token (do not ask Owner to paste secrets)
8. Refer to `docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md` for current sprint priority
9. Consult `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md` for full machine environment reference
10. Address or defer findings from baseline report (see Findings Register §24) before starting service-level work

## Next Sprint Specification

**WIN-AGENT-HARNESS-PLAN-1** — Create missing governing plan documents.

### Why this comes next
The baseline (WIN-AGENT-HARNESS-ENV-BASELINE-1) found 5 planning documents missing:
- `WIN-AGENT-HARNESS-PLAN.md`
- `WIN-CUSTODY-SANDBOX-MODEL.md`
- `WIN-HARNESS-PARITY-ROADMAP.md`
- `WIN-LIBRARIAN-HOST-OPTIONS.md`
- `WIN-SPRINT-SEQUENCE.md`

These are prerequisites for disciplined harness implementation. Without them, future PC work risks becoming ad hoc.

### Findings that influence the plan
- **F-001 (HIGH: C: drive 10 GB free)** — Should be classified as a gating risk before any model workload, long-running stability test, or large build/test cache. Does not block docs/planning.
- **F-007 (INFO: Win 10 22H2 past EOS)** — Document as operational risk. Does not block local Phase 0 planning.

### After WIN-AGENT-HARNESS-PLAN-1
Either **WIN-DISK-SPACE-RISK-TRIAGE-1** (clear C: drive risk) or **WIN-PACKET-VALIDATION-HOOK-1** (continue harness implementation), depending on priority.

### Suggested session prompt
See `docs/sprints/WIN-AGENT-HARNESS-ENV-BASELINE-1.md` for the full suggested prompt for the next session.

## Key Environment Facts (from Baseline)

| Fact | Value |
|------|-------|
| Host | DESKTOP-ISNJ51B — MSI MS-7751 |
| CPU | i5-3570K — 4 cores, 3.40 GHz |
| RAM | 24.3 GB |
| GPU | Radeon RX 570 4 GB (Vulkan) |
| C: free space | **10.2 GB (9.2%) — CRITICAL** |
| G: free space | 132.3 GB (28.5%) |
| Local IP | 192.168.0.158/24 (Wi-Fi) |
| PS version | 5.1 (no pwsh) |
| Python | 3.14.3 |
| Node | 24.14.0 |
| Rust | 1.96.0 |
| nssm | Not in PATH (available at `runtime/bin/nssm.exe`) |
| Elevation | Non-admin (service control requires elevation) |

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
4. **model-profiles.json metadata gaps.** Missing fields: `verified_context`, `verified_ngl`, `stability`, `requires_reduced_offload`, `notes`. Scheduled for WIN-RUNTIME-PROFILES-CLEANUP-1.

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
