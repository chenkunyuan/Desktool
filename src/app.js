// === Constants ===
var WORK_HOURS = 9;
var CIRCUMFERENCE = 2 * Math.PI * 26; // ~163.36
var EMOJI_STATES = [
  { maxRemaining: Infinity, emoji: '😫', label: 'OFF WORK IN' },
  { maxRemaining: 4 * 3600,     emoji: '😐', label: 'OFF WORK IN' },
  { maxRemaining: 2 * 3600,     emoji: '🙂', label: 'OFF WORK IN' },
  { maxRemaining: 1 * 3600,     emoji: '😆', label: 'OFF WORK IN' }
];

// === State ===
var state = {
  startTime: null,
  offWorkTime: null,
  clockedOut: false
};

// === DOM Refs ===
var el = {
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
  countdownText: document.getElementById('countdown-text')
};

// === Persistence ===
function loadState() {
  try {
    var saved = JSON.parse(localStorage.getItem('desktool_state'));
    if (saved && saved.offWorkTime) {
      state.offWorkTime = saved.offWorkTime;
      state.startTime = saved.startTime;
      state.clockedOut = saved.clockedOut || false;
    }
  } catch (e) {}
}

function saveState() {
  try {
    localStorage.setItem('desktool_state', JSON.stringify({
      startTime: state.startTime,
      offWorkTime: state.offWorkTime,
      clockedOut: state.clockedOut
    }));
  } catch (e) {}
}

// === Helpers ===
function getRemainingSeconds() {
  if (!state.offWorkTime) return null;
  return Math.max(0, Math.floor((state.offWorkTime - Date.now()) / 1000));
}

function formatHMS(totalSeconds) {
  var h = Math.floor(totalSeconds / 3600);
  var m = Math.floor((totalSeconds % 3600) / 60);
  var s = totalSeconds % 60;
  return pad(h) + ':' + pad(m) + ':' + pad(s);
}

function pad(n) { return n < 10 ? '0' + n : '' + n; }

function formatTime(ts) {
  var d = new Date(ts);
  return pad(d.getHours()) + ':' + pad(d.getMinutes());
}

function getEmoji(remainingSeconds) {
  for (var i = EMOJI_STATES.length - 1; i >= 0; i--) {
    if (remainingSeconds <= EMOJI_STATES[i].maxRemaining) {
      return { emoji: EMOJI_STATES[i].emoji, label: EMOJI_STATES[i].label };
    }
  }
  return { emoji: '😆', label: 'OFF WORK IN' };
}

function formatSpeed(bps) {
  if (bps < 1000) return Math.round(bps) + ' B/s';
  if (bps < 1000000) return (bps / 1000).toFixed(1) + ' KB/s';
  if (bps < 1000000000) return (bps / 1000000).toFixed(1) + ' MB/s';
  return (bps / 1000000000).toFixed(1) + ' GB/s';
}

function setGauge(fillEl, percent) {
  var pct = Math.min(Math.max(percent, 0), 100);
  var offset = CIRCUMFERENCE * (1 - pct / 100);
  fillEl.setAttribute('stroke-dasharray', CIRCUMFERENCE);
  fillEl.setAttribute('stroke-dashoffset', offset);
}

// === UI Update ===
function updateUI() {
  var now = Date.now();

  // Clocked out
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
    el.app.classList.add('dimmed');
    return;
  }

  // No start time
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
    el.app.classList.add('dimmed');
    return;
  }

  // Active countdown
  el.app.classList.remove('dimmed');
  var remaining = getRemainingSeconds();

  // Overtime
  if (remaining <= 0 && !state.clockedOut) {
    var overtimeSeconds = Math.floor((now - state.offWorkTime) / 1000);
    el.emoji.textContent = '😭';
    el.countdownLabel.textContent = 'OVERTIME';
    el.countdownLabel.style.color = 'rgba(255,107,107,0.5)';
    el.countdownTime.textContent = '+' + formatHMS(overtimeSeconds);
    el.countdownTime.style.color = '#ff6b6b';
    el.countdownTime.style.textShadow = '0 0 20px rgba(255,107,107,0.5)';
    el.countdownSub.textContent = 'SHOULD OFF AT ' + formatTime(state.offWorkTime);
    el.countdownText.classList.add('overtime-text');
    el.actionBtn.textContent = '下 班';
    el.actionBtn.className = 'overtime';
    return;
  }

  // Normal countdown
  var mood = getEmoji(remaining);
  el.emoji.textContent = mood.emoji;
  el.countdownLabel.textContent = mood.label;
  el.countdownLabel.style.color = 'rgba(0,255,136,0.3)';
  el.countdownTime.textContent = formatHMS(remaining);
  el.countdownTime.style.color = '#00ff88';
  el.countdownTime.style.textShadow = '0 0 20px rgba(0,255,136,0.5)';
  el.countdownSub.textContent = 'ON ' + formatTime(state.startTime) + ' → OFF ' + formatTime(state.offWorkTime);
  el.countdownText.classList.remove('overtime-text');
  el.actionBtn.classList.add('hidden');
}

// === Button Handler ===
el.actionBtn.addEventListener('click', function (e) {
  e.stopPropagation();
  var now = Date.now();

  if (state.clockedOut || !state.offWorkTime) {
    // Clock in
    state.startTime = now;
    state.offWorkTime = now + WORK_HOURS * 3600 * 1000;
    state.clockedOut = false;
    saveState();
    updateUI();
  } else {
    // Clock out
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

// === Tauri Event Listener ===
if (window.__TAURI__) {
  window.__TAURI__.event.listen('metrics-updated', function (event) {
    var m = event.payload;

    // CPU
    setGauge(el.cpuFill, m.cpu_percent);
    el.cpuValue.innerHTML = m.cpu_percent.toFixed(0) + '<span class="unit">%</span>';

    // Memory
    setGauge(el.memFill, m.mem_percent);
    el.memValue.innerHTML = m.mem_percent.toFixed(0) + '<span class="unit">%</span>';
    el.memDetail.textContent = m.mem_used_gb.toFixed(1) + ' / ' + m.mem_total_gb.toFixed(1) + ' GB';

    // Network
    var netPct = Math.min((m.net_down_bps + m.net_up_bps) / 10000000 * 100, 100);
    setGauge(el.netFill, netPct);
    el.netValue.innerHTML =
      '<span class="net-up">↑' + formatSpeed(m.net_up_bps) + '</span>' +
      '<br>' +
      '<span class="net-down">↓' + formatSpeed(m.net_down_bps) + '</span>';
  });
}

// === Init Gauges ===
el.cpuFill.setAttribute('stroke-dasharray', CIRCUMFERENCE);
el.cpuFill.setAttribute('stroke-dashoffset', CIRCUMFERENCE);
el.memFill.setAttribute('stroke-dasharray', CIRCUMFERENCE);
el.memFill.setAttribute('stroke-dashoffset', CIRCUMFERENCE);
el.netFill.setAttribute('stroke-dasharray', CIRCUMFERENCE);
el.netFill.setAttribute('stroke-dashoffset', CIRCUMFERENCE);

// === Start ===
loadState();
updateUI();
setInterval(updateUI, 1000);
