//! The signal-safe ledger of every process group the bar still owns.

use std::sync::atomic::{AtomicU32, Ordering};

use super::termination::kill_group;

/// Process groups the registry can hold at once.
///
/// One slot per live listener or scheduled command; a configuration with a
/// module per slot would be unusable long before the registry filled up.
pub(super) const REGISTRY_CAPACITY: usize = 512;

/// Value stored in a registry slot that holds no group.
const EMPTY_SLOT: u32 = 0;

/// Groups the bar started and has not ended yet.
///
/// The registry is a fixed array of atomics rather than a locked set because a
/// termination signal has to be able to walk it: taking a lock in a signal
/// handler risks deadlocking against the thread that already holds it, and a
/// deadlock here is exactly the leak this module exists to prevent.
pub(super) struct GroupRegistry {
    /// Leader of one group per occupied slot.
    slots: [AtomicU32; REGISTRY_CAPACITY]
}

impl GroupRegistry {
    /// A registry holding no group.
    pub(super) const fn new() -> Self {
        Self {
            slots: [const { AtomicU32::new(EMPTY_SLOT) }; REGISTRY_CAPACITY]
        }
    }

    /// Records the group led by `pid`, reporting whether a slot was free.
    pub(super) fn insert(&self, pid: u32) -> bool {
        self.slots.iter().any(|slot| {
            slot.compare_exchange(EMPTY_SLOT, pid, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
        })
    }

    /// Forgets the group led by `pid`.
    pub(super) fn remove(&self, pid: u32) {
        for slot in &self.slots {
            if slot
                .compare_exchange(pid, EMPTY_SLOT, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }
        }
    }

    /// Reports whether the group led by `pid` is recorded.
    ///
    /// Only the tests observe the registry this way; live code either records
    /// or ends groups and never has to ask.
    #[cfg(test)]
    pub(super) fn contains(&self, pid: u32) -> bool {
        self.slots
            .iter()
            .any(|slot| slot.load(Ordering::Acquire) == pid)
    }

    /// Ends every recorded group, emptying the registry.
    ///
    /// Only an atomic swap and `kill` are used, so this is safe to run from a
    /// signal handler; it reports how many groups it reached rather than
    /// logging, because logging from a handler is not.
    pub(super) fn terminate_all(&self) -> usize {
        let mut ended = 0;

        for slot in &self.slots {
            let pid = slot.swap(EMPTY_SLOT, Ordering::AcqRel);

            if pid != EMPTY_SLOT {
                kill_group(pid);
                ended += 1;
            }
        }

        ended
    }
}

/// Registry every [`GroupGuard`] records itself in.
pub(super) static LIVE_GROUPS: GroupRegistry = GroupRegistry::new();
