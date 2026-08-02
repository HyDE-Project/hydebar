//! The keeper of the last known valid configuration.
//!
//! [`ConfigManager`] holds the configuration behind a lock, hands out clones
//! of the last state that applied, and on [`ConfigManager::apply`] computes
//! the [`ConfigImpact`] of the new state relative to the old one before
//! swapping it in. A refresh that fails is recorded through
//! [`ConfigManager::degraded`] without disturbing the held state.

use std::sync::{Arc, RwLock};

use hydebar_proto::config::Config;

use super::{
    error::{ConfigDegradation, ConfigManagerError, ConfigUpdateError},
    impact::{ConfigImpact, compute_impact}
};

/// Applied configuration along with its computed impact.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigApplied {
    /// The fully validated configuration that was applied.
    pub config: Arc<Config>,
    /// The impact of applying the configuration.
    pub impact: ConfigImpact
}

/// Tracks and manages the last known valid configuration.
#[derive(Debug)]
pub struct ConfigManager {
    state: RwLock<Config>
}

impl ConfigManager {
    /// Creates a new manager seeded with the initial configuration.
    #[must_use]
    pub const fn new(initial: Config) -> Self {
        Self {
            state: RwLock::new(initial)
        }
    }

    fn with_state<F, T>(&self, f: F) -> Result<T, ConfigManagerError>
    where
        F: FnOnce(&Config) -> T
    {
        self.state
            .read()
            .map_err(|_| ConfigManagerError::Poisoned)
            .map(|guard| f(&guard))
    }

    /// Returns the last successfully applied configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigManagerError::Poisoned`] when the internal state lock
    /// was poisoned by a panicking writer.
    pub fn last_valid(&self) -> Result<Config, ConfigManagerError> {
        self.with_state(Clone::clone)
    }

    /// Records a degradation event and returns contextual information for
    /// consumers.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigManagerError::Poisoned`] when the internal state lock
    /// was poisoned by a panicking writer.
    pub fn degraded(
        &self,
        reason: ConfigUpdateError
    ) -> Result<ConfigDegradation, ConfigManagerError> {
        self.with_state(|config| ConfigDegradation {
            reason,
            last_valid: Box::new(config.clone())
        })
    }

    /// Applies a freshly loaded configuration, computing the impact relative to
    /// the previous state.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigManagerError::Poisoned`] when the internal state lock
    /// was poisoned by a panicking writer.
    pub fn apply(&self, updated: Config) -> Result<ConfigApplied, ConfigManagerError> {
        let mut guard = self
            .state
            .write()
            .map_err(|_| ConfigManagerError::Poisoned)?;

        let impact = compute_impact(&guard, &updated);
        *guard = updated.clone();
        drop(guard);

        Ok(ConfigApplied {
            config: Arc::new(updated),
            impact
        })
    }
}
