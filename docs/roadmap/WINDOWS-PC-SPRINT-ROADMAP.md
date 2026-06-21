# Windows PC Sprint Roadmap

**Status:** Active
**Scope:** Windows Runtime Node, portable Router, and future Windows Librarian client/app lane
**Current runtime-node baseline:** `c44150b`
**Last sealed sprint:** REDUCED-OFFLOAD-FIT-1

---

## Purpose

This roadmap defines the Windows PC work lane.

The Windows PC is not merely a test machine. It is a parallel execution lane for proving local runtime infrastructure, Router portability, and eventually a Windows version of The Librarian.

The work is divided into three layers:

| Layer | Focus |
|-------|-------|
| **Layer 1 — Runtime Node Reliability** | Make the Windows PC a dependable local advisory compute limb |
| **Layer 2 — Portable Router / Native Daemon** | Prevent Router behavior from becoming Windows-shaped |
| **Layer 3 — Windows Librarian Client/App** | Start a governed Owner-facing app on Windows |

---

## Architectural Distinctions

These must remain explicit in all planning and execution:

| Component | Role |
|-----------|------|
| **Runtime Node** | Local advisory compute limb. Not The Librarian. |
| **Router** | Portable runtime control contract (Python ref → Rust native) |
| **Windows Librarian** | Governed Owner-facing app/client on Windows |
| **Main Librarian core** | Custody, authority, receipts, validation, approval model |

> **Operating Principle:** The Windows Runtime Node is not The Librarian. It is an advisory compute limb. A Windows version of The Librarian must preserve custody, receipts, approval, validation, and Owner authority.

---

## Current Completed State

The Windows Runtime Node has proven the core local runtime path through 7 completed sprints.

### Layer 0 — Foundation (completed)

| Sprint | Status | Result |
|--------|--------|--------|
| RUNTIME-REPO-INIT-1 | Complete | Runtime-node repo initialized |
| WIN-SERVICE-LIFECYCLE-1 | Complete | Windows service + router lifecycle proved |
| WIN-BACKEND-SERVICE-PROOF-1 | Complete | Service-started router launched backend and cleaned up with no orphan |
| WIN-ROUTER-HARDEN-1 | Complete | Router endpoints and failure cases verified |
| WIN-MODEL-CONTEXT-FIT-2 | Complete | RX 570 context fit tested |
| ROUTER-PORTABILITY-1 | Complete | Portable Router contract documented |
| REDUCED-OFFLOAD-FIT-1 | Complete | Reduced GPU offload for OOM profiles verified (ngl=80, ctx=4096) |
| **WINDOWS-PC-PLAN-UPDATE-1** | **Complete** | Roadmap doc, startup sequence, sprint index established |

### Main Librarian Integration

| Sprint | Status | Result |
|--------|--------|--------|
| WIN-RUNTIME-INTEGRATION-1 | Sealed / Manual-audited | Main Librarian can represent Windows Runtime Node as optional advisory generation target |

**Validation caveat for WIN-RUNTIME-INTEGRATION-1:** `swift test` was not run in the Windows environment because the Swift toolchain was unavailable. Record as manual implementation audit PASS, pending future Swift harness verification on a capable environment.

---

## Layer 1 — Runtime Node Reliability

Goal: make the Windows PC a dependable local advisory compute limb.

### 1. WIN-RUNTIME-PROFILES-CLEANUP-1

**Purpose:** Make `config/model-profiles.json` reflect verified safe routing reality.

**Scope:**
- Mark phi-4 and qwen-coder as preferred/stable RX 570 profiles.
- Mark llama-3.2, qwen3, and gemma-3 according to reduced-offload results (ngl=80).
- Add explicit fields or comments if supported by schema: `verified_context`, `verified_ngl`, `stability`, `requires_reduced_offload`, `notes`.
- Avoid claiming unverified safety.

**Acceptance:**
- router still loads all profiles
- endpoint matrix still passes
- config reflects evidence
- no model binaries committed

**Expected artifacts:**
- updated `config/model-profiles.json`
- `docs/sprints/WIN-RUNTIME-PROFILES-CLEANUP-1.md`

### 2. WIN-RUNTIME-OPERATIONS-1

**Purpose:** Create a small operator toolkit for this Windows Runtime Node.

**Scope:** Add or document commands for:
- start service
- stop service
- restart service
- status
- logs
- profile list
- active backend
- clean orphan check
- port check
- service config dump

**Suggested scripts:**
- `scripts/runtime-status.ps1`
- `scripts/runtime-start.ps1`
- `scripts/runtime-stop.ps1`
- `scripts/runtime-logs.ps1`
- `scripts/runtime-clean-check.ps1`

**Acceptance:**
- scripts do not require unnecessary mutation
- scripts do not kill unrelated processes
- scripts preserve `LibrarianRunTimeNode` Manual startup policy
- scripts make support/debug easier

**Expected artifacts:**
- `docs/sprints/WIN-RUNTIME-OPERATIONS-1.md`
- optional scripts under `scripts/`

---

## Layer 2 — Portable Router / Native Daemon Evolution

Goal: prevent Router behavior from becoming Windows-shaped.

### 3. ROUTER-CONTRACT-TESTS-1

**Purpose:** Create shared conformance tests for any implementation of the portable Router contract.

**Applies to:**
- current Python reference router
- future Rust router
- future macOS/Linux wrappers

**Scope:** Test contract cases from `docs/architecture/ROUTER-PORTABILITY-CONTRACT.md`.

**Minimum cases:**
- start/status
- profiles
- health
- select
- chat
- restart
- unknown profile refusal
- invalid JSON refusal
- missing field refusal
- advisory-only enforcement
- no orphan after restart/stop

**Expected artifacts:**
- `docs/sprints/ROUTER-CONTRACT-TESTS-1.md`
- `tests/router-contract/` or equivalent
- optional PowerShell runner for Windows

**Acceptance:**
- current Python router passes the contract tests
- failures are reported as contract failures, not implementation quirks

### 4. ROUTER-RUST-CORE-1

**Purpose:** Plan and/or begin a native Router core implementation in Rust against the portable contract.

> **Important rule:** This is not "rewrite the Windows router." It is "implement the portable Router contract in a native cross-platform daemon."

**Scope options:**

*Planning-only version:*
- Rust crate structure
- API server choice
- config loader
- process manager abstraction
- OS wrapper boundary
- contract-test mapping

*Implementation version:*
- minimal Rust daemon with `/backend/status` and `/backend/profiles`
- no backend spawning yet unless explicitly scoped

**Acceptance:**
- does not replace Python router prematurely
- maps directly to portability contract
- keeps OS-specific service code thin

**Expected artifacts:**
- `docs/sprints/ROUTER-RUST-CORE-1.md`
- optional `rust/` or root Rust project skeleton, if implementation is approved

### 5. WIN-RUST-SERVICE-1

**Purpose:** Replace the current NSSM → PowerShell → Python service stack with a native Windows service wrapper only after Rust core is ready enough.

**Current proven path:**
```
Windows Service → NSSM → PowerShell launcher → Python router → llama-server
```

**Future target:**
```
Windows Service → native router daemon → llama-server
```

**Scope:**
- native service installation
- manual startup by default
- start/stop/restart proof
- backend child cleanup proof
- logs/receipts
- no orphan process after stop

**Acceptance:**
- feature parity with current NSSM proof
- no weaker lifecycle behavior
- rollback path preserved

**Expected artifacts:**
- `docs/sprints/WIN-RUST-SERVICE-1.md`
- native service code if approved

---

## Layer 3 — Windows Librarian Client/App Lane

Goal: start a Windows version of The Librarian as a governed Owner-facing app/client.

> **Important boundary:** The Windows Runtime Node is not The Librarian. The Runtime Node is a local advisory compute limb. The Windows Librarian app must preserve: Owner authority, custody, receipts, validation, governed actions, approval flow, and non-canonical model output boundaries.

### 6. WIN-LIBRARIAN-APP-PLAN-1

**Purpose:** Decide the architecture for a Windows version of The Librarian.

**Architecture options:**
- native Windows app
- .NET / WPF / WinUI app
- Tauri shell
- Electron shell
- local web app / localhost UI
- thin client to existing Librarian core

**Required decisions:**
- app shell technology
- shared core strategy
- repo strategy
- local storage strategy
- runtime-node integration boundary
- Owner UI model
- receipt and validation display model

**Acceptance:**
- no implementation until architecture is explicit
- runtime-node remains advisory-only
- custody/authority model remains intact

**Expected artifacts:**
- `docs/sprints/WIN-LIBRARIAN-APP-PLAN-1.md`
- `docs/architecture/WINDOWS-LIBRARIAN-APP.md`

### 7. WIN-LIBRARIAN-SHELL-1

**Purpose:** Create the first Windows app shell with no risky custody logic yet.

**Scope:**
- app launches
- shows placeholder navigation
- can show runtime-node status
- cannot mutate project files yet
- cannot approve or validate work yet

**Acceptance:**
- shell is stable
- runtime-node status is visible
- no authority-bearing actions implemented prematurely

### 8. WIN-LIBRARIAN-RUNTIME-INTEGRATION-1

**Purpose:** Connect Windows Librarian UI to the local runtime node as advisory compute only.

**Scope:**
- show available profiles
- select advisory runtime
- send prompt
- display advisory response
- show `[ADVISORY RUNTIME]` or structured advisory metadata
- no file writes
- no approval bypass

**Acceptance:**
- Owner can see runtime output
- runtime output is never canonical
- failure is non-fatal

### 9. WIN-LIBRARIAN-CUSTODY-UI-1

**Purpose:** Bring governed custody concepts into the Windows app UI.

**Scope:**
- project identity
- imported repo state
- receipts
- validation state
- Owner approval / rejection
- governed action packet display

**Acceptance:**
- UI actions map to governed action meanings
- no raw "button event" without canonical action semantics
- Owner authority preserved

---

## Recommended Execution Order

| Order | Sprint | Layer |
|-------|--------|-------|
| 1 | REDUCED-OFFLOAD-FIT-1 | **Done** |
| 2 | WIN-RUNTIME-PROFILES-CLEANUP-1 | Layer 1 |
| 3 | WIN-RUNTIME-OPERATIONS-1 | Layer 1 |
| 4 | ROUTER-CONTRACT-TESTS-1 | Layer 2 |
| 5 | ROUTER-RUST-CORE-1 | Layer 2 |
| 6 | WIN-RUST-SERVICE-1 | Layer 2 |
| 7 | WIN-LIBRARIAN-APP-PLAN-1 | Layer 3 |
| 8 | WIN-LIBRARIAN-SHELL-1 | Layer 3 |
| 9 | WIN-LIBRARIAN-RUNTIME-INTEGRATION-1 | Layer 3 |
| 10 | WIN-LIBRARIAN-CUSTODY-UI-1 | Layer 3 |

---

## Operating Principle

The PC lane should prove useful local capability without weakening the main Librarian authority model.

- **Runtime node work** proves compute.
- **Router work** proves portable runtime control.
- **Windows Librarian work** proves a governed Owner-facing app on Windows.
