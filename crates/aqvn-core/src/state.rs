use std::sync::atomic::{AtomicBool, Ordering};

static GATEWAY_CONNECTED: AtomicBool = AtomicBool::new(false);

/// Checks whether Gateway connection is currently active.
pub fn is_gateway_connected() -> bool {
    GATEWAY_CONNECTED.load(Ordering::SeqCst)
}

/// Sets the Gateway connection state.
pub fn set_gateway_connected(connected: bool) {
    GATEWAY_CONNECTED.store(connected, Ordering::SeqCst);
}
