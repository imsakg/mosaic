//! Deterministic package and target metadata without runtime dependency probes.
pub fn prepare_build_info() -> String {
    format!(
        "Mosaic {} ({}-{})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
        std::env::consts::OS
    )
}
