#[cfg(not(target_os = "macos"))]
compile_error!("ReMTR currently supports macOS only");

#[cfg(target_os = "macos")]
pub mod darwin;
