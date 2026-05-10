//! Agglomerative clustering of speaker embeddings using cosine similarity.
//!
//! The algorithm:
//! 1. Each segment starts as its own cluster.
//! 2. Repeatedly merge the two clusters whose average-link cosine similarity
//!    is the highest, provided it exceeds `threshold`.
//! 3. Stop when no pair of clusters meets the threshold.
//!
//! Returns a `Vec<usize>` of cluster IDs (one per input embedding), where IDs
//! are assigned in the order each speaker first appears (so the first speaker
//! is `0`, second new speaker is `1`, etc.).

/// Cluster speaker embeddings.
///
/// `threshold` is the minimum cosine similarity (in `[-1, 1]`) required to
/// merge two clusters. A value of `0.75` is a reasonable default.
pub fn cluster_speakers(embeddings: &[Vec<f32>], threshold: f32) -> Vec<usize> {
    let n = embeddings.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![0];
    }

    // Start: each segment is its own cluster.
    let mut clusters: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();

    loop {
        // Find the closest pair of clusters by average-link similarity.
        let mut best: Option<(usize, usize, f32)> = None;
        for i in 0..clusters.len() {
            for j in (i + 1)..clusters.len() {
                let sim = average_link_similarity(&clusters[i], &clusters[j], embeddings);
                if best.map(|(_, _, s)| sim > s).unwrap_or(true) {
                    best = Some((i, j, sim));
                }
            }
        }

        match best {
            Some((i, j, sim)) if sim >= threshold => {
                // Merge cluster j into cluster i.
                let merged = clusters.remove(j);
                clusters[i].extend(merged);
            }
            _ => break,
        }
    }

    // Assign cluster IDs in order of first appearance.
    let mut assignments = vec![0usize; n];
    let mut cluster_for_segment: Vec<Option<usize>> = vec![None; n];
    for (cluster_id, members) in clusters.iter().enumerate() {
        for &member in members {
            cluster_for_segment[member] = Some(cluster_id);
        }
    }

    // Renumber clusters by their first-appearing segment, so segment 0 always
    // belongs to speaker 0, the next segment with a new speaker is speaker 1,
    // and so on. This produces stable, intuitive labels.
    let mut renumber: Vec<Option<usize>> = vec![None; clusters.len()];
    let mut next_id = 0usize;
    for (segment_idx, slot) in cluster_for_segment.iter().enumerate() {
        let original = slot.expect("every segment must be assigned");
        let new_id = match renumber[original] {
            Some(id) => id,
            None => {
                let id = next_id;
                renumber[original] = Some(id);
                next_id += 1;
                id
            }
        };
        assignments[segment_idx] = new_id;
    }

    assignments
}

fn average_link_similarity(a: &[usize], b: &[usize], embeddings: &[Vec<f32>]) -> f32 {
    let mut total = 0.0f32;
    let mut count = 0usize;
    for &i in a {
        for &j in b {
            total += cosine_similarity(&embeddings[i], &embeddings[j]);
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "embedding dimensions must match");
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = (na.sqrt() * nb.sqrt()).max(f32::EPSILON);
    dot / denom
}

/// Convert a numeric speaker index into a human-readable label.
///
/// `0 -> "Speaker A"`, `1 -> "Speaker B"`, ..., `25 -> "Speaker Z"`,
/// `26 -> "Speaker AA"`, etc.
pub fn speaker_label(index: usize) -> String {
    let mut n = index;
    let mut letters = Vec::new();
    loop {
        letters.push((b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    let suffix: String = letters.into_iter().rev().collect();
    format!("Speaker {suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(v: Vec<f32>) -> Vec<f32> {
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.into_iter().map(|x| x / norm).collect()
    }

    #[test]
    fn single_segment_yields_one_cluster() {
        let embeddings = vec![unit(vec![1.0, 0.0, 0.0])];
        assert_eq!(cluster_speakers(&embeddings, 0.75), vec![0]);
    }

    #[test]
    fn empty_input_yields_empty_output() {
        let embeddings: Vec<Vec<f32>> = Vec::new();
        assert!(cluster_speakers(&embeddings, 0.75).is_empty());
    }

    #[test]
    fn identical_embeddings_collapse_into_one_cluster() {
        let e = unit(vec![1.0, 2.0, 3.0]);
        let embeddings = vec![e.clone(), e.clone(), e];
        assert_eq!(cluster_speakers(&embeddings, 0.75), vec![0, 0, 0]);
    }

    #[test]
    fn two_distinct_speakers_get_two_clusters() {
        let a = unit(vec![1.0, 0.0, 0.0, 0.0]);
        let b = unit(vec![0.0, 1.0, 0.0, 0.0]);
        let embeddings = vec![a.clone(), b.clone(), a.clone(), b];
        let result = cluster_speakers(&embeddings, 0.5);
        assert_eq!(result, vec![0, 1, 0, 1]);
    }

    #[test]
    fn three_distinct_speakers_get_three_clusters_in_appearance_order() {
        let a = unit(vec![1.0, 0.0, 0.0, 0.0]);
        let b = unit(vec![0.0, 1.0, 0.0, 0.0]);
        let c = unit(vec![0.0, 0.0, 1.0, 0.0]);
        let embeddings = vec![b.clone(), a.clone(), c.clone(), a, b, c];
        let result = cluster_speakers(&embeddings, 0.5);
        // First-appearance order: b -> 0, a -> 1, c -> 2.
        assert_eq!(result, vec![0, 1, 2, 1, 0, 2]);
    }

    #[test]
    fn high_threshold_keeps_similar_speakers_separate() {
        // Two embeddings with cosine similarity ~0.6 should not merge at 0.9.
        let a = unit(vec![1.0, 0.0]);
        let b = unit(vec![0.6, 0.8]);
        let result = cluster_speakers(&[a, b], 0.9);
        assert_eq!(result, vec![0, 1]);
    }

    #[test]
    fn speaker_label_alphabetical() {
        assert_eq!(speaker_label(0), "Speaker A");
        assert_eq!(speaker_label(1), "Speaker B");
        assert_eq!(speaker_label(25), "Speaker Z");
        assert_eq!(speaker_label(26), "Speaker AA");
        assert_eq!(speaker_label(27), "Speaker AB");
    }
}
