use core::fmt;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use anyhow::anyhow;

#[derive(Debug, Clone)]
pub struct Metrics {
    data: Arc<RwLock<HashMap<String, i64>>>,
}

impl Metrics {
    pub fn new() -> Self {
        Metrics {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn inc(&self, key: impl Into<String>) -> anyhow::Result<()> {
        let mut data = self.data.write().map_err(|e| anyhow!(e.to_string()))?;

        let counter = data.entry(key.into()).or_insert(0);
        *counter += 1;
        Ok(())
    }

    pub fn snapshot(&self) -> anyhow::Result<HashMap<String, i64>> {
        Ok(self
            .data
            .read()
            .map_err(|e| anyhow!(e.to_string()))?
            .clone())
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.snapshot().map_err(|_e| fmt::Error {})?;
        for (key, value) in data.iter() {
            writeln!(f, "{}:{}", key, value)?;
        }
        Ok(())
    }
}
