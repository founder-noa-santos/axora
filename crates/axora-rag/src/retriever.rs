//! Reciprocal-rank-fusion helpers for hybrid retrieval.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Ranked hit from a retrieval source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RankedHit {
    /// Document identifier.
    pub document_id: String,
    /// 1-based rank.
    pub rank: u32,
    /// Raw source score.
    pub score: f32,
    /// Retrieval source name.
    pub source: String,
}

/// Fused score after reciprocal rank fusion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FusedRank {
    /// Document identifier.
    pub document_id: String,
    /// Combined RRF score.
    pub score: f32,
    /// Number of lists that contained the document.
    pub source_count: u32,
}

/// Reciprocal-rank-fusion helper.
#[derive(Debug, Clone)]
pub struct ReciprocalRankFusion {
    k: f32,
}

impl ReciprocalRankFusion {
    /// Create a new RRF helper.
    pub fn new(k: f32) -> Self {
        Self { k }
    }

    /// Fuse multiple ranked lists into one sorted list.
    pub fn fuse(&self, lists: &[Vec<RankedHit>]) -> Vec<FusedRank> {
        let mut scores: HashMap<String, (f32, u32)> = HashMap::new();
        for hits in lists {
            for hit in hits {
                let entry = scores.entry(hit.document_id.clone()).or_insert((0.0, 0));
                entry.0 += 1.0 / (self.k + hit.rank as f32);
                entry.1 += 1;
            }
        }

        let mut fused = scores
            .into_iter()
            .map(|(document_id, (score, source_count))| FusedRank {
                document_id,
                score,
                source_count,
            })
            .collect::<Vec<_>>();
        fused.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        fused
    }
}

impl Default for ReciprocalRankFusion {
    fn default() -> Self {
        Self::new(60.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reciprocal_rank_fusion_orders_overlap_first() {
        let fusion = ReciprocalRankFusion::default();
        let dense = vec![
            RankedHit {
                document_id: "a".to_string(),
                rank: 1,
                score: 0.9,
                source: "dense".to_string(),
            },
            RankedHit {
                document_id: "b".to_string(),
                rank: 2,
                score: 0.8,
                source: "dense".to_string(),
            },
        ];
        let sparse = vec![
            RankedHit {
                document_id: "b".to_string(),
                rank: 1,
                score: 7.0,
                source: "bm25".to_string(),
            },
            RankedHit {
                document_id: "c".to_string(),
                rank: 2,
                score: 5.0,
                source: "bm25".to_string(),
            },
        ];

        let fused = fusion.fuse(&[dense, sparse]);
        assert_eq!(fused[0].document_id, "b");
    }
}
