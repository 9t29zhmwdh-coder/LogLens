use crate::models::log_entry::{NormalizedEntry, LogSource};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

pub mod registry;

#[async_trait]
pub trait LogParserPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn can_handle(&self, source: &LogSource) -> bool;
    fn push_line(&mut self, line: &str, source: &LogSource) -> Option<NormalizedEntry>;
    fn flush(&mut self, source: &LogSource) -> Option<NormalizedEntry>;
}

pub trait PluginFactory: Send + Sync {
    fn id(&self) -> &str;
    fn create(&self) -> Box<dyn LogParserPlugin>;
}

pub struct PluginRegistry {
    factories: HashMap<String, Arc<dyn PluginFactory>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self { factories: HashMap::new() }
    }

    pub fn register(&mut self, factory: Arc<dyn PluginFactory>) {
        self.factories.insert(factory.id().to_string(), factory);
    }

    pub fn create(&self, id: &str) -> Option<Box<dyn LogParserPlugin>> {
        self.factories.get(id).map(|f| f.create())
    }

    pub fn detect(&self, source: &LogSource) -> Option<Box<dyn LogParserPlugin>> {
        for factory in self.factories.values() {
            let mut plugin = factory.create();
            if plugin.can_handle(source) {
                return Some(plugin);
            }
        }
        None
    }
}

impl Default for PluginRegistry {
    fn default() -> Self { Self::new() }
}
