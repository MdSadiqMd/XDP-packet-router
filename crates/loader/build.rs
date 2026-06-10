fn main() {
    // Skip eBPF build on non-Linux (e.g., macOS development with rust-analyzer)
    #[cfg(not(target_os = "linux"))]
    {
        // Create empty stub so include_bytes! doesn't fail
        let out_dir = std::env::var("OUT_DIR").unwrap();
        std::fs::write(format!("{out_dir}/pshred-router"), []).ok();
        return;
    }

    #[cfg(target_os = "linux")]
    aya_build::build_ebpf(
        [aya_build::Package {
            name: "pshred-ebpf",
            root_dir: "../ebpf",
            no_default_features: false,
            features: &[],
        }],
        aya_build::Toolchain::Nightly,
    )
    .expect("failed to build eBPF program");
}
