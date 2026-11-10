//! Systemd watchdog and daemon lifecycle notification (`sd_notify`).
//!
//! Provides `sd_notify` socket communication for systemd watchdog keepalive (`WATCHDOG=1`),
//! ready state (`READY=1`), and graceful shutdown (`STOPPING=1`).

use std::time::Duration;

/// Sends a raw notification state string to systemd via `NOTIFY_SOCKET`.
/// Returns `true` if successfully sent, `false` otherwise.
#[cfg(unix)]
pub fn sd_notify(state: &str) -> bool {
    use std::env;
    use std::os::unix::net::UnixDatagram;

    let socket_path = match env::var("NOTIFY_SOCKET") {
        Ok(p) if !p.is_empty() => p,
        _ => return false,
    };

    let socket = match UnixDatagram::unbound() {
        Ok(s) => s,
        Err(_) => return false,
    };

    let payload = if state.ends_with('\n') {
        state.to_string()
    } else {
        format!("{state}\n")
    };

    let res = if let Some(abstract_name) = socket_path.strip_prefix('@') {
        #[cfg(target_os = "linux")]
        {
            use std::os::linux::net::SocketAddrExt;
            match std::os::unix::net::SocketAddr::from_abstract_name(abstract_name.as_bytes()) {
                Ok(addr) => socket
                    .connect_addr(&addr)
                    .and_then(|()| socket.send(payload.as_bytes())),
                Err(e) => Err(e),
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = abstract_name;
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "abstract sockets unsupported",
            ))
        }
    } else {
        socket.send_to(payload.as_bytes(), &socket_path)
    };

    res.is_ok()
}

#[cfg(not(unix))]
pub fn sd_notify(_state: &str) -> bool {
    false
}

/// Computes the recommended watchdog ping interval based on `WATCHDOG_USEC`.
/// Systemd recommends sending pings at half the watchdog interval ($T / 2$).
pub fn watchdog_interval() -> Option<Duration> {
    if let Ok(v) = std::env::var("WATCHDOG_USEC") {
        if let Ok(usec) = v.trim().parse::<u64>() {
            if usec > 0 {
                let ping_micros = (usec / 2).max(50_000);
                return Some(Duration::from_micros(ping_micros));
            }
        }
    }

    if std::env::var("NOTIFY_SOCKET").is_ok() {
        Some(Duration::from_secs(15))
    } else {
        None
    }
}

/// Notifies systemd that service startup is complete and ready to serve (`READY=1`).
pub fn notify_ready() -> bool {
    sd_notify("READY=1")
}

/// Sends periodic watchdog heartbeat to systemd (`WATCHDOG=1`).
pub fn notify_watchdog() -> bool {
    sd_notify("WATCHDOG=1")
}

/// Notifies systemd that service is initiating graceful shutdown (`STOPPING=1`).
pub fn notify_stopping() -> bool {
    sd_notify("STOPPING=1")
}

/// Spawns a background task sending `WATCHDOG=1` heartbeats at regular intervals.
/// Terminated when `stop_rx` receives `true`.
pub fn spawn_watchdog_task(
    mut stop_rx: tokio::sync::watch::Receiver<bool>,
) -> Option<tokio::task::JoinHandle<()>> {
    let interval = watchdog_interval()?;
    Some(tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        // First tick completes immediately
        ticker.tick().await;

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    notify_watchdog();
                }
                res = stop_rx.changed() => {
                    if res.is_err() || *stop_rx.borrow() {
                        break;
                    }
                }
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixDatagram;

    #[test]
    fn test_sd_notify_mock_socket() {
        let _lock = crate::tests::TEST_ENV_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let temp_dir = std::env::temp_dir().join(format!("test_sd_notify_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let sock_path = temp_dir.join("notify.sock");
        let listener = UnixDatagram::bind(&sock_path).expect("bind mock notify socket");
        listener.set_nonblocking(true).unwrap();

        std::env::set_var("NOTIFY_SOCKET", &sock_path);

        assert!(notify_ready());
        let mut buf = [0u8; 128];
        let (n, _) = listener.recv_from(&mut buf).expect("receive READY");
        let msg = String::from_utf8_lossy(&buf[..n]);
        assert_eq!(msg, "READY=1\n");

        assert!(notify_watchdog());
        let (n, _) = listener.recv_from(&mut buf).expect("receive WATCHDOG");
        let msg = String::from_utf8_lossy(&buf[..n]);
        assert_eq!(msg, "WATCHDOG=1\n");

        assert!(notify_stopping());
        let (n, _) = listener.recv_from(&mut buf).expect("receive STOPPING");
        let msg = String::from_utf8_lossy(&buf[..n]);
        assert_eq!(msg, "STOPPING=1\n");

        std::env::remove_var("NOTIFY_SOCKET");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_watchdog_interval_computation() {
        let _lock = crate::tests::TEST_ENV_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        std::env::set_var("WATCHDOG_USEC", "30000000"); // 30s
        let interval = watchdog_interval().expect("interval calculated");
        assert_eq!(interval, Duration::from_secs(15)); // Half interval

        std::env::remove_var("WATCHDOG_USEC");
        std::env::remove_var("NOTIFY_SOCKET");
        assert_eq!(watchdog_interval(), None);
    }
}
