#[derive(Debug, Clone, PartialEq)]
pub(super) struct Mma87History {
    pub previous: Vec<f64>,
    pub previous_previous: Vec<f64>,
    pub asymptote_widths: Vec<f64>,
    pub dual: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Mma87Step {
    pub design: Vec<f64>,
    pub history: Mma87History,
    pub approximate_objective: f64,
    pub approximate_constraint: f64,
}

// Clean-room specialization of the MMA87 equations documented by the
// MIT-licensed JuliaNonconvex/NonconvexMMA.jl implementation. One affine
// inequality leaves a scalar dual, so deterministic bisection is sufficient.
#[allow(clippy::too_many_arguments)]
pub(super) fn mma87_update(
    current: &[f64],
    objective_value: f64,
    objective_gradient: &[f64],
    constraint_value: f64,
    constraint_gradient: &[f64],
    objective_lift: f64,
    constraint_lift: f64,
    minimum: f64,
    maximum: f64,
    maximum_move: f64,
    iteration: usize,
    history: &Mma87History,
) -> Result<Mma87Step, &'static str> {
    let count = current.len();
    if count == 0
        || objective_gradient.len() != count
        || constraint_gradient.len() != count
        || history.previous.len() != count
        || history.previous_previous.len() != count
        || history.asymptote_widths.len() != count
        || !minimum.is_finite()
        || !maximum.is_finite()
        || minimum >= maximum
        || !maximum_move.is_finite()
        || maximum_move <= 0.0
        || !constraint_value.is_finite()
        || !objective_value.is_finite()
        || !objective_lift.is_finite()
        || objective_lift < 0.0
        || !constraint_lift.is_finite()
        || constraint_lift < 0.0
        || current
            .iter()
            .chain(objective_gradient)
            .chain(constraint_gradient)
            .chain(&history.previous)
            .chain(&history.previous_previous)
            .chain(&history.asymptote_widths)
            .any(|value| !value.is_finite())
    {
        return Err("MMA87 inputs must be finite, non-empty, and dimensionally consistent");
    }

    let range = maximum - minimum;
    let mut widths = history.asymptote_widths.clone();
    if iteration > 2 {
        for index in 0..count {
            let current_direction = current[index] - history.previous[index];
            let previous_direction = history.previous[index] - history.previous_previous[index];
            let factor = if current_direction == 0.0 || previous_direction == 0.0 {
                1.0
            } else if current_direction.is_sign_positive() == previous_direction.is_sign_positive()
            {
                1.2
            } else {
                0.7
            };
            widths[index] = (widths[index] * factor).clamp(range / 100.0, 10.0 * range);
        }
    }

    let mut lower_asymptotes = vec![0.0; count];
    let mut upper_asymptotes = vec![0.0; count];
    let mut lower_bounds = vec![0.0; count];
    let mut upper_bounds = vec![0.0; count];
    let mut objective_p = vec![0.0; count];
    let mut objective_q = vec![0.0; count];
    let mut constraint_p = vec![0.0; count];
    let mut constraint_q = vec![0.0; count];
    let mut objective_constant = objective_value;
    let mut constraint_constant = constraint_value;
    for index in 0..count {
        let width = widths[index];
        if width <= 0.0 {
            return Err("MMA87 asymptote widths must be positive");
        }
        lower_asymptotes[index] = current[index] - width;
        upper_asymptotes[index] = current[index] + width;
        lower_bounds[index] = minimum
            .max(lower_asymptotes[index] + 0.1 * width)
            .max(current[index] - maximum_move);
        upper_bounds[index] = maximum
            .min(upper_asymptotes[index] - 0.1 * width)
            .min(current[index] + maximum_move);
        let width_squared = width * width;
        objective_p[index] =
            width_squared * objective_gradient[index].max(0.0) + objective_lift * width / 4.0;
        objective_q[index] =
            width_squared * (-objective_gradient[index]).max(0.0) + objective_lift * width / 4.0;
        constraint_p[index] =
            width_squared * constraint_gradient[index].max(0.0) + constraint_lift * width / 4.0;
        constraint_q[index] =
            width_squared * (-constraint_gradient[index]).max(0.0) + constraint_lift * width / 4.0;
        objective_constant -= (objective_p[index] + objective_q[index]) / width;
        constraint_constant -= (constraint_p[index] + constraint_q[index]) / width;
    }

    let primal = |dual: f64| {
        (0..count)
            .map(|index| {
                let p = objective_p[index] + dual * constraint_p[index];
                let q = objective_q[index] + dual * constraint_q[index];
                let derivative_at_lower = p
                    / (upper_asymptotes[index] - lower_bounds[index]).powi(2)
                    - q / (lower_bounds[index] - lower_asymptotes[index]).powi(2);
                let derivative_at_upper = p
                    / (upper_asymptotes[index] - upper_bounds[index]).powi(2)
                    - q / (upper_bounds[index] - lower_asymptotes[index]).powi(2);
                if derivative_at_lower >= 0.0 {
                    lower_bounds[index]
                } else if derivative_at_upper <= 0.0 {
                    upper_bounds[index]
                } else {
                    let sqrt_p = p.sqrt();
                    let sqrt_q = q.sqrt();
                    (sqrt_p * lower_asymptotes[index] + sqrt_q * upper_asymptotes[index])
                        / (sqrt_p + sqrt_q)
                }
            })
            .collect::<Vec<_>>()
    };
    let approximate_constraint = |design: &[f64]| {
        constraint_constant
            + (0..count)
                .map(|index| {
                    constraint_p[index] / (upper_asymptotes[index] - design[index])
                        + constraint_q[index] / (design[index] - lower_asymptotes[index])
                })
                .sum::<f64>()
    };
    let approximate_objective = |design: &[f64]| {
        objective_constant
            + (0..count)
                .map(|index| {
                    objective_p[index] / (upper_asymptotes[index] - design[index])
                        + objective_q[index] / (design[index] - lower_asymptotes[index])
                })
                .sum::<f64>()
    };

    let at_zero = primal(0.0);
    let (dual, design) = if approximate_constraint(&at_zero) <= 0.0 {
        (0.0, at_zero)
    } else {
        let mut low = 0.0;
        let mut high = history.dual.max(1.0);
        let mut high_design = primal(high);
        for _ in 0..256 {
            if approximate_constraint(&high_design) <= 0.0 {
                break;
            }
            high *= 2.0;
            if !high.is_finite() {
                return Err("MMA87 scalar dual could not bracket a feasible subproblem");
            }
            high_design = primal(high);
        }
        if approximate_constraint(&high_design) > 0.0 {
            return Err("MMA87 scalar dual could not bracket a feasible subproblem");
        }
        for _ in 0..96 {
            let middle = 0.5 * (low + high);
            let middle_design = primal(middle);
            if approximate_constraint(&middle_design) > 0.0 {
                low = middle;
            } else {
                high = middle;
            }
        }
        let dual = 0.5 * (low + high);
        (dual, primal(dual))
    };
    let residual = approximate_constraint(&design);
    let approximate_objective = approximate_objective(&design);
    if design.iter().any(|value| !value.is_finite())
        || !residual.is_finite()
        || !approximate_objective.is_finite()
    {
        return Err("MMA87 produced a non-finite subproblem solution");
    }
    Ok(Mma87Step {
        design,
        history: Mma87History {
            previous: current.to_vec(),
            previous_previous: history.previous.clone(),
            asymptote_widths: widths,
            dual,
        },
        approximate_objective,
        approximate_constraint: residual,
    })
}

pub(super) fn conservative_lift_update(current: f64, exact_gap: f64, weight: f64) -> f64 {
    if exact_gap <= 0.0 {
        current
    } else {
        (10.0 * current).min(1.1 * (current + exact_gap / weight))
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn relative_kkt_residual(
    design: &[f64],
    objective_gradient: &[f64],
    constraint_gradient: &[f64],
    dual: f64,
    constraint_value: f64,
    minimum: f64,
    maximum: f64,
) -> f64 {
    let scale = objective_gradient
        .iter()
        .map(|value| value.abs())
        .chain(
            constraint_gradient
                .iter()
                .map(|value| dual.abs() * value.abs()),
        )
        .fold(1.0, f64::max);
    let stationarity = design
        .iter()
        .zip(objective_gradient)
        .zip(constraint_gradient)
        .map(|((density, objective), constraint)| {
            let lagrangian = objective + dual * constraint;
            if *density <= minimum + 1.0e-12 {
                (-lagrangian).max(0.0)
            } else if *density >= maximum - 1.0e-12 {
                lagrangian.max(0.0)
            } else {
                lagrangian.abs()
            }
        })
        .fold(0.0, f64::max)
        / scale;
    stationarity
        .max(constraint_value.max(0.0).abs())
        .max((-dual).max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mma87_step_matches_mit_julia_equation_fixture() {
        let current = [0.25, 0.5, 0.75];
        let history = Mma87History {
            previous: vec![0.2, 0.55, 0.8],
            previous_previous: vec![0.15, 0.6, 0.65],
            asymptote_widths: vec![0.3, 0.4, 0.5],
            dual: 0.0,
        };
        let step = mma87_update(
            &current,
            10.0,
            &[-4.0, -1.0, -0.25],
            0.0,
            &[0.3, 0.5, 0.2],
            0.0,
            0.0,
            0.001,
            1.0,
            1.0,
            3,
            &history,
        )
        .expect("MMA87 fixture step");

        assert_eq!(step.history.asymptote_widths, vec![0.36, 0.48, 0.35]);
        assert!((step.history.dual - 3.773946424640127).abs() <= 1.0e-12);
        for (actual, expected) in
            step.design
                .iter()
                .zip([0.3599675527624154, 0.42443676647900624, 0.6557010188708974])
        {
            assert!((actual - expected).abs() <= 1.0e-12);
        }
        assert!(step.approximate_constraint.abs() <= 1.0e-13);
    }

    #[test]
    fn mma87_kkt_residual_detects_interior_stationarity_and_bounds() {
        let residual = relative_kkt_residual(
            &[0.5, 0.001, 1.0],
            &[-2.0, 1.0, -1.0],
            &[1.0, 0.0, 0.0],
            2.0,
            0.0,
            0.001,
            1.0,
        );
        assert_eq!(residual, 0.0);

        let violated = relative_kkt_residual(
            &[0.5, 0.001, 1.0],
            &[-1.5, -1.0, 1.0],
            &[1.0, 0.0, 0.0],
            2.0,
            0.02,
            0.001,
            1.0,
        );
        assert!(violated >= 0.5);
    }

    #[test]
    fn mma87_step_never_exceeds_the_declared_move_limit() {
        let current = [0.5, 0.5];
        let history = Mma87History {
            previous: vec![0.4, 0.6],
            previous_previous: vec![0.3, 0.7],
            asymptote_widths: vec![0.5, 0.5],
            dual: 0.0,
        };
        let step = mma87_update(
            &current,
            1.0,
            &[-100.0, 100.0],
            -0.1,
            &[0.5, 0.5],
            0.0,
            0.0,
            0.001,
            1.0,
            0.2,
            4,
            &history,
        )
        .expect("bounded MMA87 step");

        assert!(step
            .design
            .iter()
            .zip(current)
            .all(|(next, previous)| (next - previous).abs() <= 0.2 + 1.0e-12));
    }

    #[test]
    fn conservative_lift_matches_mma02_upper_bound_update() {
        assert_eq!(conservative_lift_update(1.0e-5, -2.0, 0.25), 1.0e-5);
        assert!((conservative_lift_update(1.0e-5, 0.5, 0.25) - 1.0e-4).abs() <= 1.0e-15);
        assert!((conservative_lift_update(0.1, 0.01, 0.25) - 0.154).abs() <= 1.0e-15);
    }
}
