//! **Note**: this is a barely modified copy of the code which appears in mini-redis

type Notify = tokio::sync::broadcast::Receiver<()>;
type Guard = tokio::sync::mpsc::Sender<()>;
/// Listens for the server shutdown signal.
///
/// Shutdown is signalled using a `broadcast::Receiver`. Only a single value is
/// ever sent. Once a value has been sent via the broadcast channel, the server
/// should shutdown.
///
/// The `Shutdown` struct listens for the signal and tracks that the signal has
/// been received. Callers may query for whether the shutdown signal has been
/// received or not.
///
#[derive(Debug)]
pub(crate) struct Shutdown {
    /// `true` if the shutdown signal has been received
    shutdown: bool,

    /// The receive half of the channel used to listen for shutdown.
    notify: Notify,

    /// Optional guard as a sender so that when the `Shutdown` struct is dropped, the other side of the channel is
    /// closed.
    guard: Option<Guard>,
}

impl Clone for Shutdown {
    fn clone(&self) -> Self {
        Self {
            shutdown: self.shutdown,
            notify: self.notify.resubscribe(),
            guard: self.guard.clone(),
        }
    }
}

impl Shutdown {
    /// Create a new `Shutdown` backed by the given `broadcast::Receiver`.
    pub(crate) fn new(notify: Notify) -> Shutdown {
        Shutdown {
            shutdown: false,
            notify,
            guard: None,
        }
    }
    /// Create a new `Shutdown` backed by the given `broadcast::Receiver` with a given guard.
    pub(crate) fn with_guard(notify: Notify, guard: Guard) -> Shutdown {
        Shutdown {
            shutdown: false,
            notify,
            guard: Some(guard),
        }
    }

    /// Returns `true` if the shutdown signal has been received.
    pub(crate) fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    /// Receive the shutdown notice, waiting if necessary.
    pub(crate) async fn recv(&mut self) {
        // If the shutdown signal has already been received, then return
        // immediately.
        if self.is_shutdown() {
            return;
        }

        // Cannot receive a "lag error" as only one value is ever sent.
        let _ = self.notify.recv().await;

        // Remember that the signal has been received.
        self.shutdown = true;
    }
}

impl From<Notify> for Shutdown {
    fn from(notify: Notify) -> Self {
        Self::new(notify)
    }
}

impl From<(Notify, Guard)> for Shutdown {
    fn from((notify, guard): (Notify, Guard)) -> Self {
        Self::with_guard(notify, guard)
    }
}
impl From<(Guard, Notify)> for Shutdown {
    fn from((guard, notify): (Guard, Notify)) -> Self {
        Self::with_guard(notify, guard)
    }
}
