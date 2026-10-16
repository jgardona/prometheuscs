// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp.
//! Dense LU factorization using Crout's method with partial pivoting.

/// Factors the `n` x `n` matrix `a` in place. `ipvt` receives the pivot rows used by
/// [`lu_solve`]. Returns false when the matrix is singular.
pub fn lu_factor(a: &mut [Vec<f64>], n: usize, ipvt: &mut [usize]) -> bool {
    // a row of all zeroes means the matrix is singular
    if a.iter().take(n).any(|row| row.iter().take(n).all(|&v| v == 0.0)) {
        return false;
    }
    for j in 0..n {
        for i in 0..j {
            let mut q = a[i][j];
            for k in 0..i {
                q -= a[i][k] * a[k][j];
            }
            a[i][j] = q;
        }
        let mut largest = 0.0;
        let mut largest_row: Option<usize> = None;
        for i in j..n {
            let mut q = a[i][j];
            for k in 0..j {
                q -= a[i][k] * a[k][j];
            }
            a[i][j] = q;
            let x = q.abs();
            if x >= largest {
                largest = x;
                largest_row = Some(i);
            }
        }
        let Some(largest_row) = largest_row else {
            return false;
        };
        if j != largest_row {
            a.swap(largest_row, j);
        }
        ipvt[j] = largest_row;
        if a[j][j] == 0.0 {
            return false;
        }
        if j != n - 1 {
            let mult = 1.0 / a[j][j];
            for row in a.iter_mut().take(n).skip(j + 1) {
                row[j] *= mult;
            }
        }
    }
    true
}

/// Solves the system using the factorization made by [`lu_factor`]. On input `b` is the
/// right-hand side, on output it holds the solution.
pub fn lu_solve(a: &[Vec<f64>], n: usize, ipvt: &[usize], b: &mut [f64]) {
    let mut i = 0;
    while i < n {
        let row = ipvt[i];
        b.swap(row, i);
        if b[i] != 0.0 {
            break;
        }
        i += 1;
    }
    let bi = i;
    i += 1;
    while i < n {
        let row = ipvt[i];
        let mut tot = b[row];
        b[row] = b[i];
        for j in bi..i {
            tot -= a[i][j] * b[j];
        }
        b[i] = tot;
        i += 1;
    }
    for i in (0..n).rev() {
        let mut tot = b[i];
        for j in i + 1..n {
            tot -= a[i][j] * b[j];
        }
        b[i] = tot / a[i][i];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_system_requiring_row_swap() {
        let mut a = vec![vec![0.0, 2.0], vec![1.0, 1.0]];
        let orig = a.clone();
        let mut ipvt = vec![0; 2];
        assert!(lu_factor(&mut a, 2, &mut ipvt));
        let mut b = vec![4.0, 3.0];
        lu_solve(&a, 2, &ipvt, &mut b);
        assert!((orig[0][0] * b[0] + orig[0][1] * b[1] - 4.0).abs() < 1e-12);
        assert!((orig[1][0] * b[0] + orig[1][1] * b[1] - 3.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_zero_row() {
        let mut a = vec![vec![1.0, 1.0], vec![0.0, 0.0]];
        let mut ipvt = vec![0; 2];
        assert!(!lu_factor(&mut a, 2, &mut ipvt));
    }
}
