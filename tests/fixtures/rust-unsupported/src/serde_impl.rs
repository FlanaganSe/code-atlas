// This module is conditionally compiled behind #[cfg(feature = "serde-support")]
pub fn serialize() -> String {
    "serialized".to_string()
}
