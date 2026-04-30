// Copyright 2019-2024 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, fs::read, sync::OnceLock};

use tauri::{command, ipc::Response, path::BaseDirectory, AppHandle, Manager, Runtime};

#[derive(Clone, Copy)]
struct BenchConfig {
  bytes: Option<usize>,
  iterations: usize,
}

impl BenchConfig {
  fn get() -> Self {
    static CONFIG: OnceLock<BenchConfig> = OnceLock::new();
    *CONFIG.get_or_init(|| {
      let bytes = env::var("TAURI_BENCH_TRANSFER_BYTES")
        .ok()
        .and_then(|v| v.parse().ok());
      let iterations = env::var("TAURI_BENCH_TRANSFER_ITERATIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

      Self { bytes, iterations }
    })
  }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BenchConfigPayload {
  bytes: Option<usize>,
  iterations: usize,
  generated: bool,
}

fn generated_byte(index: usize) -> u8 {
  let mut value = (index as u32) ^ 0x9e37_79b9;
  value ^= value >> 16;
  value = value.wrapping_mul(0x7feb_352d);
  value ^= value >> 15;
  value = value.wrapping_mul(0x846c_a68b);
  value ^= value >> 16;
  (value as u8) | 1
}

fn generated_payload(size: usize) -> Vec<u8> {
  static PAYLOAD: OnceLock<Vec<u8>> = OnceLock::new();
  PAYLOAD
    .get_or_init(|| (0..size).map(generated_byte).collect())
    .clone()
}

#[command]
fn bench_config() -> BenchConfigPayload {
  let config = BenchConfig::get();
  BenchConfigPayload {
    bytes: config.bytes,
    iterations: config.iterations,
    generated: config.bytes.is_some(),
  }
}

#[command]
fn app_should_close(
  exit_code: i32,
  total_bytes: Option<u64>,
  duration_ms: Option<f64>,
  checksum: Option<u64>,
) {
  if let (Some(total_bytes), Some(duration_ms), Some(checksum)) =
    (total_bytes, duration_ms, checksum)
  {
    println!(
      "BENCH_TRANSFER total_bytes={total_bytes} duration_ms={duration_ms:.3} checksum={checksum}"
    );
  }
  std::process::exit(exit_code);
}

#[command]
async fn read_file<R: Runtime>(app: AppHandle<R>) -> Result<Response, String> {
  if let Some(size) = BenchConfig::get().bytes {
    return Ok(Response::new(generated_payload(size)));
  }

  let path = app
    .path()
    .resolve(".tauri_3mb.json", BaseDirectory::Home)
    .map_err(|e| e.to_string())?;
  let contents = read(path).map_err(|e| e.to_string())?;
  Ok(Response::new(contents))
}

fn main() {
  tauri::Builder::default()
    .setup(|app| {
      #[cfg(target_os = "macos")]
      app.set_activation_policy(tauri::ActivationPolicy::Accessory);
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      app_should_close,
      bench_config,
      read_file
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
