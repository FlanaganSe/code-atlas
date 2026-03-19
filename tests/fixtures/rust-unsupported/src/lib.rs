#[cfg(feature = "serde-support")]
mod serde_impl;

mod regular;

pub fn hello() -> &'static str {
    "hello"
}
