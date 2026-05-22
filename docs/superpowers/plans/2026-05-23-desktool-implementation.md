# Desktool Cyberpunk Widget — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Windows desktop widget showing off-work countdown (start+9h), emoji mood indicator, clock-in/out buttons, and real-time CPU/memory/network circular gauges with a dark cyberpunk neon aesthetic.

**Architecture:** Tauri v1 desktop app. Rust backend collects system metrics via `sysinfo` crate and emits them to a pure HTML/CSS/JS frontend via Tauri events. Frontend handles countdown timer, emoji state machine, and clock-in/out logic with localStorage persistence.

**Tech Stack:** Tauri v1, Rust (`sysinfo`, `serde`, `serde_json`), vanilla HTML/CSS/JS (zero npm dependencies)

---

### Task 1: Install Prerequisites & Scaffold Project

**Files:**
- Create: `package.json`
- Create: `src-tauri/Cargo.toml` (via `cargo tauri init`)
- Create: `src-tauri/tauri.conf.json` (via `cargo tauri init`)
- Create: `src-tauri/src/main.rs` (via `cargo tauri init`)

- [ ] **Step 1: Install Tauri CLI**

```bash
cargo install tauri-cli --version "^1.6"
```

- [ ] **Step 2: Create frontend scaffold**

```bash
mkdir -p src
```

- [ ] **Step 3: Create package.json**

```json
{
  "name": "desktool",
  "version": "1.0.0",
  "private": true,
  "scripts": {
    "tauri": "tauri"
  }
}
```

- [ ] **Step 4: Create minimal src/index.html stub for init**

```html
<!DOCTYPE html>
<html><head><meta charset="UTF-8"></head><body></body></html>
```

- [ ] **Step 5: Initialize Tauri**

```bash
cd D:/Claude/MyProjects/desktool/Desktool
cargo tauri init
```

When prompted:
- App name: `Desktool`
- Window title: `Desktool`
- Web assets location: `../src`
- Dev server URL: press Enter (skip)
- Dev server command: press Enter (skip)

- [ ] **Step 6: Verify scaffold structure**

Expected files created:
```
src-tauri/
├── Cargo.toml
├── tauri.conf.json
├── src/main.rs
├── icons/
├── build.rs
```

Run `ls src-tauri/src/` and `ls src-tauri/` to verify.

- [ ] **Step 7: Add sysinfo dependency**

Verify `src-tauri/Cargo.toml` has these dependencies:

```toml
[dependencies]
tauri = { version = "1", features = ["shell-open"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sysinfo = "0.31"
```

If any are missing, add them.

- [ ] **Step 8: Verify build**

```bash
cargo tauri build --debug 2>&1 | tail -20
```

Expected: Build succeeds with no errors. A `.exe` is produced in `src-tauri/target/debug/`.

- [ ] **Step 9: Commit**

```bash
git add package.json src/ src-tauri/ .gitignore
git commit -m "chore: scaffold Tauri project with sysinfo dependency"
```

---

### Task 2: Configure Tauri Window

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Rewrite tauri.conf.json**

Replace the generated content with:

```json
{
  "build": {
    "distDir": "../src",
    "devPath": "../src",
    "beforeDevCommand": "",
    "beforeBuildCommand": ""
  },
  "tauri": {
    "bundle": {
      "active": true,
      "identifier": "com.desktool.app",
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ]
    },
    "allowlist": {
      "all": false
    },
    "windows": [
      {
        "title": "Desktool",
        "width": 390,
        "height": 185,
        "resizable": false,
        "decorations": false,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "transparent": true,
        "center": false,
        "x": 0,
        "y": 0
      }
    ]
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "feat: configure frameless always-on-top window (390x185)"
```

---

### Task 3: Rust Backend — System Monitor Service

**Files:**
- Create: `src-tauri/src/main.rs` (overwrite generated)

- [ ] **Step 1: Write main.rs**

```rust
// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Networks, System};
use tauri::{Manager, Window};

#[derive(Clone, Serialize)]
struct SystemMetrics {
    cpu_percent: f32,
    mem_used_gb: f32,
    mem_total_gb: f32,
    mem_percent: f32,
    net_down_bps: f64,
    net_up_bps: f64,
}

#[derive(Default)]
struct NetworkState {
    prev_down: u64,
    prev_up: u64,
    prev_time: Option<Instant>,
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_window("main").unwrap();

            // Position window at bottom-right of primary monitor
            if let Some(monitor) = window.primary_monitor().unwrap() {
                let size = monitor.size();
                let scale = monitor.scale_factor();
                let win_w = 390.0 * scale;
                let win_h = 185.0 * scale;
                let margin = 20.0 * scale;
                window
                    .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                        x: ((size.width as f64) - win_w - margin) as i32,
                        y: ((size.height as f64) - win_h - margin) as i32,
                    }))
                    .unwrap();
            }

            let net_state = Arc::new(Mutex::new(NetworkState::default()));

            // Spawn system monitor thread
            let window_clone = window.clone();
            let net_state_clone = net_state.clone();
            thread::spawn(move || {
                let mut sys = System::new_all();
                let mut networks = Networks::new_with_refreshed_list();
                sys.refresh_cpu();
                sys.refresh_memory();

                // Initialize network state
                {
                    let mut ns = net_state_clone.lock().unwrap();
                    let total_down: u64 = networks.iter().map(|(_, n)| n.received()).sum();
                    let total_up: u64 = networks.iter().map(|(_, n)| n.transmitted()).sum();
                    ns.prev_down = total_down;
                    ns.prev_up = total_up;
                    ns.prev_time = Some(Instant::now());
                }

                loop {
                    thread::sleep(Duration::from_secs(1));

                    // CPU
                    sys.refresh_cpu();
                    let cpu = sys.global_cpu_usage();

                    // Memory
                    sys.refresh_memory();
                    let total_bytes = sys.total_memory();
                    let used_bytes = sys.used_memory();
                    let total_gb = total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    let used_gb = used_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    let mem_pct = if total_bytes > 0 {
                        (used_bytes as f32 / total_bytes as f32) * 100.0
                    } else {
                        0.0
                    };

                    // Network
                    networks.refresh();
                    let total_down: u64 = networks.iter().map(|(_, n)| n.received()).sum();
                    let total_up: u64 = networks.iter().map(|(_, n)| n.transmitted()).sum();

                    let (down_bps, up_bps) = {
                        let mut ns = net_state_clone.lock().unwrap();
                        let elapsed = ns
                            .prev_time
                            .map(|t| t.elapsed().as_secs_f64())
                            .unwrap_or(1.0);
                        let down = ((total_down.saturating_sub(ns.prev_down)) as f64 / elapsed)
                            .max(0.0);
                        let up =
                            ((total_up.saturating_sub(ns.prev_up)) as f64 / elapsed).max(0.0);
                        ns.prev_down = total_down;
                        ns.prev_up = total_up;
                        ns.prev_time = Some(Instant::now());
                        (down, up)
                    };

                    let metrics = SystemMetrics {
                        cpu_percent: cpu.max(0.0).min(100.0),
                        mem_used_gb: used_gb,
                        mem_total_gb: total_gb,
                        mem_percent: mem_pct.max(0.0).min(100.0),
                        net_down_bps: down_bps,
                        net_up_bps: up_bps,
                    };

                    let _ = window_clone.emit("metrics-updated", metrics);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: Verify compilation**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```

Expected: Compilation succeeds.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/main.rs src-tauri/Cargo.toml
git commit -m "feat: add Rust system monitor with CPU/memory/network metrics emission"
```

---

### Task 4: HTML Structure

**Files:**
- Create: `src/index.html` (overwrite stub)

- [ ] **Step 1: Write index.html**

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Desktool</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <div id="app">

    <!-- Top Row: Emoji + Countdown + Button -->
    <div id="countdown-row">
      <div id="emoji">🎉</div>
      <div id="countdown-text">
        <div id="countdown-label">STATUS</div>
        <div id="countdown-time">已 下 班</div>
        <div id="countdown-sub"></div>
      </div>
      <button id="action-btn">上 班</button>
    </div>

    <!-- Bottom Row: 3 Circular Gauges -->
    <div id="gauges-row">
      <!-- CPU -->
      <div class="gauge">
        <div class="gauge-ring">
          <svg viewBox="0 0 60 60">
            <circle class="track" cx="30" cy="30" r="26"/>
            <circle class="fill cpu-fill" id="cpu-fill" cx="30" cy="30" r="26"/>
          </svg>
          <div class="gauge-value" id="cpu-value">--<span class="unit">%</span></div>
        </div>
        <div class="gauge-label">CPU</div>
      </div>

      <!-- Memory -->
      <div class="gauge">
        <div class="gauge-ring">
          <svg viewBox="0 0 60 60">
            <circle class="track" cx="30" cy="30" r="26"/>
            <circle class="fill mem-fill" id="mem-fill" cx="30" cy="30" r="26"/>
          </svg>
          <div class="gauge-value" id="mem-value">--<span class="unit">%</span></div>
        </div>
        <div class="gauge-label">MEM</div>
      </div>

      <!-- Network -->
      <div class="gauge">
        <div class="gauge-ring">
          <svg viewBox="0 0 60 60">
            <circle class="track" cx="30" cy="30" r="26"/>
            <circle class="fill net-fill" id="net-fill" cx="30" cy="30" r="26"/>
          </svg>
          <div class="gauge-value" id="net-value">
            <span class="net-up">--</span><br><span class="net-down">--</span>
          </div>
        </div>
        <div class="gauge-label">NET</div>
      </div>
    </div>

    <!-- Memory detail -->
    <div id="mem-detail"></div>
  </div>

  <script src="app.js"></script>
</body>
</html>
```

- [ ] **Step 2: Verify HTML loads in browser**

Open `src/index.html` in a browser — should show a blank page. Check DevTools console for 404s on `styles.css` and `app.js` (expected, not created yet).

- [ ] **Step 3: Commit**

```bash
git add src/index.html
git commit -m "feat: add HTML structure with countdown row and gauge row"
```

---

### Task 5: CSS — Cyberpunk Visual Theme

**Files:**
- Create: `src/styles.css`

- [ ] **Step 1: Write styles.css**

```css
/* === Reset === */
* { margin: 0; padding: 0; box-sizing: border-box; }

html, body {
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent;
  font-family: 'Consolas', 'Courier New', monospace;
  -webkit-app-region: drag;
  user-select: none;
  cursor: grab;
}

body:active { cursor: grabbing; }

/* === Main Container === */
#app {
  background: #08080f;
  border-radius: 12px;
  border: 1px solid rgba(0, 255, 136, 0.12);
  padding: 16px 24px;
  width: 100%;
  height: 100%;
  box-shadow: 0 0 40px rgba(0, 255, 136, 0.06), 0 4px 20px rgba(0, 0, 0, 0.6);
}

/* === Countdown Row === */
#countdown-row {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 14px;
}

#emoji {
  font-size: 42px;
  flex-shrink: 0;
  width: 52px;
  text-align: center;
  line-height: 1;
}

#countdown-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

#countdown-label {
  font-size: 9px;
  color: rgba(0, 255, 136, 0.3);
  letter-spacing: 2px;
  margin-bottom: 2px;
}

#countdown-time {
  font-size: 32px;
  font-weight: bold;
  letter-spacing: 3px;
  color: #00ff88;
  text-shadow: 0 0 20px rgba(0, 255, 136, 0.5);
  line-height: 1;
}

#countdown-sub {
  font-size: 9px;
  color: rgba(0, 255, 136, 0.2);
  letter-spacing: 1px;
  margin-top: 2px;
  min-height: 12px;
}

#action-btn {
  flex-shrink: 0;
  background: rgba(0, 255, 136, 0.08);
  border: 1px solid rgba(0, 255, 136, 0.25);
  border-radius: 6px;
  color: #00ff88;
  font-family: 'Consolas', monospace;
  font-size: 13px;
  font-weight: bold;
  padding: 10px 18px;
  letter-spacing: 3px;
  cursor: pointer;
  white-space: nowrap;
  -webkit-app-region: no-drag;
  transition: background 0.2s, box-shadow 0.2s;
}

#action-btn:hover {
  background: rgba(0, 255, 136, 0.15);
  box-shadow: 0 0 16px rgba(0, 255, 136, 0.15);
}

#action-btn.overtime {
  background: rgba(255, 107, 107, 0.08);
  border-color: rgba(255, 107, 107, 0.3);
  color: #ff6b6b;
}

#action-btn.overtime:hover {
  background: rgba(255, 107, 107, 0.15);
  box-shadow: 0 0 16px rgba(255, 107, 107, 0.15);
}

#action-btn.hidden { display: none; }

/* Overtime color overrides */
.overtime-text #countdown-label { color: rgba(255, 107, 107, 0.5); }
.overtime-text #countdown-time {
  color: #ff6b6b;
  text-shadow: 0 0 20px rgba(255, 107, 107, 0.5);
}
.overtime-text #countdown-sub { color: rgba(255, 107, 107, 0.25); }

/* === Gauges Row === */
#gauges-row {
  display: flex;
  justify-content: space-around;
  text-align: center;
  padding: 0 24px;
}

.gauge { text-align: center; }

.gauge-ring {
  position: relative;
  width: 60px;
  height: 60px;
  margin: 0 auto;
}

.gauge-ring svg {
  position: absolute;
  inset: 0;
  transform: rotate(-90deg);
}

.track {
  fill: none;
  stroke: rgba(0, 255, 136, 0.06);
  stroke-width: 3;
}

.fill {
  fill: none;
  stroke-width: 3;
  stroke-linecap: round;
  transition: stroke-dashoffset 0.4s ease;
}

.cpu-fill {
  stroke: #00ff88;
  filter: drop-shadow(0 0 5px rgba(0, 255, 136, 0.6));
}

.mem-fill {
  stroke: #00ccff;
  filter: drop-shadow(0 0 5px rgba(0, 204, 255, 0.6));
}

.net-fill {
  stroke: #00ff88;
  filter: drop-shadow(0 0 5px rgba(0, 255, 136, 0.6));
}

.gauge-value {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  font-weight: bold;
  color: #00ff88;
  line-height: 1.2;
}

.mem-fill + .gauge-value,
.gauge-ring:has(.mem-fill) .gauge-value { color: #00ccff; }

/* Quick fix: use ID-based color override */
#mem-value { color: #00ccff !important; }

.gauge-value .unit { font-size: 8px; }

.net-up, .net-down {
  font-size: 9px;
  font-weight: bold;
  color: #00ff88;
  line-height: 1.3;
}

.gauge-label {
  font-size: 8px;
  color: rgba(0, 255, 136, 0.3);
  letter-spacing: 1px;
  margin-top: 2px;
}

/* === Memory Detail === */
#mem-detail {
  text-align: center;
  margin-top: 4px;
  font-size: 8px;
  color: rgba(0, 204, 255, 0.2);
  letter-spacing: 1px;
}

/* === Dimmed gauges (clocked out state) === */
#app.dimmed .gauge-ring { opacity: 0.4; }
#app.dimmed .gauge-label { color: rgba(0, 255, 136, 0.2); }
#app.dimmed #mem-detail { color: rgba(0, 204, 255, 0.15); }
```

- [ ] **Step 2: Commit**

```bash
git add src/styles.css
git commit -m "feat: add cyberpunk CSS theme with neon glow and SVG ring gauge styles"
```

---

### Task 6: JavaScript — Countdown Timer & Emoji State Machine

**Files:**
- Create: `src/app.js`

- [ ] **Step 1: Write app.js Part 1 — State and countdown logic**

```js
// === Constants ===
const WORK_HOURS = 9;
const CIRCUMFERENCE = 2 * Math.PI * 26; // ~163.4
const EMOJI_STATES = [
  { maxRemaining: Infinity, emoji: '😫', label: 'OFF WORK IN' },
  { maxRemaining: 4 * 3600,     emoji: '😐', label: 'OFF WORK IN' },
  { maxRemaining: 2 * 3600,     emoji: '🙂', label: 'OFF WORK IN' },
  { maxRemaining: 1 * 3600,     emoji: '😆', label: 'OFF WORK IN' },
];

// === State ===
let state = {
  startTime: null,      // timestamp when [上班] clicked
  offWorkTime: null,    // startTime + 9h
  clockedOut: false,    // true after [下班] clicked
};

// === DOM Refs ===
const el = {
  app: document.getElementById('app'),
  emoji: document.getElementById('emoji'),
  countdownLabel: document.getElementById('countdown-label'),
  countdownTime: document.getElementById('countdown-time'),
  countdownSub: document.getElementById('countdown-sub'),
  actionBtn: document.getElementById('action-btn'),
  cpuFill: document.getElementById('cpu-fill'),
  memFill: document.getElementById('mem-fill'),
  netFill: document.getElementById('net-fill'),
  cpuValue: document.getElementById('cpu-value'),
  memValue: document.getElementById('mem-value'),
  netValue: document.getElementById('net-value'),
  memDetail: document.getElementById('mem-detail'),
  countdownText: document.getElementById('countdown-text'),
};

// === Load persisted state ===
function loadState() {
  try {
    const saved = JSON.parse(localStorage.getItem('desktool_state'));
    if (saved && saved.offWorkTime) {
      state.offWorkTime = saved.offWorkTime;
      state.startTime = saved.startTime;
      state.clockedOut = saved.clockedOut || false;
    }
  } catch (_) {}
}

function saveState() {
  try {
    localStorage.setItem('desktool_state', JSON.stringify({
      startTime: state.startTime,
      offWorkTime: state.offWorkTime,
      clockedOut: state.clockedOut,
    }));
  } catch (_) {}
}

// === Countdown Logic ===
function getRemainingSeconds() {
  if (!state.offWorkTime) return null;
  return Math.max(0, Math.floor((state.offWorkTime - Date.now()) / 1000));
}

function formatHMS(totalSeconds) {
  const h = Math.floor(totalSeconds / 3600);
  const m = Math.floor((totalSeconds % 3600) / 60);
  const s = totalSeconds % 60;
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

function formatTime(ts) {
  const d = new Date(ts);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

function getEmoji(remainingSeconds) {
  for (const s of EMOJI_STATES) {
    if (remainingSeconds > s.maxRemaining) continue;
    return { emoji: s.emoji, label: s.label };
  }
  return { emoji: '😆', label: 'OFF WORK IN' };
}

// === UI Update ===
function updateUI() {
  const now = Date.now();

  // State: clocked out
  if (state.clockedOut) {
    el.emoji.textContent = '🎉';
    el.countdownLabel.textContent = 'STATUS';
    el.countdownLabel.style.color = 'rgba(0,255,136,0.3)';
    el.countdownTime.textContent = '已 下 班';
    el.countdownTime.style.color = '#00ff88';
    el.countdownTime.style.textShadow = '0 0 20px rgba(0,255,136,0.5)';
    el.countdownSub.textContent = '';
    el.countdownText.classList.remove('overtime-text');
    el.actionBtn.textContent = '上 班';
    el.actionBtn.className = '';
    el.actionBtn.classList.remove('hidden', 'overtime');
    el.app.classList.add('dimmed');
    return;
  }

  // State: no start time (fresh state)
  if (!state.offWorkTime) {
    el.emoji.textContent = '🎉';
    el.countdownLabel.textContent = 'STATUS';
    el.countdownLabel.style.color = 'rgba(0,255,136,0.3)';
    el.countdownTime.textContent = '已 下 班';
    el.countdownTime.style.color = '#00ff88';
    el.countdownTime.style.textShadow = '0 0 20px rgba(0,255,136,0.5)';
    el.countdownSub.textContent = '';
    el.countdownText.classList.remove('overtime-text');
    el.actionBtn.textContent = '上 班';
    el.actionBtn.className = '';
    el.actionBtn.classList.remove('hidden', 'overtime');
    el.app.classList.add('dimmed');
    return;
  }

  // Active countdown state
  el.app.classList.remove('dimmed');
  const remaining = getRemainingSeconds();

  // Overtime: past off-work time, not yet clocked out
  if (remaining <= 0 && !state.clockedOut) {
    const overtimeSeconds = Math.floor((now - state.offWorkTime) / 1000);
    const { emoji } = EMOJI_STATES[0]; // not used, overtime is 😭
    el.emoji.textContent = '😭';
    el.countdownLabel.textContent = 'OVERTIME';
    el.countdownLabel.style.color = 'rgba(255,107,107,0.5)';
    el.countdownTime.textContent = '+' + formatHMS(overtimeSeconds);
    el.countdownTime.style.color = '#ff6b6b';
    el.countdownTime.style.textShadow = '0 0 20px rgba(255,107,107,0.5)';
    el.countdownSub.textContent = 'SHOULD OFF AT ' + formatTime(state.offWorkTime);
    el.countdownSub.style.color = 'rgba(255,107,107,0.25)';
    el.countdownText.classList.add('overtime-text');
    el.actionBtn.textContent = '下 班';
    el.actionBtn.className = 'overtime';
    el.actionBtn.classList.remove('hidden');
    return;
  }

  // Normal countdown
  const { emoji, label } = getEmoji(remaining);
  el.emoji.textContent = emoji;
  el.countdownLabel.textContent = label;
  el.countdownLabel.style.color = 'rgba(0,255,136,0.3)';
  el.countdownTime.textContent = formatHMS(remaining);
  el.countdownTime.style.color = '#00ff88';
  el.countdownTime.style.textShadow = '0 0 20px rgba(0,255,136,0.5)';
  el.countdownSub.textContent =
    'ON ' + formatTime(state.startTime) + ' → OFF ' + formatTime(state.offWorkTime);
  el.countdownSub.style.color = 'rgba(0,255,136,0.2)';
  el.countdownText.classList.remove('overtime-text');
  el.actionBtn.classList.add('hidden');
}

// === Button Handler ===
el.actionBtn.addEventListener('click', function (e) {
  e.stopPropagation();
  if (state.clockedOut || !state.offWorkTime) {
    // Clock in
    const now = Date.now();
    state.startTime = now;
    state.offWorkTime = now + WORK_HOURS * 3600 * 1000;
    state.clockedOut = false;
    saveState();
    updateUI();
  } else if (!state.clockedOut) {
    // Clock out (only available during overtime)
    state.clockedOut = true;
    saveState();
    updateUI();
  }
});

// === Right-click context menu ===
document.addEventListener('contextmenu', function (e) {
  e.preventDefault();
  if (window.__TAURI__) {
    window.__TAURI__.process.exit(0);
  }
});

// === Initialize ===
loadState();
updateUI();
setInterval(updateUI, 1000);
```

- [ ] **Step 2: Commit**

```bash
git add src/app.js
git commit -m "feat: add countdown timer, emoji state machine, and clock-in/out logic"
```

---

### Task 7: JavaScript — System Metrics Gauge Updates

**Files:**
- Modify: `src/app.js` (append to existing)

- [ ] **Step 1: Append gauge update logic to app.js**

Add this code at the end of `app.js` (after the `setInterval(updateUI, 1000);` line):

```js
// === Gauge Helpers ===
function setGauge(fillEl, valEl, percent) {
  const offset = CIRCUMFERENCE * (1 - Math.min(Math.max(percent, 0), 100) / 100);
  fillEl.setAttribute('stroke-dasharray', CIRCUMFERENCE);
  fillEl.setAttribute('stroke-dashoffset', offset);
}

function formatSpeed(bps) {
  if (bps < 1000) return Math.round(bps) + ' B/s';
  if (bps < 1_000_000) return (bps / 1000).toFixed(1) + ' KB/s';
  if (bps < 1_000_000_000) return (bps / 1_000_000).toFixed(1) + ' MB/s';
  return (bps / 1_000_000_000).toFixed(1) + ' GB/s';
}

// === Tauri Event Listener ===
if (window.__TAURI__) {
  window.__TAURI__.event.listen('metrics-updated', function (event) {
    const m = event.payload;

    // CPU
    setGauge(el.cpuFill, el.cpuValue, m.cpu_percent);
    const cpuPct = m.cpu_percent.toFixed(0);
    el.cpuValue.innerHTML = cpuPct + '<span class="unit">%</span>';

    // Memory
    setGauge(el.memFill, el.memValue, m.mem_percent);
    const memPct = m.mem_percent.toFixed(0);
    el.memValue.innerHTML = memPct + '<span class="unit">%</span>';
    el.memDetail.textContent =
      m.mem_used_gb.toFixed(1) + ' / ' + m.mem_total_gb.toFixed(1) + ' GB';

    // Network
    const netPct = Math.min((m.net_down_bps + m.net_up_bps) / 10_000_000 * 100, 100);
    setGauge(el.netFill, el.netValue, netPct);
    el.netValue.innerHTML =
      '<span class="net-up">↑' + formatSpeed(m.net_up_bps) + '</span>' +
      '<br>' +
      '<span class="net-down">↓' + formatSpeed(m.net_down_bps) + '</span>';
  });
}

// Initialize gauge dasharrays
[CIRCUMFERENCE].forEach(function () {
  el.cpuFill.setAttribute('stroke-dasharray', CIRCUMFERENCE);
  el.cpuFill.setAttribute('stroke-dashoffset', CIRCUMFERENCE);
  el.memFill.setAttribute('stroke-dasharray', CIRCUMFERENCE);
  el.memFill.setAttribute('stroke-dashoffset', CIRCUMFERENCE);
  el.netFill.setAttribute('stroke-dasharray', CIRCUMFERENCE);
  el.netFill.setAttribute('stroke-dashoffset', CIRCUMFERENCE);
});
```

- [ ] **Step 2: Commit**

```bash
git add src/app.js
git commit -m "feat: add SVG gauge ring updates from Tauri system metrics events"
```

---

### Task 8: App Icons

**Files:**
- Create: `src-tauri/icons/` PNG files

- [ ] **Step 1: Generate minimal placeholder icons**

Use ImageMagick or a pixel editor to create a simple neon-green dot/square as the app icon. The minimal sizes needed:

```bash
# Generate a simple 32x32 green square PNG as placeholder
# If ImageMagick is not available, skip this step — Tauri will use defaults
```

For the plan, Tauri's default icons in `src-tauri/icons/` from `cargo tauri init` are sufficient for v1. No action needed unless custom icon desired.

- [ ] **Step 2: Commit (if icons changed)**

---

### Task 9: Build, Run & Verify

- [ ] **Step 1: Verify dev build compiles**

```bash
cargo tauri dev 2>&1 | head -30
```

Expected: Widget window appears at bottom-right of screen with cyberpunk styling.

- [ ] **Step 2: Manual verification checklist**

```
[ ] Widget appears bottom-right, always on top
[ ] Shows 🎉 "已下班" + [上班] button initially
[ ] Click [上班] → countdown starts, emoji 😫 (if > 4h to go)
[ ] CPU gauge ring animates, matches Task Manager
[ ] Memory gauge shows reasonable values with GB detail
[ ] Network gauge shows upload/download speeds
[ ] Right-click anywhere → app exits cleanly
[ ] Reopen app → start time persists (countdown continues)
[ ] No taskbar icon
[ ] Window is draggable anywhere on screen
```

- [ ] **Step 3: Verify release build**

```bash
cargo tauri build 2>&1 | tail -10
```

Expected: `.msi` installer produced in `src-tauri/target/release/bundle/msi/`.

- [ ] **Step 4: Commit and tag**

```bash
git add .
git commit -m "chore: final polish and verification"
git tag v1.0.0
```
