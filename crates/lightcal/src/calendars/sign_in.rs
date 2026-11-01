//! Serialize cancellation with the local commit of a browser sign-in attempt.
use std::sync::{Mutex, MutexGuard};

pub struct SignInGate {
    state: Mutex<State>,
}

struct State {
    generation: u64,
    committed: bool,
}

impl SignInGate {
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(State {
                generation: 0,
                committed: false,
            }),
        }
    }

    /// The state is two plain values that are never left half-updated, so a poisoned lock
    /// (a panic elsewhere) is recovered instead of disabling sign-in for the session.
    fn state(&self) -> Result<MutexGuard<'_, State>, String> {
        Ok(self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner))
    }

    pub fn begin(&self) -> Result<u64, String> {
        let mut state = self.state()?;
        state.generation += 1;
        state.committed = false;
        Ok(state.generation)
    }

    pub fn is_current(&self, generation: u64) -> bool {
        self.state()
            .is_ok_and(|state| state.generation == generation)
    }

    /// A completed local save must still be delivered to the UI, even if Cancel
    /// was clicked before the queued completion callback ran.
    pub fn cancel(&self) -> Result<bool, String> {
        let mut state = self.state()?;
        if state.committed {
            return Ok(false);
        }
        state.generation += 1;
        Ok(true)
    }

    /// Call after acquiring the credential lock. Only local credential I/O may
    /// run in `save`; browser consent and provider requests stay outside it.
    pub fn commit<T>(
        &self,
        generation: u64,
        save: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let mut state = self.state()?;
        if state.generation != generation {
            return Err("Sign-in cancelled".into());
        }
        let result = save()?;
        state.committed = true;
        Ok(result)
    }
}

impl Default for SignInGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Arc};

    #[test]
    fn cancellation_while_waiting_for_credentials_prevents_the_save() {
        let gate = Arc::new(SignInGate::new());
        let generation = gate.begin().unwrap();
        let credentials = Arc::new(Mutex::new(()));
        let guard = credentials.lock().unwrap();
        let (ready, token_received) = mpsc::channel();
        let worker_gate = gate.clone();
        let worker_credentials = credentials.clone();
        let worker = std::thread::spawn(move || {
            assert!(worker_gate.is_current(generation));
            ready.send(()).unwrap();
            let _guard = worker_credentials.lock().unwrap();
            worker_gate.commit(generation, || {
                panic!("Cancelled credentials must not be saved")
            })
        });
        token_received.recv().unwrap();
        assert!(gate.cancel().unwrap());
        drop(guard);
        assert_eq!(
            worker.join().unwrap(),
            Err::<(), _>("Sign-in cancelled".into())
        );
    }

    #[test]
    fn a_committed_connection_is_not_discarded_before_ui_delivery() {
        let gate = SignInGate::new();
        let generation = gate.begin().unwrap();
        gate.commit(generation, || Ok(())).unwrap();
        assert!(!gate.cancel().unwrap());
        assert!(gate.is_current(generation));
    }

    #[test]
    fn gate_recovers_after_its_lock_is_poisoned() {
        let gate = Arc::new(SignInGate::new());
        let poisoner = gate.clone();
        let handle = std::thread::spawn(move || {
            let generation = poisoner.begin().unwrap();
            let _ = poisoner.commit(generation, || -> Result<(), String> {
                panic!("simulated panic while the gate is locked")
            });
        });
        assert!(handle.join().is_err());
        assert!(gate.state.is_poisoned());
        let generation = gate.begin().unwrap();
        assert!(gate.is_current(generation));
        assert!(gate.cancel().unwrap());
    }

    #[test]
    fn a_new_attempt_rejects_an_older_authorization_response() {
        let gate = SignInGate::new();
        let previous = gate.begin().unwrap();
        let current = gate.begin().unwrap();
        assert_eq!(
            gate.commit(previous, || Ok(())),
            Err("Sign-in cancelled".into())
        );
        gate.commit(current, || Ok(())).unwrap();
    }
}
