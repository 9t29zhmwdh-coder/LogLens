#![allow(unused_imports)]
use std::collections::HashMap;
use dashmap::DashMap;
use strsim::normalized_levenshtein;
use crate::models::log_entry::{NormalizedEntry, LogLevel};
use crate::models::cluster::LogCluster;
use super::fingerprint::compute_fingerprint;

const SIMILARITY_THRESHOLD: f64 = 0.85;
const MAX_SAMPLES: usize = 5;

pub struct ClusterGrouper {
    clusters: DashMap<String, LogCluster>,
    /// fingerprint → cluster_id
    fp_index: DashMap<String, String>,
}

impl ClusterGrouper {
    pub fn new() -> Self {
        Self {
            clusters: DashMap::new(),
            fp_index: DashMap::new(),
        }
    }

    /// Returns the cluster_id the entry was assigned to.
    pub fn process(&self, entry: &mut NormalizedEntry) -> String {
        let (fingerprint, template) = compute_fingerprint(&entry.message);
        entry.fingerprint = fingerprint.clone();

        // Fast path: exact fingerprint match
        if let Some(cid) = self.fp_index.get(&fingerprint) {
            let cid = cid.clone();
            self.update_cluster(&cid, entry);
            entry.cluster_id = Some(cid.clone());
            return cid;
        }

        // Slow path: fuzzy similarity against existing templates
        let mut best_match: Option<(String, f64)> = None;
        for cluster in self.clusters.iter() {
            let score = normalized_levenshtein(&template, &cluster.template);
            if score >= SIMILARITY_THRESHOLD {
                if best_match.as_ref().map_or(true, |(_, s)| score > *s) {
                    best_match = Some((cluster.id.clone(), score));
                }
            }
        }

        if let Some((cid, _)) = best_match {
            self.fp_index.insert(fingerprint, cid.clone());
            self.update_cluster(&cid, entry);
            entry.cluster_id = Some(cid.clone());
            return cid;
        }

        // Create new cluster
        let cluster = LogCluster::new(&fingerprint, &template, entry.level.clone());
        let cid = cluster.id.clone();
        self.fp_index.insert(fingerprint, cid.clone());
        self.clusters.insert(cid.clone(), cluster);
        self.update_cluster(&cid, entry);
        entry.cluster_id = Some(cid.clone());
        cid
    }

    fn update_cluster(&self, cid: &str, entry: &NormalizedEntry) {
        if let Some(mut c) = self.clusters.get_mut(cid) {
            c.count += 1;
            c.last_seen = entry.timestamp;
            if c.first_seen > entry.timestamp {
                c.first_seen = entry.timestamp;
            }
            if !c.source_ids.contains(&entry.source_id) {
                c.source_ids.push(entry.source_id.clone());
            }
            if c.sample_ids.len() < MAX_SAMPLES {
                c.sample_ids.push(entry.id.clone());
            }
            if let Some(ref svc) = entry.service {
                if !c.services.contains(svc) {
                    c.services.push(svc.clone());
                }
            }
            // Escalate level
            if entry.level > c.level {
                c.level = entry.level.clone();
            }
        }
    }

    pub fn all_clusters(&self) -> Vec<LogCluster> {
        self.clusters.iter().map(|e| e.value().clone()).collect()
    }

    pub fn get_cluster(&self, id: &str) -> Option<LogCluster> {
        self.clusters.get(id).map(|c| c.clone())
    }

    pub fn top_errors(&self, n: usize) -> Vec<LogCluster> {
        let mut clusters: Vec<_> = self.clusters.iter()
            .filter(|e| e.level >= LogLevel::Error)
            .map(|e| e.value().clone())
            .collect();
        clusters.sort_by(|a, b| b.count.cmp(&a.count));
        clusters.truncate(n);
        clusters
    }
}

impl Default for ClusterGrouper {
    fn default() -> Self { Self::new() }
}
