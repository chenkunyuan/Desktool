#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Networks, System};
use tauri::Manager;

#[derive(Clone, Serialize)]
struct SystemMetrics {
    cpu_percent: f32,
    mem_used_gb: f32,
    mem_total_gb: f32,
    mem_percent: f32,
    net_down_bps: f64,
    net_up_bps: f64,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            mem_used_gb: 0.0,
            mem_total_gb: 0.0,
            mem_percent: 0.0,
            net_down_bps: 0.0,
            net_up_bps: 0.0,
        }
    }
}

#[derive(Default)]
struct NetworkState {
    prev_down: u64,
    prev_up: u64,
    prev_time: Option<Instant>,
}

struct MetricsStore(Mutex<SystemMetrics>);

#[tauri::command]
fn get_metrics(store: tauri::State<MetricsStore>) -> SystemMetrics {
    store.0.lock().unwrap().clone()
}

#[tauri::command]
fn get_window_pos(window: tauri::Window) -> (i32, i32) {
    let pos = window.outer_position().unwrap_or(tauri::PhysicalPosition { x: 0, y: 0 });
    (pos.x, pos.y)
}

#[tauri::command]
fn set_window_pos(window: tauri::Window, x: i32, y: i32) {
    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
}

fn main() {
    tauri::Builder::default()
        .manage(MetricsStore(Mutex::new(SystemMetrics::default())))
        .invoke_handler(tauri::generate_handler![get_metrics, get_window_pos, set_window_pos])
        .setup(|app| {
            let window = app.get_window("main").unwrap();

            // Position at bottom-right of primary monitor
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

            // Inject drag fix (CSS + custom window drag via __TAURI_INVOKE__)
            let drag_fix_js = r#"
                (function init(){
                    if (document.readyState === 'loading') {
                        document.addEventListener('DOMContentLoaded', init);
                        return;
                    }
                    var s=document.createElement('style');
                    s.textContent='html,body{cursor:move!important}body:active{cursor:move!important}';
                    document.head.appendChild(s);

                    var invoke = window.__TAURI_INVOKE__;
                    if(typeof invoke!=='function')return;

                    var drag=null;
                    document.addEventListener('mousedown',function(e){
                        if(e.target.closest('#action-btn')||e.target.closest('.gauge-value'))return;
                        invoke('get_window_pos').then(function(p){
                            drag={wx:p[0],wy:p[1],mx:e.screenX,my:e.screenY};
                        }).catch(function(){});
                        e.preventDefault();
                    });
                    document.addEventListener('mousemove',function(e){
                        if(!drag)return;
                        invoke('set_window_pos',{
                            x:drag.wx+(e.screenX-drag.mx),
                            y:drag.wy+(e.screenY-drag.my)
                        }).catch(function(){});
                    });
                    document.addEventListener('mouseup',function(){drag=null;});
                })();
            "#;
            let _ = window.eval(drag_fix_js);

            let app_handle = app.handle();
            thread::spawn(move || {
                let mut sys = System::new_all();
                let mut networks = Networks::new_with_refreshed_list();
                sys.refresh_cpu_all();
                sys.refresh_memory();

                let mut net_state = NetworkState::default();
                {
                    let total_down: u64 = networks.iter().map(|(_, n)| n.received()).sum();
                    let total_up: u64 = networks.iter().map(|(_, n)| n.transmitted()).sum();
                    net_state.prev_down = total_down;
                    net_state.prev_up = total_up;
                    net_state.prev_time = Some(Instant::now());
                }

                loop {
                    thread::sleep(Duration::from_secs(1));

                    // CPU
                    sys.refresh_cpu_all();
                    let cpu: f32 = {
                        let cpus = sys.cpus();
                        if cpus.is_empty() {
                            0.0
                        } else {
                            cpus.iter().map(|p| p.cpu_usage()).sum::<f32>() / cpus.len() as f32
                        }
                    };

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
                        let elapsed = net_state
                            .prev_time
                            .map(|t| t.elapsed().as_secs_f64())
                            .unwrap_or(1.0);
                        let down = ((total_down.saturating_sub(net_state.prev_down)) as f64
                            / elapsed)
                            .max(0.0);
                        let up = ((total_up.saturating_sub(net_state.prev_up)) as f64 / elapsed)
                            .max(0.0);
                        net_state.prev_down = total_down;
                        net_state.prev_up = total_up;
                        net_state.prev_time = Some(Instant::now());
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

                    // Update gauges via JS eval (Tauri v1.8 event system unreliable)
                    if let Some(win) = app_handle.get_window("main") {
                        let js = format!(
                            r#"(function(){{
                                var C=2*Math.PI*26;
                                function sg(el,p){{var x=Math.min(Math.max(p,0),100);el.setAttribute('stroke-dasharray',C);el.setAttribute('stroke-dashoffset',C*(1-x/100));}}
                                var cf=document.getElementById('cpu-fill'),mf=document.getElementById('mem-fill'),nf=document.getElementById('net-fill');
                                if(cf){{
                                    sg(cf,{cpu});document.getElementById('cpu-value').innerHTML={cpu}.toFixed(0)+'<span class="unit">%</span>';
                                    sg(mf,{mem});document.getElementById('mem-value').innerHTML={mem}.toFixed(0)+'<span class="unit">%</span>';
                                    document.getElementById('mem-detail').textContent={memUsed}.toFixed(1)+' / '+{memTotal}.toFixed(1)+' GB';
                                    var np=Math.min(({netDown}+{netUp})/10000000*100,100);sg(nf,np);
                                    function fs(b){{if(b<1000)return Math.round(b)+' B/s';if(b<1000000)return (b/1000).toFixed(1)+' KB/s';if(b<1000000000)return (b/1000000).toFixed(1)+' MB/s';return (b/1000000000).toFixed(1)+' GB/s';}}
                                    document.getElementById('net-value').innerHTML='<span class="net-up">↑'+fs({netUp})+'</span><span class="net-down">↓'+fs({netDown})+'</span>';
                                }}
                            }})();"#,
                            cpu=metrics.cpu_percent,
                            mem=metrics.mem_percent,
                            memUsed=metrics.mem_used_gb,
                            memTotal=metrics.mem_total_gb,
                            netDown=metrics.net_down_bps,
                            netUp=metrics.net_up_bps
                        );
                        let _ = win.eval(&js);
                    }
                    // Also update managed state for invoke-based polling
                    if let Some(store) = app_handle.try_state::<MetricsStore>() {
                        *store.0.lock().unwrap() = metrics;
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
