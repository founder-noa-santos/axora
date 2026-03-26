//! Pure f32 vector operations for research retrieval (no SQLite, no I/O).
//! Used by [`crate::storage`] and unit-tested for mathematical correctness.

/// Dot product with 8-wide unrolled tail (contiguous; LLVM can vectorize the inner loop).
#[must_use]
#[inline]
pub fn dot_f32(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let len = a.len();
    let mut sum = 0f32;
    let mut i = 0;
    while i + 8 <= len {
        sum += a[i] * b[i]
            + a[i + 1] * b[i + 1]
            + a[i + 2] * b[i + 2]
            + a[i + 3] * b[i + 3]
            + a[i + 4] * b[i + 4]
            + a[i + 5] * b[i + 5]
            + a[i + 6] * b[i + 6]
            + a[i + 7] * b[i + 7];
        i += 8;
    }
    while i < len {
        sum += a[i] * b[i];
        i += 1;
    }
    sum
}

/// Squared L2 norm (avoids an extra sqrt when only comparing cosines).
#[must_use]
#[inline]
pub fn l2_norm_squared(v: &[f32]) -> f32 {
    dot_f32(v, v)
}

#[must_use]
#[inline]
pub fn l2_norm(v: &[f32]) -> f32 {
    l2_norm_squared(v).sqrt()
}

/// Cosine similarity for equal-length vectors. Returns `0.0` if either norm is zero or non-finite.
/// Does **not** assume inputs are normalized (full `q·d / (‖q‖‖d‖)`).
#[must_use]
pub fn cosine_similarity(q: &[f32], d: &[f32]) -> f32 {
    if q.len() != d.len() {
        return 0.0;
    }
    let qn = l2_norm(q);
    let dn = l2_norm(d);
    cosine_similarity_with_norms(q, qn, d, dn)
}

/// Same as [`cosine_similarity`] but reuses precomputed `‖q‖` and `‖d‖` (must match vectors).
#[must_use]
#[inline]
pub fn cosine_similarity_with_norms(q: &[f32], q_norm: f32, d: &[f32], d_norm: f32) -> f32 {
    if q.len() != d.len() {
        return 0.0;
    }
    if q_norm == 0.0 || d_norm == 0.0 || !q_norm.is_finite() || !d_norm.is_finite() {
        return 0.0;
    }
    let dot = dot_f32(q, d);
    if !dot.is_finite() {
        return 0.0;
    }
    let s = dot / (q_norm * d_norm);
    if s.is_finite() {
        s
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: f32, b: f32, eps: f32) {
        assert!(
            (a - b).abs() < eps,
            "expected {a} ≈ {b}, diff {}",
            (a - b).abs()
        );
    }

    #[test]
    fn dot_orthogonal_unit_vectors_dim3() {
        let a = [1f32, 0., 0.];
        let b = [0f32, 1., 0.];
        assert_close(dot_f32(&a, &b), 0.0, 1e-7);
        assert_close(dot_f32(&a, &a), 1.0, 1e-7);
    }

    #[test]
    fn dot_known_small_vector() {
        let a = [1f32, 2., 3.];
        let b = [4f32, 5., 6.];
        assert_close(dot_f32(&a, &b), 32.0, 1e-6); // 4+10+18
    }

    #[test]
    fn cosine_identical_unit_vectors() {
        let v = [3f32, 4., 0., 0., 0.];
        let n = l2_norm(&v);
        let u = [v[0] / n, v[1] / n, 0., 0., 0.];
        assert_close(cosine_similarity(&u, &u), 1.0, 1e-6);
    }

    #[test]
    fn cosine_orthogonal() {
        let a = [1f32, 0., 0., 0.];
        let b = [0f32, 1., 0., 0.];
        assert_close(cosine_similarity(&a, &b), 0.0, 1e-7);
    }

    #[test]
    fn cosine_opposite_direction() {
        let a = [1f32, 0., 0.];
        let b = [-1f32, 0., 0.];
        assert_close(cosine_similarity(&a, &b), -1.0, 1e-6);
    }

    #[test]
    fn cosine_arbitrary_pair_hand_computed() {
        // q = (2,0,0), d = (1,1,0) -> cos = 2 / (2 * sqrt(2)) = 1/sqrt(2)
        let q = [2f32, 0., 0.];
        let d = [1f32, 1., 0.];
        let expected = 1.0f32 / 2.0f32.sqrt();
        assert_close(cosine_similarity(&q, &d), expected, 1e-5);
    }

    #[test]
    fn cosine_zero_vector_returns_zero() {
        let z = [0f32; 5];
        let v = [1f32, 0., 0., 0., 0.];
        assert_eq!(cosine_similarity(&z, &v), 0.0);
        assert_eq!(cosine_similarity(&v, &z), 0.0);
    }

    #[test]
    fn dot_length_mismatch_returns_zero_in_release() {
        let a = [1f32, 2.];
        let b = [1f32];
        assert_eq!(dot_f32(&a, &b), 0.0);
    }

    #[test]
    fn cosine_with_precomputed_norms_matches_full() {
        let q = [3f32, 4., 0., 0.];
        let d = [5f32, 0., 12., 0.];
        let full = cosine_similarity(&q, &d);
        let qn = l2_norm(&q);
        let dn = l2_norm(&d);
        let pre = cosine_similarity_with_norms(&q, qn, &d, dn);
        assert_close(full, pre, 1e-5);
    }

    #[test]
    fn l2_norm_example() {
        let v = [3f32, 4.];
        assert_close(l2_norm(&v), 5.0, 1e-6);
    }

    #[test]
    fn dot_chunked_tail_not_multiple_of_eight() {
        let mut a = vec![1f32; 13];
        let b = vec![2f32; 13];
        let expected: f32 = 13.0 * 2.0;
        assert_close(dot_f32(&a, &b), expected, 1e-5);
        a.push(1.0);
        let mut b2 = b;
        b2.push(2.0);
        assert_close(dot_f32(&a, &b2), expected + 2.0, 1e-5);
    }
}
