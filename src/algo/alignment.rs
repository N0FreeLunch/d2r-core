pub struct BitAligner {
    pub match_score: i32,
    pub mismatch_penalty: i32,
    pub gap_open: i32,
    pub gap_extend: i32,
}

pub struct AlignmentResult {
    pub score: i32,
    pub actual_aligned: Vec<Option<bool>>,
    pub expected_aligned: Vec<Option<bool>>,
    pub gap_indices: Vec<usize>,
}

impl AlignmentResult {
    /// Returns aligned pair as two strings, gaps shown as '-'
    pub fn pretty_print(&self) -> String {
        let mut actual_str = String::new();
        let mut expect_str = String::new();

        for bit in &self.actual_aligned {
            match bit {
                Some(true) => actual_str.push('1'),
                Some(false) => actual_str.push('0'),
                None => actual_str.push('-'),
            }
        }

        for bit in &self.expected_aligned {
            match bit {
                Some(true) => expect_str.push('1'),
                Some(false) => expect_str.push('0'),
                None => expect_str.push('-'),
            }
        }

        format!("ACTUAL: {}\nEXPECT: {}", actual_str, expect_str)
    }

    /// Returns percentage of matching positions over aligned length.
    pub fn similarity_pct(&self) -> f64 {
        if self.actual_aligned.is_empty() {
            return 0.0;
        }

        let mut match_count = 0;
        for i in 0..self.actual_aligned.len() {
            if self.actual_aligned[i] == self.expected_aligned[i] && self.actual_aligned[i].is_some() {
                match_count += 1;
            }
        }

        (match_count as f64 / self.actual_aligned.len() as f64) * 100.0
    }
}

impl BitAligner {
    pub fn new(match_score: i32, mismatch_penalty: i32, gap_open: i32, gap_extend: i32) -> Self {
        Self {
            match_score,
            mismatch_penalty,
            gap_open,
            gap_extend,
        }
    }

    pub fn align(&self, actual: &[bool], expected: &[bool]) -> AlignmentResult {
        let n = actual.len();
        let m = expected.len();
        let neg_inf = -1_000_000;

        // m_matrix[i][j]: score of best alignment of actual[..i] and expected[..j] ending with match/mismatch
        // x_matrix[i][j]: score of best alignment of actual[..i] and expected[..j] ending with gap in actual
        // y_matrix[i][j]: score of best alignment of actual[..i] and expected[..j] ending with gap in expected
        let mut m_matrix = vec![vec![neg_inf; m + 1]; n + 1];
        let mut x_matrix = vec![vec![neg_inf; m + 1]; n + 1];
        let mut y_matrix = vec![vec![neg_inf; m + 1]; n + 1];

        m_matrix[0][0] = 0;

        for i in 1..=n {
            y_matrix[i][0] = self.gap_open + (i as i32 - 1) * self.gap_extend;
        }
        for j in 1..=m {
            x_matrix[0][j] = self.gap_open + (j as i32 - 1) * self.gap_extend;
        }

        for i in 1..=n {
            for j in 1..=m {
                let s_ij = if actual[i - 1] == expected[j - 1] {
                    self.match_score
                } else {
                    self.mismatch_penalty
                };

                m_matrix[i][j] = s_ij
                    + i32::max(
                        m_matrix[i - 1][j - 1],
                        i32::max(x_matrix[i - 1][j - 1], y_matrix[i - 1][j - 1]),
                    );

                x_matrix[i][j] = i32::max(
                    m_matrix[i][j - 1] + self.gap_open,
                    x_matrix[i][j - 1] + self.gap_extend,
                );

                y_matrix[i][j] = i32::max(
                    m_matrix[i - 1][j] + self.gap_open,
                    y_matrix[i - 1][j] + self.gap_extend,
                );
            }
        }

        let mut i = n;
        let mut j = m;
        let mut actual_aligned = Vec::new();
        let mut expected_aligned = Vec::new();

        let final_score = i32::max(
            m_matrix[i][j],
            i32::max(x_matrix[i][j], y_matrix[i][j]),
        );

        let mut current_state = if final_score == m_matrix[i][j] {
            0 // M
        } else if final_score == x_matrix[i][j] {
            1 // X
        } else {
            2 // Y
        };

        while i > 0 || j > 0 {
            match current_state {
                0 => {
                    // M
                    actual_aligned.push(Some(actual[i - 1]));
                    expected_aligned.push(Some(expected[j - 1]));
                    let prev_score = m_matrix[i][j]
                        - (if actual[i - 1] == expected[j - 1] {
                            self.match_score
                        } else {
                            self.mismatch_penalty
                        });
                    i -= 1;
                    j -= 1;
                    if i > 0 || j > 0 {
                        if i > 0 && j > 0 && prev_score == m_matrix[i][j] {
                            current_state = 0;
                        } else if j > 0 && prev_score == x_matrix[i][j] {
                            current_state = 1;
                        } else if i > 0 && prev_score == y_matrix[i][j] {
                            current_state = 2;
                        } else {
                            // Boundary conditions or minimal score case
                            if i > 0 && j > 0 {
                                // Prefer M if tied at boundary
                                current_state = 0;
                            } else if j > 0 {
                                current_state = 1;
                            } else {
                                current_state = 2;
                            }
                        }
                    }
                }
                1 => {
                    // X: Gap in actual
                    actual_aligned.push(None);
                    expected_aligned.push(Some(expected[j - 1]));
                    let is_open = x_matrix[i][j] == m_matrix[i][j - 1] + self.gap_open;
                    j -= 1;
                    if is_open {
                        current_state = 0;
                    } else {
                        current_state = 1;
                    }
                }
                2 => {
                    // Y: Gap in expected
                    actual_aligned.push(Some(actual[i - 1]));
                    expected_aligned.push(None);
                    let is_open = y_matrix[i][j] == m_matrix[i - 1][j] + self.gap_open;
                    i -= 1;
                    if is_open {
                        current_state = 0;
                    } else {
                        current_state = 2;
                    }
                }
                _ => unreachable!(),
            }
        }

        actual_aligned.reverse();
        expected_aligned.reverse();

        let mut gap_indices = Vec::new();
        for k in 0..actual_aligned.len() {
            if actual_aligned[k].is_none() || expected_aligned[k].is_none() {
                gap_indices.push(k);
            }
        }

        AlignmentResult {
            score: final_score,
            actual_aligned,
            expected_aligned,
            gap_indices,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_bit_insertion_detected() {
        let aligner = BitAligner::new(2, -1, -3, -1);
        // expected has an extra 0 at index 2
        let actual = vec![true, false, true, false, true];
        let expected = vec![true, false, false, true, false, true];
        let result = aligner.align(&actual, &expected);
        // gap must appear at a single position
        assert_eq!(result.gap_indices.len(), 1);
        assert!(result.score > 0);
    }

    #[test]
    fn test_pretty_print_shows_gap() {
        let aligner = BitAligner::new(2, -1, -3, -1);
        let actual = vec![true, false, true, false, true];
        let expected = vec![true, false, false, true, false, true];
        let result = aligner.align(&actual, &expected);
        let s = result.pretty_print();
        // gap character '-' must appear in ACTUAL line
        assert!(s.contains('-'));
    }

    #[test]
    fn test_similarity_identical() {
        let aligner = BitAligner::new(2, -1, -3, -1);
        let seq = vec![true, false, true];
        let result = aligner.align(&seq, &seq);
        assert!((result.similarity_pct() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_identical_sequences_perfect_score() {
        let aligner = BitAligner::new(2, -1, -3, -1);
        let seq = vec![true, false, true];
        let result = aligner.align(&seq, &seq);
        assert!(result.gap_indices.is_empty());
        assert_eq!(result.score, 3 * 2); // all match
    }
}
