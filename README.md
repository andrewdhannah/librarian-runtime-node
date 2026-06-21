# Librarian Runtime Node

**Local model runtime custody node for The Librarian.**

A governed local runtime node that hosts and supervises model backends for
The Librarian project. Provides profile selection, backend health monitoring,
chat proxying, process restart, and structured refusal behavior — all within
a strict advisory-only authority boundary.

---

## Current Status

**WIN-ROUTER-IMPL-1 — COMPLETE / ROUTING READY**

The Python router is the current behavioral reference implementation. It has
been verified across all 6 endpoint types and 8 verification cases. Two
critical runtime defects were found and fixed during implementation testing:
a deadlock in the health-check loop and a blocking-risk from single-threaded
request handling.

Current verified endpoints:

| Endpoint | Method | Behavior |
|----------|--------|----------|
| `/backend/status` | GET | Returns all profile states, active profile, counts |
| `/backend/profiles` | GET | Returns all 5 registered profiles with metadata |
| `/backend/health` | GET | Per-profile health, overall status |
| `/backend/select` (valid) | POST | Starts backend on assigned port |
| `/backend/select` (invalid) | POST | 403 `unknown_profile` structured refusal |
| `/backend/chat` (allowed) | POST | Proxies to backend, returns content |
| `/backend/chat` (authority) | POST | 403 `authority_required` structured refusal |
| `/backend/restart` | POST | Stops + starts backend, PID changes |

### Governance Principle

> Confidence was not allowed to become authority until runtime behavior was tested.

### Production Rule

> Python may prove the behavior. Rust should eventually own the production custody daemon.

---

## Runtime Profiles

Five model profiles are registered and verified at `ngl=99`, `context=1024`:

| Profile | Port | Size | Task Classes |
|---------|------|------|-------------|
| `phi-4` | 9120 | 2.32 GB | general_advisory, summarization_advisory |
| `qwen-coder` | 9121 | 1.76 GB | code_advisory, fallback_small_model |
| `llama-3.2` | 9122 | 2.16 GB | general_advisory |
| `qwen3` | 9123 | 2.33 GB | general_advisory, reasoning |
| `gemma-3` | 9124 | 2.32 GB | general_advisory |

**Current profile settings:**

| Profile | Context | ngl | Status |
|---------|---------|-----|--------|
| `phi-4` | 4096 | 99 | Verified safe |
| `qwen-coder` | 4096 | 99 | Verified safe |
| `llama-3.2` | 1024 | 99 | **OOM at load** — unstable at ngl=99 |
| `qwen3` | 1024 | 99 | **OOM at load** — unstable at ngl=99 |
| `gemma-3` | 1024 | 99 | **OOM at load** — unstable at ngl=99 |

For the three OOM profiles, `context: 1024` is a **legacy/default value**,
not a verified safe value at ngl=99. These profiles require reduced GPU
offload (lower `ngl`) to fit on the RX 570 4 GB. Larger context sizes
for verified profiles require explicit fit testing (see WIN-MODEL-CONTEXT-FIT-2).

---

## Architecture

```
┌─────────────────────────────────────────────┐
│              Router (Python)                  │
│  ┌─────────┐  ┌──────────┐  ┌────────────┐  │
│  │ Profile  │  │  Refusal │  │  Evidence   │  │
│  │ Registry │  │   Layer  │  │   Writer    │  │
│  └────┬─────┘  └────┬─────┘  └────────────┘  │
│       │             │                         │
│  ┌────▼─────────────▼─────────────────────┐  │
│  │       ProcessManager (per profile)       │  │
│  │  start │ health │ identity │ stop        │  │
│  └───────────────┬──────────────────────────┘  │
└──────────────────┼──────────────────────────────┘
                   │
    ┌──────────────┼──────────────┐
    ▼              ▼              ▼
 llama-server   llama-server   llama-server
 (phi-4:9120)  (qwen-coder:   (llama-3.2:
                  9121)         9122)
```

- **Router** — Python `ThreadingHTTPServer` serving 6 endpoint types
- **Model profiles config** — JSON schema defining alias, port, model path, ngl, context, task classes, authority
- **Process supervisor** — `ProcessManager` per profile: start, health poll, identity verify, restart, stop
- **Health checks** — Polls `/health` every 15s, transitions state: `starting → healthy → degraded → failed`
- **Refusal layer** — 8 structured refusal conditions (unknown_profile, authority_required, profile_unavailable, etc.)
- **Evidence fixtures** — Verified responses captured as JSON fixtures for regression testing
- **Future service lifecycle** — WIN-SERVICE-LIFECYCLE-1 will wrap the router as a Windows service

---

## Roadmap

| Phase | Sprint | Scope | Status |
|-------|--------|-------|--------|
| 1 | WIN-SERVICE-LIFECYCLE-1 | Windows service lifecycle (NSSM, auto-start, process custody) | 🡆 NEXT |
| 2 | WIN-ROUTER-HARDEN-1 | Python contract hardening (logging, timeouts, validation, regression) | Planned |
| 3 | WIN-ROUTER-RUST-1 | Rust reimplementation at behavioral parity with Python reference | Proposed |
| 4 | WIN-ROUTER-RUST-SERVICE-1 | Rust router as service-hosted default; Python retained as fallback | Proposed |

---

## Non-Goals

- Not the main Librarian application.
- Not a model repository or download manager.
- Not a general-purpose public inference server.
- Not an authority engine — does not grant approval, completion, or canonical status.
- Not a replacement for the Mac-hosted canonical Librarian repo.

---

## Safety / Custody Notes

- **Local-first runtime component.** This node serves the Librarian project
  but operates as an independent runtime.
- **Do not commit** generated logs, model files, indexes, or secrets to this repo.
  See `.gitignore` for the full exclusion policy.
- **Service installation must be owner-approved.** No Windows service mutation
  without explicit authorization (see WIN-SERVICE-LIFECYCLE-1 boundary).
- **Backend child processes must remain under router custody.** A known lifecycle
  gap exists: an orphan `llama-server.exe` was observed after earlier restart
  testing. This is a target for WIN-SERVICE-LIFECYCLE-1.
- **Config is local.** Copy `config/runtime-node.example.json` to
  `config/runtime-node.local.json` and edit for your environment. The local
  config file is gitignored.
