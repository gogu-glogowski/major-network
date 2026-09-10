fn main() {
    eprintln!(
        "mnp-server {} — v0.0.x accepts. HELLO spike is next (Quinn/IPv6).",
        env!("CARGO_PKG_VERSION")
    );
    let _ = mnp_core::HELLO_ACK;
    std::process::exit(2);
}
