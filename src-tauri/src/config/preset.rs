use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum PresetCategory {
    Standard,
    Fragment,
    Desync,
    DesyncFake,
    Quic,
    Aggressive,

    #[default]
    Custom,
}

/*
   Definition for a single DPI bypass preset.
   Design decision: `args` field contains direct winws parameters.
   This avoids the need to modify Rust code when adding a new preset (Open/Closed principle).
*/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub label: String,
    pub description: String,
    pub icon: String,
    pub args: Vec<String>,
    pub is_custom: bool,

    #[serde(default)]
    pub priority: u8,

    #[serde(default)]
    pub category: PresetCategory,
}

/*
   Returns the built-in preset list embedded in the binary at compile-time.
   No Disk I/O is performed; fixed at compilation time via `include_str!`.
*/
pub fn builtin_presets() -> Vec<Preset> {
    serde_json::from_str(include_str!("../../../presets/builtin.json"))
        .expect("presets/builtin.json geçerli JSON olmalıdır — bu bir compile-time invariant'tır")
}
