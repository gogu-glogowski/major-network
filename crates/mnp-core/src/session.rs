//! v0.0.3 session state machine.
//!
//! Default `lab_auto_accept = true` so READY is reachable after HELLO without
//! identity crypto. That flag is stripped when software mutual auth lands.

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Disconnected,
    QuicConnected,
    MnpHello,
    IdentityPending,
    Authenticated,
    Ready,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("illegal transition {from:?} --{event}-->")]
    Illegal {
        from: SessionState,
        event: &'static str,
    },
}

#[derive(Debug, Clone)]
pub struct Session {
    state: SessionState,
    lab_auto_accept: bool,
}

impl Session {
    /// Lab default: auto-accept identity after HELLO.
    pub fn new_lab() -> Self {
        Self {
            state: SessionState::Disconnected,
            lab_auto_accept: true,
        }
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn lab_auto_accept(&self) -> bool {
        self.lab_auto_accept
    }

    pub fn on_quic_connected(&mut self) -> Result<(), SessionError> {
        self.step(
            SessionState::Disconnected,
            SessionState::QuicConnected,
            "quic_connected",
        )
    }

    /// HELLO / HELLO_ACK accepted. With `LAB_AUTO_ACCEPT`, continues to READY.
    pub fn on_hello_ok(&mut self) -> Result<(), SessionError> {
        self.step(SessionState::QuicConnected, SessionState::MnpHello, "hello")?;
        self.step(
            SessionState::MnpHello,
            SessionState::IdentityPending,
            "hello",
        )?;
        if self.lab_auto_accept {
            tracing::warn!(lab_auto_accept = true, "identity skipped (lab only)");
            self.step(
                SessionState::IdentityPending,
                SessionState::Authenticated,
                "lab_auto_accept",
            )?;
            // AUTHENTICATED → READY is a no-op until policy exists.
            self.step(
                SessionState::Authenticated,
                SessionState::Ready,
                "authz_noop",
            )?;
        }
        Ok(())
    }

    pub fn on_goodbye_or_loss(&mut self) {
        self.state = SessionState::Disconnected;
    }

    fn step(
        &mut self,
        expected: SessionState,
        next: SessionState,
        event: &'static str,
    ) -> Result<(), SessionError> {
        if self.state != expected {
            return Err(SessionError::Illegal {
                from: self.state,
                event,
            });
        }
        self.state = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lab_auto_accept_reaches_ready() {
        let mut s = Session::new_lab();
        s.on_quic_connected().unwrap();
        s.on_hello_ok().unwrap();
        assert_eq!(s.state(), SessionState::Ready);
        assert!(s.lab_auto_accept());
    }

    #[test]
    fn hello_before_quic_is_illegal() {
        let mut s = Session::new_lab();
        let err = s.on_hello_ok().unwrap_err();
        assert!(matches!(
            err,
            SessionError::Illegal {
                from: SessionState::Disconnected,
                event: "hello"
            }
        ));
    }

    #[test]
    fn double_connect_is_illegal() {
        let mut s = Session::new_lab();
        s.on_quic_connected().unwrap();
        assert!(s.on_quic_connected().is_err());
    }

    #[test]
    fn goodbye_returns_to_disconnected() {
        let mut s = Session::new_lab();
        s.on_quic_connected().unwrap();
        s.on_hello_ok().unwrap();
        s.on_goodbye_or_loss();
        assert_eq!(s.state(), SessionState::Disconnected);
    }
}
