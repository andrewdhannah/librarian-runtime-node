# Session Handoff — Librarian Runtime Node

> Quick summary for an agent or human picking up where the last session left off.
> Root: `G:\openwork\librarian-runtime-node\`
> Updated: 2026-06-19

## Repo Identity

- **Project:** Librarian Runtime Node
- **Local path:** `G:\openwork\librarian-runtime-node\`
- **Remote:** `https://github.com/andrewdhannah/librarian-runtime-node.git`
- **Description:** Local model runtime custody node for The Librarian.

## Current Status

**WIN-ROUTER-IMPL-1: COMPLETE / ROUTING READY**

The Python router is the current behavioral reference implementation, verified
across 6 endpoint types × 8 verification cases. Two runtime defects were found
and fixed: a deadlock in `_check_health()` and a blocking-risk from single-threaded
`HTTPServer`. Full evidence at `fixtures/windows-runtime-node/router-impl/`.

**RUNTIME-REPO-INIT-1: COMPLETE** (this session)

The runtime node now has its own Git repository, README, architecture docs,
and sprint planning structure.

## Key Files

| Path | Description |
|------|-------------|
| `router/router.py` | Python router implementation |
| `config/model-profiles.json` | Deployed profile config (5 profiles) |
| `config/runtime-node.example.json` | Template for local config |
| `config/runtime-node.local.json` | **Local config — gitignored** |
| `docs/architecture/RUNTIME-NODE-ARCHITECTURE.md` | Component architecture and contracts |
| `docs/sprints/` | Sprint records |
| `fixtures/windows-runtime-node/router-impl/` | Verified endpoint evidence (JSON fixtures) |
| `.gitignore` | Exclusion policy for models, logs, secrets |

## Known Issues

1. **Orphan backend process.** PID 948 (`llama-server.exe`) was left running
   after prior restart testing. This is a known lifecycle-custody gap and a
   target for WIN-SERVICE-LIFECYCLE-1.

2. **SSH key not configured.** The remote uses HTTPS. Push requires
   authentication credentials.

## Next Sprint

### WIN-SERVICE-LIFECYCLE-1 — Windows Router Service Lifecycle (startup only)

**Boundary:** Do not install NSSM, do not modify Windows services, do not
change router behavior. This session inspects, verifies, and reports only.

**Startup prompt:**
1. Inspect current repo state, docs, service feasibility, and Windows runtime paths.
2. Verify working tree, HEAD, existing evidence, and router health.
3. Scope the smallest safe service lifecycle sprint.
4. Do not install NSSM, modify service state, or change files until the
   startup report is complete and owner-approved.

**Required inspections:**
- NSSM availability and feasibility for Python script hosting
- Windows `sc.exe` service creation options
- Child process group handling on Windows
- Config path stability under service-level execution
- Orphan process prevention strategy

## Open Structural Question

Before WIN-SERVICE-LIFECYCLE-1 mutates anything, the next session should
report on the runtime-repo relationship:

- **Option A:** Independent repo (current choice)
- **Option B:** Subfolder/submodule under the main Librarian repo
- **Option C:** Runtime-only folder with canonical docs mirrored from Mac repo

This was resolved as Option A for RUNTIME-REPO-INIT-1. The next session
should confirm or recommend a change.

## Boundaries

- No NSSM install or service mutation without owner approval.
- No model downloads.
- No GGUF/safetensors/LLM binary files committed.
- WIN-SERVICE-LIFECYCLE-1 is not complete — do not mark it as such.
