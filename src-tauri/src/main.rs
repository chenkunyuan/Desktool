#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::sync::{Arc, Mutex};
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

            let net_state = Arc::new(Mutex::new(NetworkState::default()));

            let window_clone = window.clone();
            let net_state_clone = net_state.clone();
            thread::spawn(move || {
                let mut sys = System::new_all();
                let mut networks = Networks::new_with_refreshed_list();
                sys.refresh_cpu_all();
                sys.refresh_memory();

                // Seed network state
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
