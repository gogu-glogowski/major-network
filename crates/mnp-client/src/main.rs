fn main() {
    eprintln!(
        "mnp-client {} — v0.0.x initiates. HELLO spike is next (Quinn/IPv6).",
        env!("CARGO_PKG_VERSION")
    );
    let _ = mnp_core::HELLO;
    std::process::exit(2);
}
