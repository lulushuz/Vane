#[cfg(target_os = "windows")]
pub fn get_total_network_bytes() -> (u64, u64) {
    use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};

    let mut table_ptr: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    unsafe {
        if GetIfTable2(&mut table_ptr).is_ok() && !table_ptr.is_null() {
            let table = &*table_ptr;
            let mut total_rx = 0u64;
            let mut total_tx = 0u64;
            let count = table.NumEntries as usize;
            let rows = std::slice::from_raw_parts(table.Table.as_ptr(), count);
            for row in rows {
                // Filter out loopback (24), tunnel (131), and virtual adapters.
                // Keep active physical Ethernet (6) and Wi-Fi (71) or general active physical adapters.
                if row.OperStatus.0 == 1 && row.Type != 24 && row.Type != 131 && row.Type != 53 {
                    total_rx += row.InOctets;
                    total_tx += row.OutOctets;
                }
            }
            FreeMibTable(table_ptr as *const std::ffi::c_void);
            return (total_rx, total_tx);
        }
    }
    (0, 0)
}

#[cfg(not(target_os = "windows"))]
pub fn get_total_network_bytes() -> (u64, u64) {
    if let Ok(content) = std::fs::read_to_string("/proc/net/dev") {
        let mut total_rx = 0u64;
        let mut total_tx = 0u64;
        for line in content.lines().skip(2) {
            let Some((iface_raw, stats_raw)) = line.split_once(':') else {
                continue;
            };
            let iface = iface_raw.trim();
            if iface == "lo"
                || iface.starts_with("docker")
                || iface.starts_with("br-")
                || iface.starts_with("veth")
                || iface.starts_with("virbr")
                || iface.starts_with("tun")
            {
                continue;
            }
            let fields: Vec<&str> = stats_raw.split_whitespace().collect();
            if fields.len() >= 9 {
                if let (Ok(rx), Ok(tx)) = (fields[0].parse::<u64>(), fields[8].parse::<u64>()) {
                    total_rx = total_rx.saturating_add(rx);
                    total_tx = total_tx.saturating_add(tx);
                }
            }
        }
        return (total_rx, total_tx);
    }
    (0, 0)
}
