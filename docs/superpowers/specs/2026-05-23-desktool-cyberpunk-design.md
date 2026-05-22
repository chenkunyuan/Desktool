# Desktool — Cyberpunk Desktop Monitor Widget Design

## Purpose

A compact, always-on-top desktop widget for Windows that displays real-time system information (clock, CPU, memory, network speed) with a dark cyberpunk visual aesthetic: neon green/cyan glow, monospace typography, and circular SVG ring gauges.

## Technology Stack

**Tauri + HTML/CSS/JS + Rust**

- **Frontend**: Pure HTML/CSS/vanilla JS — no framework, no npm dependencies. SVG ring gauges, CSS glow effects, CSS transitions for animations.
- **Backend**: Rust via Tauri. `sysinfo` crate for system metrics. `serde` + `serde_json` for IPC serialization.
- **Runtime**: Windows WebView2 (built into Windows 11). Binary ~5MB, memory ~80MB.

## Visual Design

### Style: Dark Cyberpunk

| Element | Value |
|---------|-------|
| Background | Deep black `#08080F` |
| Panel | Semi-transparent `rgba(0,255,136,0.03)` with `1px solid rgba(0,255,136,0.12)` border |
| Primary accent | Neon green `#00FF88` — clock, CPU gauge, network gauge, glow shadows |
| Secondary accent | Neon cyan `#00CCFF` — memory gauge |
| Font | `'Consolas', 'Courier New', monospace` |
| Glow | `text-shadow: 0 0 20px rgba(0,255,136,0.5)` for text; `drop-shadow(0 0 8px)` for SVG circles |

### Layout

```
┌─────────────────────────────┐
│        14:30:25             │  ← Clock 36px Bold, neon green, center
│      2026-05-23 FRIDAY      │  ← Date 11px, dim
│                             │
│    (CPU)    (MEM)    (NET)  │  ← 3 SVG ring gauges, 70×70px each
│     42%      62%     ↑1.2  │  ← Ring color: green / cyan / green
│                      ↓5.8  │
│     CPU      MEM      NET   │  ← Labels 10px
│                             │
│       9.9 / 15.9 GB         │  ← Memory detail, dim cyan
│      ───────────────        │  ← Gradient divider
│       DESKTOOL v1.0         │  ← Footer, very dim
└─────────────────────────────┘
```

- Size: **~280 × 320 px** (fixed)
- Border radius: **12px**
- Padding: **24px horizontal, 22px vertical**
- Gauge ring: **radius 30px, stroke 4px**, circumference **188.5**

### Animations

- Ring gauge progress: `transition: stroke-dashoffset 0.4s ease` on SVG circle element
- No animation on clock text (avoids jitter with monospace digits)

### Gauge Ring Calculation

```
circumference = 2 * PI * 30 = 188.5
dashoffset = circumference * (1 - percent / 100)
```

## Window Behavior

| Property | Value |
|----------|-------|
| Frameless | Yes (`decorations: false` in tauri.conf.json) |
| Always on top | Yes (`alwaysOnTop: true`) |
| Taskbar | Hidden (`skipTaskbar: true`) |
| Resizable | No (fixed 280×320) |
| Transparent background | Yes (HTML body background handles appearance) |
| Initial position | Bottom-right corner of primary monitor, 20px margin |
| Drag | Entire window surface draggable (CSS `-webkit-app-region: drag`) |
| Context menu | Right-click → Exit |

## Architecture

### Frontend (`src/`)

```
src/
├── index.html    — Single-page structure: clock + 3 gauges + memory detail
├── styles.css    — All styling: fonts, colors, glow, layout, animations, drag region
└── app.js        — UI loop: listens for Tauri events, updates DOM, formats values
```

**app.js responsibilities:**
- Listen for `metrics-updated` event from Rust backend
- Update clock (`setInterval` every 1s for smooth second hand, or via backend event)
- Update gauge rings: recalculate `stroke-dashoffset` per ring
- Format bytes to human-readable (B/s → KB/s → MB/s)
- Handle right-click context menu → invoke Tauri exit command

**Zero frontend dependencies.** CSS glow and SVG rings need no libraries.

### Backend (`src-tauri/`)

```
src-tauri/
├── Cargo.toml       — dependencies: tauri, sysinfo, serde, serde_json
├── tauri.conf.json  — window config (size, position, decorations, alwaysOnTop)
└── src/main.rs      — system monitoring loop + Tauri commands
```

**main.rs responsibilities:**
- Initialize `sysinfo::System` once
- Spawn a background thread with 1-second interval
- On each tick:
  1. `System::refresh_cpu()` + read CPU usage %
  2. `System::refresh_memory()` → total/used in bytes
  3. `Networks::refresh()` → compute delta bytes/sec from previous tick
  4. Serialize metrics as JSON via `serde`
  5. Emit to frontend via `window.emit("metrics-updated", payload)`
- Register a Tauri command for app exit

### Data Flow

```
[Background Thread, 1s interval]
  sysinfo reads CPU/MEM/NET
        │
        ▼
  SystemMetrics struct (serde serialized)
        │
        ▼
  window.emit("metrics-updated", json)
        │
        ▼
[Frontend app.js]
  event listener receives payload
        │
        ▼
  Update DOM elements:
  - Clock text content
  - SVG circle stroke-dashoffset attributes
  - Network speed text content
  - Memory detail text content
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

## Window Configuration (tauri.conf.json)

```json
{
  "windows": [{
    "title": "Desktool",
    "width": 280,
    "height": 320,
    "resizable": false,
    "decorations": false,
    "alwaysOnTop": true,
    "skipTaskbar": true,
    "transparent": true,
    "center": false,
    "x": null, "y": null
  }]
}
```

Initial position calculation done in Rust on startup: read primary monitor work area, position at (right - width - 20, bottom - height - 20).

## Files to Create (8 core files)

| # | File | Purpose |
|---|------|---------|
| 1 | `src/index.html` | HTML structure: clock, 3 gauge sections, footer |
| 2 | `src/styles.css` | All CSS: layout, fonts, colors, glow effects, SVG ring styles, drag region |
| 3 | `src/app.js` | Frontend logic: Tauri event listener, DOM updates, formatting, right-click menu |
| 4 | `src-tauri/Cargo.toml` | Rust dependencies |
| 5 | `src-tauri/tauri.conf.json` | Window config, app identifier |
| 6 | `src-tauri/src/main.rs` | System monitor loop, Tauri commands, window positioning |
| 7 | `src-tauri/icons/` | App icon (minimal — a neon green square/dot) |
| 8 | `package.json` | Tauri frontend project metadata (minimal) |

## Verification

1. `cargo tauri dev` — widget appears bottom-right, always on top
2. Clock updates every second, digits don't jump (monospace)
3. CPU gauge ring moves when launching a CPU-intensive task
4. Memory gauge reflects Task Manager values
5. Network speeds show activity when downloading a file
6. Drag widget anywhere — cursor is `grab` over surface
7. Right-click → Exit closes cleanly
8. No taskbar icon, no Alt+Tab entry
9. Window stays on top of all other windows

## Future (Out of Scope for v1)

- Per-core CPU display
- Click-through mode toggle
- Opacity slider
- Position persistence (save to local storage or config file)
- Color theme variants (green-only, amber, red)
- Network interface selector (WiFi vs Ethernet)
- Compact mode toggle (show only clock + one gauge)
