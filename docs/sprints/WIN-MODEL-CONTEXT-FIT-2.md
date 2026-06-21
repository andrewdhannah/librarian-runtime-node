# Sprint: WIN-MODEL-CONTEXT-FIT-2
## Goal: Determine safe context-size limits for Windows RX 570 runtime profiles.

### Environment
- **GPU**: AMD Radeon RX 570 (4 GB Polaris)
- **Runtime Node**: `LibrarianRunTimeNode` (Windows Service)
- **Router Port**: `9130`
- **Baseline Context**: `1024`
- **Baseline NGL**: `99`

### Test Matrix
For each profile, test context sizes: `1024` (Baseline), `2048`, `3072`, `4096`.

| Profile | 1024 | 2048 | 3072 | 4096 | Safe Cap |
| :--- | :---: | :---: | :---: | :---: | :---: |
| phi-4 | PASS | PASS | PASS | PASS | 4096 |
| qwen-coder | PASS | PASS | PASS | PASS | 4096 |
| llama-3.2 | OOM | OOM | OOM | OOM | 1024* |
| qwen3 | OOM | OOM | OOM | OOM | 1024* |
| gemma-3 | OOM | OOM | OOM | OOM | 1024* |

\* Baseline 1024 is a **legacy/default value**, not a verified safe cap at ngl=99. 
These models OOM'd at load even at context=1024. A reduced-offload fit test 
(e.g., REDUCED-OFFLOAD-FIT-1) is needed to determine actual safe operating points.

### Test Method
1. Start `LibrarianRunTimeNode` (or manual router for debugging).
2. For each profile/context:
   - Update `config/model-profiles.json` with target context.
   - Trigger `/backend/restart` for the profile.
   - Send chat request: "Reply with OK only."
   - Record: Launch success, Response success, Crash/OOM, Cleanup.
3. Stop service and verify no orphans.

### Results
- **phi-4**: Stable up to 4096.
- **qwen-coder**: Stable up to 4096.
- **llama-3.2 / qwen3 / gemma-3**: All encountered `ErrorOutOfDeviceMemory` (OOM) when attempting to load with `ngl=99` and `context=1024`.

### Final Recommendations
- Update `phi-4` and `qwen-coder` to `context: 4096`.
- Keep others at `context: 1024` as a legacy/default value, **not** a verified safe setting.
- `llama-3.2`, `qwen3`, and `gemma-3` OOM at ngl=99 even at context=1024. They require
  reduced GPU offload (lower `ngl`) and a separate fit test to establish actual safe limits.
