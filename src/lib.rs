mod types;

use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use nusb::hotplug::HotplugEvent;
use nusb::MaybeFuture;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub use types::{UsbDevice, UsbEvent, UsbEventType};

const DEFAULT_DEBOUNCE_MS: u32 = 500;

struct WatcherState {
  running: Arc<AtomicBool>,
  _handle: JoinHandle<()>,
}

static WATCHER: OnceLock<Mutex<Option<WatcherState>>> = OnceLock::new();

fn watcher_state() -> &'static Mutex<Option<WatcherState>> {
  WATCHER.get_or_init(|| Mutex::new(None))
}

/// List all currently connected USB devices.
#[napi]
pub fn list_devices() -> Result<Vec<UsbDevice>> {
  let iter = nusb::list_devices()
    .wait()
    .map_err(|e| Error::from_reason(e.to_string()))?;

  Ok(iter.map(|d| UsbDevice::from_device_info(&d)).collect())
}

/// Process a single HotplugEvent into the debounce buffer.
/// `id_to_key` maps device_id -> (debounce_key, serial_number) and persists
/// across windows so Disconnected events can resolve their serial.
fn process_event(
  event: HotplugEvent,
  pending: &mut HashMap<String, UsbEvent>,
  id_to_key: &mut HashMap<String, (String, Option<String>)>,
) {
  match event {
    HotplugEvent::Connected(info) => {
      let device_id = format!("{:?}", info.id());
      let device = UsbDevice::from_device_info(&info);
      let key = match &device.serial_number {
        Some(s) if !s.is_empty() => s.clone(),
        _ => format!("{}:{}:{}", device.vendor_id, device.product_id, &device_id),
      };
      id_to_key.insert(device_id.clone(), (key.clone(), device.serial_number.clone()));
      pending.insert(
        key,
        UsbEvent {
          event_type: UsbEventType::Connected,
          serial_number: device.serial_number.clone(),
          device: Some(device),
          device_id,
        },
      );
    }
    HotplugEvent::Disconnected(id) => {
      let device_id = format!("{:?}", id);
      if let Some((key, serial)) = id_to_key.remove(&device_id) {
        pending.insert(
          key,
          UsbEvent {
            event_type: UsbEventType::Disconnected,
            serial_number: serial,
            device: None,
            device_id,
          },
        );
      }
    }
  }
}

/// Flush all pending events to the JS callback.
/// Note: `id_to_key` is intentionally NOT cleared here -- it must persist
/// across debounce windows so future Disconnected events (which only carry
/// a device_id) can still resolve to the correct device key.
fn flush_pending(
  pending: &mut HashMap<String, UsbEvent>,
  callback: &ThreadsafeFunction<UsbEvent>,
) {
  for (_, event) in pending.drain() {
    callback.call(Ok(event), ThreadsafeFunctionCallMode::NonBlocking);
  }
}

/// Start watching for USB device connect/disconnect events.
/// The callback receives `(err, event)` for each hotplug event.
/// Only one watcher can be active at a time.
///
/// Events are debounced to coalesce USB re-enumeration sequences
/// (Connected -> Disconnected -> Connected) into a single event.
/// The optional `debounce_ms` parameter controls the window (default 500ms).
#[napi]
pub fn watch_devices(
  callback: ThreadsafeFunction<UsbEvent>,
  debounce_ms: Option<u32>,
) -> Result<()> {
  let mut guard = watcher_state()
    .lock()
    .map_err(|e| Error::from_reason(e.to_string()))?;

  if guard.is_some() {
    return Err(Error::from_reason("Watcher is already running"));
  }

  let debounce = Duration::from_millis(
    debounce_ms.unwrap_or(DEFAULT_DEBOUNCE_MS) as u64,
  );
  let running = Arc::new(AtomicBool::new(true));
  let running_clone = running.clone();

  let handle = std::thread::spawn(move || {
    let watch = match nusb::watch_devices() {
      Ok(w) => w,
      Err(e) => {
        callback.call(
          Err(Error::from_reason(e.to_string())),
          ThreadsafeFunctionCallMode::NonBlocking,
        );
        return;
      }
    };

    let (tx, rx) = mpsc::channel();
    let running_reader = running_clone.clone();

    // Reader thread: pipes raw nusb events into the mpsc channel.
    // Blocks on the async stream; zero cost when idle.
    std::thread::spawn(move || {
      for event in futures_lite::stream::block_on(watch) {
        if !running_reader.load(Ordering::Relaxed) {
          break;
        }
        if tx.send(event).is_err() {
          break;
        }
      }
    });

    let mut pending: HashMap<String, UsbEvent> = HashMap::new();
    let mut id_to_key: HashMap<String, (String, Option<String>)> = HashMap::new();

    // Pre-populate id_to_key with already-connected devices so that
    // disconnect events for them can be resolved.
    if let Ok(existing) = nusb::list_devices().wait() {
      for info in existing {
        let device_id = format!("{:?}", info.id());
        let device = UsbDevice::from_device_info(&info);
        let key = match &device.serial_number {
          Some(s) if !s.is_empty() => s.clone(),
          _ => format!("{}:{}:{}", device.vendor_id, device.product_id, &device_id),
        };
        id_to_key.insert(device_id, (key, device.serial_number));
      }
    }

    // Main debounce loop: block on first event, then collect within window.
    loop {
      // Block until the first event arrives (zero CPU while waiting)
      let first = match rx.recv() {
        Ok(e) => e,
        Err(_) => break,
      };

      if !running_clone.load(Ordering::Relaxed) {
        break;
      }

      process_event(first, &mut pending, &mut id_to_key);

      // Collect more events within the debounce window
      let deadline = Instant::now() + debounce;
      loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
          break;
        }
        match rx.recv_timeout(remaining) {
          Ok(event) => {
            if !running_clone.load(Ordering::Relaxed) {
              break;
            }
            process_event(event, &mut pending, &mut id_to_key);
          }
          Err(RecvTimeoutError::Timeout) => break,
          Err(RecvTimeoutError::Disconnected) => {
            flush_pending(&mut pending, &callback);
            return;
          }
        }
      }

      flush_pending(&mut pending, &callback);
    }
  });

  *guard = Some(WatcherState {
    running,
    _handle: handle,
  });

  Ok(())
}

/// Stop the active USB device watcher.
/// The watcher thread will exit after the next event or on process exit.
#[napi]
pub fn stop_watching() -> Result<()> {
  let mut guard = watcher_state()
    .lock()
    .map_err(|e| Error::from_reason(e.to_string()))?;

  if let Some(state) = guard.take() {
    state.running.store(false, Ordering::Relaxed);
  }

  Ok(())
}
