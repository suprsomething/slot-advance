use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use slot_store::{write_slot_state, GameProfiles, SlotState};

#[derive(Default)]
struct Queue {
    next: Option<(PathBuf, SlotState)>,
    /// The latest per-game profiles waiting to be written. Like `next`, only the newest
    /// matters: a burst of shader presses writes the file once, with the last of them.
    profiles: Option<(PathBuf, GameProfiles)>,
    busy: bool,
    closed: bool,
}

type Shared = Arc<(Mutex<Queue>, Condvar)>;

pub struct StateWriter {
    shared: Shared,
    threaded: bool,
    failing: Arc<AtomicBool>,
}

impl StateWriter {
    pub fn spawn() -> Self {
        let shared: Shared = Arc::default();
        let failing: Arc<AtomicBool> = Arc::default();
        let worker = shared.clone();
        let flag = failing.clone();
        let threaded = thread::Builder::new()
            .name("slot-state".into())
            .spawn(move || work(&worker, &flag))
            .is_ok();
        StateWriter {
            shared,
            threaded,
            failing,
        }
    }

    pub fn save(&self, root: &Path, state: &SlotState) {
        if !self.threaded {
            return write(root, state, &self.failing);
        }
        let (lock, wake) = &*self.shared;
        let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
        q.next = Some((root.to_path_buf(), state.clone()));
        wake.notify_all();
    }

    /// Queues `Config/game_profiles.txt`, written off the game thread like `slot.state`.
    pub fn save_profiles(&self, root: &Path, profiles: &GameProfiles) {
        if !self.threaded {
            return write_profiles(root, profiles, &self.failing);
        }
        let (lock, wake) = &*self.shared;
        let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
        q.profiles = Some((root.to_path_buf(), profiles.clone()));
        wake.notify_all();
    }

    pub fn card_failing(&self) -> Arc<AtomicBool> {
        self.failing.clone()
    }

    pub fn flush(&self) {
        let (lock, wake) = &*self.shared;
        let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
        while q.next.is_some() || q.profiles.is_some() || q.busy {
            q = wake.wait(q).unwrap_or_else(|e| e.into_inner());
        }
    }
}

impl Drop for StateWriter {
    fn drop(&mut self) {
        self.flush();
        let (lock, wake) = &*self.shared;
        lock.lock().unwrap_or_else(|e| e.into_inner()).closed = true;
        wake.notify_all();
    }
}

fn work(shared: &Shared, failing: &AtomicBool) {
    let (lock, wake) = &**shared;
    loop {
        let job = {
            let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
            loop {
                if let Some(job) = q.next.take() {
                    q.busy = true;
                    break Job::State(job);
                }
                if let Some(job) = q.profiles.take() {
                    q.busy = true;
                    break Job::Profiles(job);
                }
                if q.closed {
                    return;
                }
                q = wake.wait(q).unwrap_or_else(|e| e.into_inner());
            }
        };
        match job {
            Job::State((root, state)) => write(&root, &state, failing),
            Job::Profiles((root, profiles)) => write_profiles(&root, &profiles, failing),
        }
        lock.lock().unwrap_or_else(|e| e.into_inner()).busy = false;
        wake.notify_all();
    }
}

enum Job {
    State((PathBuf, SlotState)),
    Profiles((PathBuf, GameProfiles)),
}

fn write_profiles(root: &Path, profiles: &GameProfiles, failing: &AtomicBool) {
    let written = profiles.write(root);
    if let Err(e) = &written {
        eprintln!("slot: game_profiles.txt: {e}");
    }
    failing.store(written.is_err(), Ordering::Relaxed);
}

fn write(root: &Path, state: &SlotState, failing: &AtomicBool) {
    let written = write_slot_state(root, state);
    if let Err(e) = &written {
        eprintln!("slot: slot.state: {e}");
    }
    failing.store(written.is_err(), Ordering::Relaxed);
}
