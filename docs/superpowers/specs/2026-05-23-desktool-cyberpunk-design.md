# Desktool — Cyberpunk Off-Work Countdown Widget Design

## Purpose

An always-on-top Windows desktop widget that displays:
- **Off-work countdown** — counts down to off-work time (start time + 9 hours)
- **Emoji mood indicator** — changes based on how close to off-work time
- **Start/End work buttons** — clock in / clock out
- **System metrics** — CPU, memory, network speed with circular SVG ring gauges

Visual style: dark cyberpunk — neon green/cyan glow, monospace typography, deep black background.

## Technology Stack

**Tauri + HTML/CSS/JS + Rust**

- **Frontend**: Pure HTML/CSS/vanilla JS — no framework, no npm dependencies.
- **Backend**: Rust via Tauri. `sysinfo` crate for system metrics. `serde` for IPC serialization.
- **Runtime**: Windows WebView2 (built into Windows 11). Binary ~5MB.

## Visual Design

### Style: Dark Cyberpunk

| Element | Value |
|---------|-------|
| Background | Deep black `#08080F` |
| Border | `1px solid rgba(0,255,136,0.12)` |
| Primary accent | Neon green `#00FF88` — countdown, CPU gauge, network gauge, [上班] button |
| Secondary accent | Neon cyan `#00CCFF` — memory gauge |
| Danger accent | Red `#FF6B6B` — overtime state, [下班] button |
| Font | `'Consolas', 'Courier New', monospace` |
| Glow | `text-shadow: 0 0 20px rgba(0,255,136,0.5)`; `drop-shadow(0 0 5px)` on SVG circles |

### Layout (390 × 185 px)

```
┌──────────────────────────────────────────┐
│  🙂  │  OFF WORK IN            [上班]    │  ← Emoji 42px + countdown text + button
│      │  03:29:35                        │  ← 32px Bold, neon green
│      │  ON 09:20 → OFF 18:20           │  ← 9px dim
├──────────────────────────────────────────┤
│     (CPU)      (MEM)      (NET)         │  ← 3 SVG ring gauges, 60×60px
│      42%        62%        ↑1.2         │  ← Ring colors: green / cyan / green
│                            ↓5.8         │
│      CPU        MEM        NET          │  ← Labels 8px
│              9.9 / 15.9 GB              │  ← Memory detail, dim cyan
└──────────────────────────────────────────┘
```

- Size: **390 × 185 px** (fixed)
- Border radius: **12px**
- Padding: **16px top/bottom, 24px left/right**
- Gauge ring: **radius 26px, stroke 3px**, circumference **163.4**

### Emoji State Machine

| Emoji | Condition | Time Range (9h workday) |
|-------|-----------|------------------------|
| 😫 | > 4h remaining | Start → 5h elapsed |
| 😐 | 2–4h remaining | 5h → 7h elapsed |
| 🙂 | 1–2h remaining | 7h → 8h elapsed |
| 😆 | < 1h remaining | 8h → 9h elapsed |
| 😭 | Overtime (past off-work time) | > 9h elapsed (not yet clocked out) |
| 🎉 | Clocked out | After clicking [下班] |

### Button State Machine

| State | Button Shown | Action |
|-------|-------------|--------|
| Initial / After clock-out | `[上班]` (green) | Records current time as start. Off-work = now + 9h. |
| Counting down | None | Auto-tracked emoji stages |
| Overtime (past off-work) | `[下班]` (red) | Transitions to 🎉 "已下班" state |

### Countdown Logic

- Off-work time = start time + 9 hours (540 minutes)
- Start time persisted in localStorage → survives app restarts
- Clicking [上班] again (next day) overwrites previous start time

### Animations

- Ring gauge progress: `transition: stroke-dashoffset 0.4s ease`
- No animation on countdown text (monospace digits)

## Window Behavior

| Property | Value |
|----------|-------|
| Frameless | Yes (`decorations: false`) |
| Always on top | Yes (`alwaysOnTop: true`) |
| Taskbar | Hidden (`skipTaskbar: true`) |
| Resizable | No (fixed 390×185) |
| Transparent | Yes |
| Position | Bottom-right, 20px margin |
| Drag | Full surface draggable (`-webkit-app-region: drag`) |
| Right-click | Exit option |

## Architecture

### Frontend (`src/`)

```
src/
├── index.html    — Structure: countdown row + gauge row
├── styles.css    — All styling: glow, layout, animations, drag region
└── app.js        — Countdown timer, emoji logic, button handlers, Tauri IPC listener
```

**app.js responsibilities:**
- Countdown: `setInterval` every 1s → calculate remaining time → update DOM + emoji
- Button: [上班] records start time to localStorage; [下班] sets clocked-out state
- Emoji: determine correct emoji based on remaining time / overtime / clocked-out
- Gauges: listen for `metrics-updated` from Rust → update SVG rings
- Format: bytes → KB/s / MB/s human-readable
- Right-click: context menu → invoke Tauri exit

### Backend (`src-tauri/`)

```
src-tauri/
├── Cargo.toml       — tauri, sysinfo, serde, serde_json
├── tauri.conf.json  — window: 390×185, frameless, alwaysOnTop, skipTaskbar
└── src/main.rs      — 1s system monitor loop + Tauri commands
```

**main.rs responsibilities:**
- Background thread with 1-second interval
- Each tick: `refresh_cpu()` → `refresh_memory()` → `Networks::refresh()`
- Serialize `SystemMetrics` as JSON → emit `"metrics-updated"` to frontend
- Tauri command: window exit, window positioning on startup

### Data Flow

```
[Rust Background Thread, 1s]
  sysinfo reads CPU/MEM/NET
        │
        ▼
  SystemMetrics (serde JSON)
        │
        ▼
  window.emit("metrics-updated", payload)
        │
        ▼
[app.js event listener]
  Update SVG ring dashoffset
  Update network speed text
  Update memory detail text

[app.js setInterval, 1s]
  Read Date.now()
  Calculate remaining = offWorkTime - now
  Update countdown DOM
  Update emoji based on remaining
  Show/hide [上班]/[下班] buttons
```

## Data Model

```rust
#[derive(Serialize)]
struct SystemMetrics {
    cpu_percent: f32,       // 0.0–100.0
    mem_used_gb: f32,       // e.g. 9.9
    mem_total_gb: f32,      // e.g. 15.9
    mem_percent: f32,       // 0.0–100.0
    net_down_bps: f64,      // bytes per second download
    net_up_bps: f64,        // bytes per second upload
}
```

Frontend state (in `localStorage`):
```js
{
    startTime: 1716456000000,  // timestamp when [上班] clicked
    clockedOut: false           // true after [下班] clicked
}
```

## Window Configuration

```json
{
  "windows": [{
    "title": "Desktool",
    "width": 390,
    "height": 185,
    "resizable": false,
    "decorations": false,
    "alwaysOnTop": true,
    "skipTaskbar": true,
    "transparent": true
  }]
}
```

## Files to Create

| # | File | Purpose |
|---|------|---------|
| 1 | `src/index.html` | HTML: emoji + countdown row, gauge row, buttons |
| 2 | `src/styles.css` | CSS: layout, glow, SVG rings, drag, colors, animations |
| 3 | `src/app.js` | Countdown timer, emoji FSM, button logic, Tauri events, formatting |
| 4 | `src-tauri/Cargo.toml` | Rust: tauri, sysinfo, serde, serde_json |
| 5 | `src-tauri/tauri.conf.json` | Window: 390×185, frameless, alwaysOnTop, skipTaskbar |
| 6 | `src-tauri/src/main.rs` | System monitor loop (1s), Tauri commands, window positioning |
| 7 | `src-tauri/icons/` | App icon |
| 8 | `package.json` | Tauri project metadata |

## Verification

1. `cargo tauri dev` → widget appears bottom-right, always on top
2. Shows 🎉 "已下班" + [上班] button initially
3. Click [上班] → countdown starts → emoji changes over time
4. CPU/MEM/NET gauges update in real-time
5. Past off-work time → 😭 + red countdown + [下班] button appears
6. Click [下班] → 🎉 "已下班" + [上班] button returns
7. Drag widget anywhere
8. Right-click → Exit works
9. Close + reopen → start time persists (localStorage)
10. No taskbar icon

## Future (Out of Scope for v1)

- Customizable work hours (not just 9h default)
- Per-core CPU display
- Click-through mode
- Opacity adjustment
- Position persistence
- Color theme variants
- Network interface selector
