/// Event-driven network change detection using Windows message loop.
///
/// Replaces the previous 30-second polling approach in lib.rs.
/// WM_DEVICECHANGE fires immediately when a network adapter is added,
/// removed, or changes state — zero CPU overhead between events.
///
/// Architecture: A hidden "message-only" HWND (HWND_MESSAGE parent)
/// is created on a dedicated background thread. RegisterDeviceNotification
/// registers interest in device interface changes (covers all NICs).
/// DBT_DEVICEARRIVAL and DBT_DEVICEREMOVECOMPLETE trigger the "network_changed" event.
#[cfg(target_os = "windows")]
pub fn spawn_network_watcher(app: tauri::AppHandle) -> Result<(), String> {
    std::thread::Builder::new()
        .name("vane-net-watcher".into())
        .spawn(move || run_message_loop(app))
        .map(|_| ())
        .map_err(|e| format!("Network watcher thread could not be started: {}", e))
}

// Thread-local storage for the AppHandle inside the WndProc.
// Using thread_local is safe here because the WndProc is always called
// on the same thread that created the window.
#[cfg(target_os = "windows")]
std::thread_local! {
    static APP_HANDLE: std::cell::RefCell<Option<tauri::AppHandle>> =
        const { std::cell::RefCell::new(None) };
    static LAST_EVENT_TIME: std::cell::Cell<Option<std::time::Instant>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(target_os = "windows")]
fn run_message_loop(app: tauri::AppHandle) {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DispatchMessageW, GetMessageW, RegisterClassW,
        RegisterDeviceNotificationW, DEVICE_NOTIFY_WINDOW_HANDLE, HWND_MESSAGE, MSG,
        WINDOW_EX_STYLE, WNDCLASSW, WS_OVERLAPPED,
    };

    // GUID for network adapters device class:
    // {4d36e972-e325-11ce-bfc1-08002be10318}
    const GUID_NET_DEVICE: windows::core::GUID = windows::core::GUID {
        data1: 0x4d36e972,
        data2: 0xe325,
        data3: 0x11ce,
        data4: [0xbf, 0xc1, 0x08, 0x00, 0x2b, 0xe1, 0x03, 0x18],
    };

    // DEV_BROADCAST_DEVICEINTERFACE_W struct (manual definition for clarity)
    #[repr(C)]
    struct DevBroadcastDeviceInterface {
        db_size: u32,
        db_device_type: u32,
        db_reserved: u32,
        db_classguid: windows::core::GUID,
        db_name: u16,
    }

    const DBT_DEVTYP_DEVICEINTERFACE: u32 = 5;

    let class_name: Vec<u16> = "VaneNetWatcher\0".encode_utf16().collect();

    let wnd_class = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        ..Default::default()
    };

    // Store the app handle before entering the message loop
    APP_HANDLE.with(|cell| {
        *cell.borrow_mut() = Some(app);
    });

    unsafe {
        RegisterClassW(&wnd_class);

        // HWND_MESSAGE = message-only window, no visible UI, minimal overhead
        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            None,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("Network watcher: CreateWindowExW failed: {}", e);
                return;
            }
        };

        // Register for device interface notifications using the correct Win32 API.
        // DEV_BROADCAST_DEVICEINTERFACE with GUID_NET_DEVICE covers all NICs.
        let mut filter = DevBroadcastDeviceInterface {
            db_size: std::mem::size_of::<DevBroadcastDeviceInterface>() as u32,
            db_device_type: DBT_DEVTYP_DEVICEINTERFACE,
            db_reserved: 0,
            db_classguid: GUID_NET_DEVICE,
            db_name: 0,
        };

        let notify_result = RegisterDeviceNotificationW(
            hwnd,
            &mut filter as *mut _ as *mut std::ffi::c_void,
            DEVICE_NOTIFY_WINDOW_HANDLE,
        );

        if notify_result.is_err() {
            tracing::warn!(
                "RegisterDeviceNotificationW failed: {:?}. Network changes will not be monitored.",
                notify_result
            );
            // Continue anyway — WM_DEVICECHANGE may still fire for some events
        } else {
            tracing::info!("Network watcher: Network change monitoring started.");
        }

        // Standard Windows message pump — blocks until WM_QUIT
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
    }
}

/// Window procedure for the message-only window.
///
/// Intercepts WM_DEVICECHANGE to emit Tauri events on network adapter changes.
#[cfg(target_os = "windows")]
unsafe extern "system" fn wnd_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use tauri::Emitter;
    use windows::Win32::UI::WindowsAndMessaging::{DefWindowProcW, WM_DEVICECHANGE};

    const DBT_DEVICEARRIVAL: usize = 0x8000;
    const DBT_DEVICEREMOVECOMPLETE: usize = 0x8004;

    if msg == WM_DEVICECHANGE {
        let event_type = wparam.0;
        if event_type == DBT_DEVICEARRIVAL || event_type == DBT_DEVICEREMOVECOMPLETE {
            APP_HANDLE.with(|cell| {
                if let Some(app) = cell.borrow().as_ref() {
                    let now = std::time::Instant::now();
                    LAST_EVENT_TIME.with(|last_time_cell| {
                        let last = last_time_cell.get();
                        if last.is_none_or(|t| {
                            now.duration_since(t) > std::time::Duration::from_millis(500)
                        }) {
                            last_time_cell.set(Some(now));
                            tracing::info!(
                                "WM_DEVICECHANGE: Network change detected (event=0x{:X}).",
                                event_type
                            );
                            let _ = app.emit("network_changed", ());
                        }
                    });
                }
            });
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// No-op for non-Windows targets.
#[cfg(not(target_os = "windows"))]
pub fn spawn_network_watcher(_app: tauri::AppHandle) -> Result<(), String> {
    Ok(())
}
