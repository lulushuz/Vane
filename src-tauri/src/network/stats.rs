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
        let mut total_rx = 0;
        let mut total_tx = 0;
        for line in content.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 10 {
                if let (Ok(rx), Ok(tx)) = (parts[1].parse::<u64>(), parts[9].parse::<u64>()) {
                    total_rx += rx;
                    total_tx += tx;
                }
            }
        }
        return (total_rx, total_tx);
    }
    (0, 0)
}
