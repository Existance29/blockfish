use crate::BasicMatrix;
use red_union_find::UF;
use std::ops::Range;

// Parameters

#[derive(Clone, Debug)]
pub struct ScoreParams {
    pub row_factor: i64,
    pub piece_estimate_factor: i64,
    pub piece_penalty: i64,
}

impl Default for ScoreParams {
    fn default() -> Self {
        Self {
            row_factor: 0,
            piece_estimate_factor: 3,
            piece_penalty: 4,
        }
    }
}

/// Computes the "score" for the given snapshot. Lower is better.
///
/// Note: used by A* to compute "h" value (remaining cost heuristic).
pub fn score(params: &ScoreParams, matrix: &BasicMatrix) -> i64 {
    let mut matrix = matrix.clone();
    matrix.insert_empty_bottom_row();

    let mut score = 0;
    let mut depth = 0;

    let mut residue_buf = ResidueBuf::new();
    while let Some((i, res)) = covered_hole(&matrix, &mut residue_buf) {
        let rows = (i + 1)..res.end;
        let pieces: i64 = negative_spaces(&matrix, rows.clone())
            .map(|area| ((area + 3) / 4) as i64)
            .sum();

        matrix.remove_rows(rows);

        score += std::cmp::max(1, pieces - depth);
        depth += 1;
    }

    score * params.piece_estimate_factor + (matrix.rows() as i64) * params.row_factor
}

/// Computes the "penalty" for placing the given number of pieces.
///
/// Note: used in A* to compute "g" value (path cost).
pub fn penalty(params: &ScoreParams, depth: usize) -> i64 {
    (depth as i64) * params.piece_penalty
}

/// Returns the area of each disjoint contiguous negative space in the given matrix.
fn negative_spaces<'a>(
    matrix: &'a BasicMatrix,
    row_range: Range<u16>,
) -> impl Iterator<Item = u16> + 'a {
    gaps_contiguous_areas(row_range.map(move |i| matrix.gaps(i)))
}

/// Returns the size of each contiguous area given by the overlapping, neighboring ranges
/// in `iter`.
///
/// # Example
///
/// If `iter` is `[[0..5, 7..10], [0..4], [2..3]]` returns `[10, 3]`. `10` is given by the
/// set of intervals `{0..5,0..4,2..3}` which all overlap; `3` is given by the last
/// interval `7..10`.
fn gaps_contiguous_areas<I>(iter: I) -> impl Iterator<Item = u16> + 'static
where
    I: IntoIterator,
    I::Item: IntoIterator<Item = Range<u16>>,
{
    let iter = iter.into_iter();
    let size_hint = iter.size_hint().1.unwrap_or(0);
    let mut gaps = Vec::with_capacity(size_hint * 2);
    let mut row_end_idxs = Vec::with_capacity(size_hint);
    for row_gaps in iter {
        gaps.extend(row_gaps);
        row_end_idxs.push(gaps.len());
    }

    let mut uf = UF::new_reflexive(gaps.len());
    let mut idx0 = std::usize::MAX;
    let mut idx1 = 0;
    for idx2 in row_end_idxs {
        if idx0 < std::usize::MAX {
            let row1 = &gaps[idx0..idx1];
            let row2 = &gaps[idx1..idx2];
            for (i0, i1) in intersecting_ranges(row1, row2) {
                uf.union(idx0 + i0, idx1 + i1);
            }
        }
        idx0 = idx1;
        idx1 = idx2;
    }

    let mut areas = vec![0; gaps.len()];
    for (i, gap) in gaps.into_iter().enumerate() {
        areas[uf.find(i)] += gap.end - gap.start;
    }

    areas.into_iter().filter(|&a| a > 0)
}

/// Given `xs` and `ys` both ordered lists of non-overlapping ranges, returns every pair
/// of indices `(i, j)` such that `xs[i]` intersects with `ys[j]`.
fn intersecting_ranges<'a, T: Ord>(
    xs: &'a [Range<T>],
    ys: &'a [Range<T>],
) -> impl Iterator<Item = (usize, usize)> + 'a {
    let (mut i1, mut i2) = (0, 0);
    std::iter::from_fn(move || loop {
        let r1 = xs.get(i1)?;
        let r2 = ys.get(i2)?;
        if r2.start >= r1.end {
            i1 += 1;
        } else if r1.start >= r2.end {
            i2 += 1;
        } else if r2.end >= r1.end {
            i1 += 1;
            return Some((i1 - 1, i2));
        } else {
            i2 += 1;
            return Some((i1, i2 - 1));
        }
    })
}

type ResidueBuf = Vec<Range<u16>>;

/// Searches for a hole covered by residue. If any is found returns `Some((i, r))` where
/// `i` is the row containing the hole and `r` is the (half-open) range of rows above `i`
/// containing the residue.
///
/// `buf` is used for internal bookkeeping to perform the algorithm, and ought to be
/// reused to save on allocations.
///
/// Example:
///
/// 5 | . . . . .
/// 4 | . x . . .  <-
/// 3 | x x x . .  <- r
/// 2 | x _ x x x
/// 1 | x _ x x x  <- i
/// 0 | x x x _ x         <= note: this is a valid hole, but the one above is found first.
///
/// Returns `Some((1, 3..5))`.
#[allow(unused)]
fn covered_hole(mat: &BasicMatrix, buf: &mut ResidueBuf) -> Option<(u16, Range<u16>)> {
    let (n_rows, n_cols) = (mat.rows(), mat.cols());
    // `buf[j]` holds the known range of residue cells in column `j`
    buf.clear();
    buf.resize(n_cols as usize, 0..0);
    for i in (0..n_rows).rev() {
        for j in 0..n_cols {
            // do nothing for empty cells
            if !mat.get((i, j)) {
                continue;
            }
            let res = &mut buf[j as usize];
            if res.start > i + 1 {
                // prev cell was empty, so we finished finding a hole
                return Some((i + 1, res.clone()));
            } else {
                // update the residue since the cell above was also filled
                *res = i..std::cmp::max(i + 1, res.end);
            }
        }
    }
    // run a simplified version of the inner loop for underneath the bottom row, i.e. i=-1
    for j in 0..n_cols {
        let res = &buf[j as usize];
        if res.start > 0 {
            return Some((0, res.clone()));
        }
    }
    None
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::basic_matrix;

    #[test]
    fn test_intersecting_ranges() {
        let irs = |xs: &[Range<i32>], ys| intersecting_ranges(xs, ys).collect::<Vec<_>>();
        // 0 1 2 3 4 5 6 7 8 9 10 11 12 13
        // {-----}     {-----------}    {----
        //     {-----} {-}   {--------}
        //     {#}     {#}   {#####}
        let xs = [0..3, 6..11, 13..20];
        let ys = [2..5, 6..7, 9..12];
        assert_eq!(irs(&xs, &ys), [(0, 0), (1, 1), (1, 2)]);
        assert_eq!(irs(&ys, &xs), [(0, 0), (1, 1), (2, 1)]);
        assert_eq!(irs(&xs, &[]), []);
        assert_eq!(irs(&[], &xs), []);
        assert_eq!(irs(&xs, &[10..15]), [(1, 0), (2, 0)]);
        assert_eq!(irs(&xs, &[11..15]), [(2, 0)]);
    }

    fn neg_space(mat: BasicMatrix) -> Vec<u16> {
        let mut nss = negative_spaces(&mat, 0..mat.rows()).collect::<Vec<_>>();
        nss.sort();
        nss
    }

    #[test]
    fn test_negative_spaces_1() {
        let (xx, __) = (true, false);
        assert_eq!(
            neg_space(basic_matrix![
                [xx, xx, xx, xx, xx, __],
                [xx, __, xx, xx, xx, xx],
                [xx, xx, __, xx, xx, xx],
            ]),
            [1, 1, 1],
            "cheese"
        );
        assert_eq!(
            neg_space(basic_matrix![
                [xx, __, xx, xx, xx, xx],
                [xx, xx, __, __, xx, xx],
            ]),
            [1, 2],
            "2w gap"
        );
    }

    #[test]
    fn test_negative_spaces_2() {
        let (xx, __) = (true, false);
        assert_eq!(
            neg_space(basic_matrix![
                [xx, __, __, __, xx, xx],
                [xx, __, xx, xx, xx, xx],
                [xx, xx, xx, __, __, xx],
            ]),
            [2, 4],
            "overlap (depth=1)"
        );
        assert_eq!(
            neg_space(basic_matrix![
                [xx, xx, xx, __, __, xx],
                [xx, __, xx, xx, xx, xx],
                [xx, __, __, __, xx, xx],
            ]),
            [2, 4],
            "overlap (depth=1, inverted)"
        );
        assert_eq!(
            neg_space(basic_matrix![
                [xx, __, __, __, xx, xx],
                [xx, __, xx, __, __, xx],
                [__, xx, __, xx, xx, xx],
            ]),
            [1, 1, 6],
            "overlap (depth=2)"
        );
        assert_eq!(
            neg_space(basic_matrix![
                [__, xx, __, xx, xx, xx],
                [xx, __, xx, __, __, xx],
                [xx, __, __, __, xx, xx],
            ]),
            [1, 1, 6],
            "overlap (depth=2, inverted)"
        );
    }

    #[test]
    fn test_negative_spaces_3() {
        let (xx, __) = (true, false);
        assert_eq!(
            neg_space(basic_matrix![
                [xx, __, __, __, xx, __],
                [__, __, xx, __, xx, xx],
                [xx, __, xx, __, __, xx],
            ]),
            [1, 9],
            "overlap (depth=3)"
        );
        assert_eq!(
            neg_space(basic_matrix![
                [xx, __, xx, __, __, xx],
                [__, __, xx, __, xx, xx],
                [xx, __, __, __, xx, __],
            ]),
            [1, 9],
            "overlap (depth=3, ivnerted)"
        );
    }

    #[test]
    fn test_negative_spaces_all_clear() {
        assert_eq!(neg_space(BasicMatrix::with_cols(5)), [0u16; 0]);
    }

    #[test]
    fn test_covered_hole_0() {
        let (xx, __) = (true, false);
        assert_eq!(
            covered_hole(&BasicMatrix::with_cols(5), &mut Default::default()),
            None,
            "zero"
        );
        assert_eq!(
            covered_hole(
                &basic_matrix![[xx, xx, xx, xx, xx, xx], [xx, xx, xx, xx, xx, xx],],
                &mut Default::default()
            ),
            None,
            "full",
        );
    }

    #[test]
    fn test_covered_hole_1() {
        let (xx, rr, __) = (true, true, false);
        let mut rbuf = ResidueBuf::default();
        assert_eq!(
            covered_hole(
                &basic_matrix![
                    [xx, xx, xx, xx, xx, xx],
                    [__, xx, xx, xx, xx, xx],
                    [rr, xx, xx, xx, xx, xx],
                ],
                &mut rbuf
            ),
            Some((1, 2..3)),
            "cheese h=1"
        );
        assert_eq!(
            covered_hole(
                &basic_matrix![
                    [__, xx, xx, xx, xx, xx],
                    [rr, xx, __, xx, xx, xx],
                    [rr, xx, rr, xx, __, xx],
                    [rr, xx, rr, xx, rr, xx],
                    [rr, xx, rr, xx, rr, xx],
                ],
                &mut rbuf
            ),
            Some((2, 3..5)),
            "cheese h=2"
        );
        assert_eq!(
            covered_hole(
                &basic_matrix![
                    [__, xx, xx, xx, xx, xx],
                    [rr, xx, xx, __, xx, xx],
                    [rr, xx, xx, __, xx, xx],
                    [rr, xx, xx, rr, xx, xx],
                    [rr, xx, xx, rr, xx, xx],
                ],
                &mut rbuf
            ),
            Some((1, 3..5)),
            "cheese d=2"
        );
        assert_eq!(
            covered_hole(
                &basic_matrix![
                    [__, xx, xx, xx, xx, xx],
                    [rr, xx, xx, __, xx, xx],
                    [rr, xx, xx, rr, xx, __],
                    [rr, xx, xx, rr, xx, xx],
                    [rr, xx, xx, rr, xx, __],
                ],
                &mut rbuf
            ),
            Some((2, 3..4)),
            "reuse rbuf"
        );
    }

    #[test]
    fn test_covered_hole_2() {
        let (xx, rr, __) = (true, true, false);
        assert_eq!(
            covered_hole(
                &basic_matrix![
                    [xx, xx, xx, __, xx, xx],
                    [xx, xx, __, __, xx, xx],
                    [xx, __, __, __, __, xx],
                ],
                &mut Default::default()
            ),
            None,
            "funnel"
        );
        assert_eq!(
            covered_hole(
                &basic_matrix![
                    [xx, xx, xx, __, xx, xx],
                    [__, __, __, rr, __, __],
                    [__, __, __, rr, __, __],
                    [__, __, __, rr, __, __],
                ],
                &mut Default::default()
            ),
            Some((0, 1..4)),
            "spike"
        );
        assert_eq!(
            covered_hole(
                &basic_matrix![
                    [xx, xx, xx, __, xx, xx],
                    [__, __, __, rr, __, __],
                    [__, __, __, rr, __, __],
                    [__, __, rr, rr, __, __],
                ],
                &mut Default::default()
            ),
            Some((1, 3..4)),
            "overhang"
        );
    }

    #[test]
    fn test_covered_hole_3() {
        let (xx, rr, __) = (true, true, false);
        assert_eq!(
            covered_hole(
                &basic_matrix![
                    [xx, xx, xx, xx, xx, xx],
                    [xx, __, xx, __, xx, xx],
                    [xx, rr, xx, rr, xx, xx],
                ],
                &mut Default::default()
            ),
            Some((1, 2..3)),
            "double"
        );
        // assert_eq!(
        //     covered_hole(
        //         &basic_matrix![
        //             [xx, xx, xx, xx, xx, xx],
        //             [xx, __, xx, __, xx, xx],
        //             [xx, rr, xx, rr, xx, xx],
        //             [__, __, __, rr, __, __],
        //         ],
        //         &mut Default::default()
        //     ),
        //     Some((1, 2..4)),
        //     "double, right has greater residue"
        // );
        // assert_eq!(
        //     covered_hole(
        //         &basic_matrix![
        //             [xx, xx, xx, __, xx, xx],
        //             [xx, __, xx, __, xx, xx],
        //             [xx, rr, xx, rr, xx, xx],
        //         ],
        //         &mut Default::default()
        //     ),
        //     Some((0, 2..3)),
        //     "double, right has deeper residue"
        // );
    }
}
