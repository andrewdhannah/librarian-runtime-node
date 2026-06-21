# Windows Agent Startup Sequence

**Status:** Active
**Scope:** Windows PC agent sessions working on Librarian runtime-node or Windows Librarian tasks

---

## Purpose

This document defines the expected startup sequence for agents working on the Windows PC lane.

The goal is to match the discipline of the Mac startup sequence: every agent must establish repository state, environment state, sprint scope, and custody boundaries before making changes.

Eventually this should be handled by The Librarian directly. Until then, agents must run this sequence manually at the start of each sprint/session.

---

## Core Rule

No agent should modify files before completing startup inspection.

Startup inspection must answer:

- What repo am I in?
- What branch and HEAD am I on?
- Is the working tree clean?
- What sprint am I executing?
- What files are in scope?
- What services/processes are already running?
- What must not be touched?
- What validation command applies?

---

## Standard Windows Startup Checklist

### 1. Identify working directory

```powershell
Get-Location
```

**Expected runtime-node repo:**

```
G:\OpenWork\librarian-runtime-node
```

**Expected main Librarian repo:**

```
G:\OpenWork\TheLibrarian-main
```

If the path is unexpected, stop and report.

### 2. Record git state

```powershell
git status --short
git rev-parse --short HEAD
git branch --show-current
git log --oneline -5
```

**Rules:**
- If dirty, classify the dirty files before changing anything.
- Do not stash blindly.
- Do not commit unrelated changes.
- Do not mix sprint scopes.

### 3. Verify runtime-node service state

For runtime-node work:

```powershell
Get-Service LibrarianRunTimeNode -ErrorAction SilentlyContinue
```

Record:
- service exists or missing
- status
- startup type if available
- whether Manual startup is preserved

The service should remain **Manual** unless a sprint explicitly changes that policy.

### 4. Verify elevation when needed

For service control work:

```powershell
([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
  [Security.Principal.WindowsBuiltInRole]::Administrator
)
```

If this returns `False`, do not attempt service registration or service mutation.

### 5. Check active runtime processes

```powershell
Get-CimInstance Win32_Process |
  Where-Object {
    $_.CommandLine -like "*librarian-runtime-node*" -or
    $_.Name -eq "llama-server.exe"
  } |
  Select-Object ProcessId, ParentProcessId, Name, CommandLine
```

**Rules:**
- Do not kill unrelated Python, llama, OpenWork, browser, editor, shell, or system processes.
- Only terminate processes proven to belong to the runtime-node sprint.
- Record PIDs before and after lifecycle tests.

### 6. Check relevant ports

**Router port:**
```powershell
netstat -ano | findstr ":9130"
```

**Known backend ports:**
```powershell
netstat -ano | findstr ":9120"
netstat -ano | findstr ":9121"
netstat -ano | findstr ":9122"
netstat -ano | findstr ":9123"
netstat -ano | findstr ":9124"
```

**Interpretation:**
- `LISTENING` means active process owns the port.
- `TIME_WAIT` is normal TCP cleanup and is not by itself a failed cleanup.
- Record PID for any `LISTENING` entry.

### 7. Verify ignored binaries remain untracked

Runtime-node rule: `runtime/bin/nssm.exe` must remain ignored/untracked.

```powershell
git status --short --ignored
```

**Do not commit:**
- model binaries
- nssm.exe
- local logs
- generated backend logs
- secrets
- machine-local cache files

### 8. Inspect sprint-relevant docs

For runtime-node work, inspect recent sprint docs:

```powershell
Get-ChildItem docs\sprints | Sort-Object LastWriteTime -Descending | Select-Object -First 10
```

Common docs to review:
- `docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md`
- `docs/operations/WINDOWS-AGENT-STARTUP-SEQUENCE.md`
- `docs/architecture/ROUTER-PORTABILITY-CONTRACT.md`
- `docs/architecture/RUNTIME-NODE-ARCHITECTURE.md`
- `docs/sprints/<most-recent-sprint>.md`
- `SESSION-HANDOFF.md`, if present

### 9. Confirm sprint scope

Before changing files, state:

1. sprint name
2. starting HEAD
3. files likely to change
4. files explicitly out of scope
5. acceptance criteria
6. validation command

### 10. Validate after changes

Runtime-node validation depends on sprint type.

**Service/router lifecycle:**
```powershell
Start-Service LibrarianRunTimeNode
Start-Sleep -Seconds 8
Invoke-RestMethod http://127.0.0.1:9130/backend/status
Stop-Service LibrarianRunTimeNode
Start-Sleep -Seconds 8
```

**Router endpoint check:**
```powershell
Invoke-RestMethod http://127.0.0.1:9130/backend/status
Invoke-RestMethod http://127.0.0.1:9130/backend/profiles
Invoke-RestMethod http://127.0.0.1:9130/backend/health
```

**Main Librarian Swift validation:**
```powershell
swift test
```

If Swift is unavailable on Windows, record:
> Harness not run: Swift toolchain unavailable in current Windows environment.
> Manual audit performed.

Do not record as harness `PASS`.

---

## Closeout Requirements

Every Windows agent sprint must report:

| Field | Required |
|-------|----------|
| Repo path | Yes |
| Starting HEAD | Yes |
| Final HEAD | Yes |
| Branch | Yes |
| Files changed | Yes |
| Service state | Yes, if runtime-related |
| Process/orphan check | Yes, if runtime-related |
| Validation command | Yes |
| Validation result | Yes |
| Git status | Yes |
| Actual commit message | Yes |
| Sprint status | Yes |

**Sprint status** must be one of:
- `SEALED`
- `COMPLETE / DOCS ONLY`
- `MANUAL-AUDITED`
- `BLOCKED`
- `NEEDS FIX`

---

## Non-Negotiable Boundaries

Agents must not:

1. commit model binaries
2. commit `nssm.exe`
3. silently change service startup to `Automatic`
4. kill unrelated processes
5. mix unrelated dirty files into a sprint
6. treat model output as authority
7. bypass Owner approval
8. bypass validation
9. mark unrun harnesses as `PASS`
10. call legacy/default settings verified if tests showed OOM or failure

---

## Future Direction

The Librarian should eventually own this startup sequence directly.

**Target future behavior:**
1. Agent opens sprint.
2. The Librarian runs environment inspection.
3. The Librarian records repo/service/process state.
4. The Librarian builds a governed work packet.
5. Agent receives bounded task capsule.
6. Agent returns receipt.
7. The Librarian validates and records closeout.

Until that exists, this startup sequence is the manual operating contract.
