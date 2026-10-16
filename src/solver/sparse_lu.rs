// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from EJML (Efficient Java Matrix Library), Copyright (c) 2009-2020 Peter Abeles,
// Apache License 2.0, as adapted for CircuitJS1 (sharpie7/circuitjs1 PR #920).
//! Sparse LU factorization (left-looking, partial pivoting, CSparse style).

/// A square sparse matrix in compressed sparse column form.
#[derive(Debug, Clone, Default)]
pub struct Csc {
    num_cols: usize,
    rows: Vec<usize>,
    values: Vec<f64>,
    col_idx: Vec<usize>,
}

impl Csc {
    /// Builds a sparse matrix from a dense row-major matrix, dropping entries with
    /// `|value| <= tol`.
    pub fn from_dense(a: &[Vec<f64>], tol: f64) -> Self {
        let n = a.len();
        let mut m = Csc { num_cols: n, col_idx: vec![0; n + 1], ..Csc::default() };
        for i in 0..n {
            for (j, row) in a.iter().enumerate() {
                let value = row[i];
                if value.abs() > tol || value.is_nan() {
                    m.rows.push(j);
                    m.values.push(value);
                }
            }
            m.col_idx[i + 1] = m.rows.len();
        }
        m
    }

    /// Number of stored entries.
    pub fn nz_length(&self) -> usize {
        self.rows.len()
    }

    /// Number of columns.
    pub fn num_cols(&self) -> usize {
        self.num_cols
    }

    fn push(&mut self, row: usize, value: f64) {
        self.rows.push(row);
        self.values.push(value);
    }
}

/// Sparse LU factorization of a square matrix.
#[derive(Debug, Default)]
pub struct SparseLu {
    l: Csc,
    u: Csc,
    pinv: Vec<i64>,
    x: Vec<f64>,
    xi: Vec<usize>,
    w: Vec<i64>,
    n: usize,
    scratch_b: Vec<f64>,
}

impl SparseLu {
    /// Creates an empty factorization.
    pub fn new() -> Self {
        Self::default()
    }

    /// Factors `a`. Returns false when the matrix is singular.
    pub fn set_a(&mut self, a: &Csc) -> bool {
        let n = a.num_cols;
        self.n = n;
        self.l = Csc { num_cols: n, col_idx: vec![0; n + 1], ..Csc::default() };
        self.u = Csc { num_cols: n, col_idx: vec![0; n + 1], ..Csc::default() };
        self.pinv = vec![-1; n];
        self.x = vec![0.0; n];
        self.xi = vec![0; n];
        self.w = vec![0; 2 * n];
        self.perform_lu(a)
    }

    fn perform_lu(&mut self, a: &Csc) -> bool {
        let n = self.n;
        for k in 0..n {
            self.l.col_idx[k] = self.l.rows.len();
            self.u.col_idx[k] = self.u.rows.len();
            self.w[..n].fill(0);
            let top = solve_col_b(
                &self.l,
                true,
                a,
                k,
                &mut self.x,
                &self.pinv,
                &mut self.xi,
                &mut self.w,
            );
            let mut ipiv: Option<usize> = None;
            let mut best = f64::MIN;
            for p in top..n {
                let i = self.xi[p];
                if self.pinv[i] < 0 {
                    let t = self.x[i].abs();
                    if t > best {
                        best = t;
                        ipiv = Some(i);
                    }
                } else {
                    self.u.push(self.pinv[i] as usize, self.x[i]);
                }
            }
            let Some(ipiv) = ipiv else { return false };
            if best <= 0.0 || best.is_nan() {
                return false;
            }
            let pivot = self.x[ipiv];
            self.u.push(k, pivot);
            self.pinv[ipiv] = k as i64;
            self.l.push(ipiv, 1.0);
            for p in top..n {
                let i = self.xi[p];
                if self.pinv[i] < 0 {
                    self.l.push(i, self.x[i] / pivot);
                }
                self.x[i] = 0.0;
            }
        }
        self.l.col_idx[n] = self.l.rows.len();
        self.u.col_idx[n] = self.u.rows.len();
        for row in &mut self.l.rows {
            *row = self.pinv[*row] as usize;
        }
        true
    }

    /// Solves `A x = b` in place using the last factorization.
    pub fn solve(&mut self, b: &mut [f64]) {
        let n = self.n;
        debug_assert_eq!(b.len(), n);
        self.scratch_b.clear();
        self.scratch_b.extend_from_slice(b);
        for k in 0..n {
            b[self.pinv[k] as usize] = self.scratch_b[k];
        }
        solve_l(&self.l, b);
        solve_u(&self.u, b);
    }
}

#[allow(clippy::too_many_arguments)]
fn solve_col_b(
    g: &Csc,
    lower: bool,
    b: &Csc,
    col_b: usize,
    x: &mut [f64],
    pinv: &[i64],
    xi: &mut [usize],
    w: &mut [i64],
) -> usize {
    let x_rows = g.num_cols;
    let top = search_nz_rows_in_x(g, b, col_b, pinv, xi, w);
    for &row in &xi[top..x_rows] {
        x[row] = 0.0;
    }
    for px in b.col_idx[col_b]..b.col_idx[col_b + 1] {
        x[b.rows[px]] = b.values[px];
    }
    for px in top..x_rows {
        let j = xi[px];
        let jj = pinv[j];
        if jj >= 0 {
            let jj = jj as usize;
            let (p, q);
            if lower {
                x[j] /= g.values[g.col_idx[jj]];
                p = g.col_idx[jj] + 1;
                q = g.col_idx[jj + 1];
            } else {
                x[j] /= g.values[g.col_idx[jj + 1] - 1];
                p = g.col_idx[jj];
                q = g.col_idx[jj + 1] - 1;
            }
            for k in p..q {
                x[g.rows[k]] -= g.values[k] * x[j];
            }
        }
    }
    top
}

fn search_nz_rows_in_x(
    g: &Csc,
    b: &Csc,
    col_b: usize,
    pinv: &[i64],
    xi: &mut [usize],
    w: &mut [i64],
) -> usize {
    let x_rows = g.num_cols;
    let mut top = x_rows;
    for i in b.col_idx[col_b]..b.col_idx[col_b + 1] {
        let row_b = b.rows[i];
        if row_b < x_rows && w[row_b] == 0 {
            top = search_dfs(row_b, g, top, pinv, xi, w);
        }
    }
    for i in top..x_rows {
        w[xi[i]] = 0;
    }
    top
}

fn search_dfs(
    row_b: usize,
    g: &Csc,
    mut top: usize,
    pinv: &[i64],
    xi: &mut [usize],
    w: &mut [i64],
) -> usize {
    let n = g.num_cols;
    let mut head: isize = 0;
    xi[0] = row_b;
    while head >= 0 {
        let h = head as usize;
        let g_col = xi[h];
        let g_col_new = pinv[g_col];
        let in_range = g_col_new >= 0 && (g_col_new as usize) < n;
        if w[g_col] == 0 {
            w[g_col] = 1;
            w[n + h] = if in_range { g.col_idx[g_col_new as usize] as i64 } else { 0 };
        }
        let mut done = true;
        let idx0 = w[n + h] as usize;
        let idx1 = if in_range { g.col_idx[g_col_new as usize + 1] } else { 0 };
        for j in idx0..idx1 {
            let jrow = g.rows[j];
            if jrow < n && w[jrow] == 0 {
                w[n + h] = j as i64 + 1;
                head += 1;
                xi[head as usize] = jrow;
                done = false;
                break;
            }
        }
        if done {
            head -= 1;
            top -= 1;
            xi[top] = g_col;
        }
    }
    top
}

fn solve_l(l: &Csc, x: &mut [f64]) {
    let mut idx0 = l.col_idx[0];
    for col in 0..l.num_cols {
        let idx1 = l.col_idx[col + 1];
        x[col] /= l.values[idx0];
        let x_j = x[col];
        for i in idx0 + 1..idx1 {
            x[l.rows[i]] -= l.values[i] * x_j;
        }
        idx0 = idx1;
    }
}

fn solve_u(u: &Csc, x: &mut [f64]) {
    let mut idx1 = u.col_idx[u.num_cols];
    for col in (0..u.num_cols).rev() {
        let idx0 = u.col_idx[col];
        x[col] /= u.values[idx1 - 1];
        let x_j = x[col];
        for i in idx0..idx1 - 1 {
            x[u.rows[i]] -= u.values[i] * x_j;
        }
        idx1 = idx0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn residual(a: &[Vec<f64>], x: &[f64], b: &[f64]) -> f64 {
        a.iter()
            .zip(b)
            .map(|(row, bi)| (row.iter().zip(x).map(|(a, x)| a * x).sum::<f64>() - bi).abs())
            .fold(0.0, f64::max)
    }

    #[test]
    fn solves_small_system_with_pivoting() {
        let a = vec![vec![0.0, 2.0, 1.0], vec![1.0, 1.0, 0.0], vec![3.0, 0.0, 1.0]];
        let b = vec![5.0, 3.0, 7.0];
        let mut lu = SparseLu::new();
        assert!(lu.set_a(&Csc::from_dense(&a, 0.0)));
        let mut x = b.clone();
        lu.solve(&mut x);
        assert!(residual(&a, &x, &b) < 1e-12);
    }

    #[test]
    fn detects_singular_matrix() {
        let a = vec![vec![1.0, 2.0], vec![2.0, 4.0]];
        let mut lu = SparseLu::new();
        assert!(!lu.set_a(&Csc::from_dense(&a, 0.0)));
    }

    #[test]
    fn solves_tridiagonal_system() {
        let n = 200;
        let mut a = vec![vec![0.0; n]; n];
        for i in 0..n {
            a[i][i] = 2.0;
            if i > 0 {
                a[i][i - 1] = -1.0;
                a[i - 1][i] = -1.0;
            }
        }
        let b: Vec<f64> = (0..n).map(|i| (i % 7) as f64).collect();
        let mut lu = SparseLu::new();
        assert!(lu.set_a(&Csc::from_dense(&a, 0.0)));
        let mut x = b.clone();
        lu.solve(&mut x);
        assert!(residual(&a, &x, &b) < 1e-9);
    }
}
