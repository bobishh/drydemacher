#![deny(unsafe_op_in_unsafe_fn)]

#[derive(Debug, Clone, PartialEq)]
pub struct SolveEvidence {
    pub solution: Vec<f64>,
    pub factor_time_ms: f64,
    pub solve_time_ms: f64,
}

#[repr(C)]
struct NativeEvidence {
    status: i32,
    factor_ms: f64,
    solve_ms: f64,
}

unsafe extern "C" {
    fn ecky_accelerate_sparse_solve(
        dimension: i32,
        nonzero_count: isize,
        rows: *const i32,
        columns: *const i32,
        values: *const f64,
        rhs_count: i32,
        rhs: *const f64,
        solution: *mut f64,
    ) -> NativeEvidence;
}

pub fn solve_symmetric_upper(
    dimension: usize,
    rows: &[i32],
    columns: &[i32],
    values: &[f64],
    right_hand_sides_column_major: &[f64],
    rhs_count: usize,
) -> Result<SolveEvidence, String> {
    if dimension == 0
        || dimension > i32::MAX as usize
        || rhs_count == 0
        || rhs_count > i32::MAX as usize
        || rows.len() != columns.len()
        || rows.len() != values.len()
        || right_hand_sides_column_major.len() != dimension.saturating_mul(rhs_count)
    {
        return Err("invalid Accelerate sparse dimensions".into());
    }
    let mut solution = vec![0.0; right_hand_sides_column_major.len()];
    let evidence = unsafe {
        ecky_accelerate_sparse_solve(
            dimension as i32,
            values.len() as isize,
            rows.as_ptr(),
            columns.as_ptr(),
            values.as_ptr(),
            rhs_count as i32,
            right_hand_sides_column_major.as_ptr(),
            solution.as_mut_ptr(),
        )
    };
    if evidence.status != 0 {
        return Err(format!(
            "Accelerate sparse Cholesky status {}",
            evidence.status
        ));
    }
    Ok(SolveEvidence {
        solution,
        factor_time_ms: evidence.factor_ms,
        solve_time_ms: evidence.solve_ms,
    })
}
