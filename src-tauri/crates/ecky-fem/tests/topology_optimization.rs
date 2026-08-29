use ecky_fem::{
    advance_simp_state, advance_simp_state_traced_checkpointed, finalize_simp_state,
    initialize_simp_state, optimize_simp, topology_state_digest, FemDirichletConstraint,
    FemIndexedTet4Mesh, FemLinearSolverIdentity, FemMaterial, FemMma87State, FemPoint3,
    FemTopologyControls, FemTopologyIteration, FemTopologyLoadCase, FemTopologyState,
    FemTopologyTermination, FEM_SCHEMA_VERSION,
};

#[test]
fn product_runtime_derives_worst_case_solve_capacity() {
    assert_eq!(ecky_fem::topology_required_solve_capacity(120, 5), 19_805);
    assert_eq!(ecky_fem::topology_required_solve_capacity(0, 5), 5);
}

#[test]
fn resume_identity_ignores_runtime_resource_caps() {
    let mesh = two_path_mesh();
    let loads = [FemTopologyLoadCase {
        id: "runtime-cap".into(),
        weight: 1.0,
        rhs_n: tip_axis_load(mesh.nodes.len(), 5, 2, -10.0),
    }];
    let constraints = fixed_nodes(&[0, 2, 3]);
    let first = controls();
    let mut second = first.clone();
    second.maximum_iterations = first.maximum_iterations * 2;
    second.maximum_solve_count = first.maximum_solve_count * 2;
    second.maximum_working_memory_bytes = first.maximum_working_memory_bytes * 2;
    second.maximum_result_bytes = first.maximum_result_bytes * 2;
    second.maximum_wall_time_ms = first.maximum_wall_time_ms * 2;

    let first_state =
        initialize_simp_state(&mesh, &material(), &loads, &constraints, &first).unwrap();
    let second_state =
        initialize_simp_state(&mesh, &material(), &loads, &constraints, &second).unwrap();

    assert_eq!(first_state.input_digest, second_state.input_digest);
}

#[test]
fn unstructured_tet4_design_reduces_weighted_compliance_at_fixed_volume() {
    let mesh = two_path_mesh();
    let result = optimize_simp(
        &mesh,
        &material(),
        &[FemTopologyLoadCase {
            id: "tip-z".into(),
            weight: 1.0,
            rhs_n: tip_load(mesh.nodes.len(), 5, -10.0),
        }],
        &fixed_nodes(&[0, 2, 3]),
        &controls(),
    )
    .expect("topology optimization");

    assert!(result.iterations.len() >= 2);
    assert!(result
        .iterations
        .iter()
        .all(|iteration| iteration.maximum_density_change <= controls().move_limit + 1.0e-12));
    assert!(result.final_compliance < result.initial_compliance);
    assert!((result.final_volume_fraction - 0.55).abs() < 2.0e-3);
    assert!(result.densities.iter().all(|rho| rho.is_finite()));
    assert!(result
        .iterations
        .iter()
        .all(|iteration| iteration.kkt_residual.is_finite()));
    assert!(result
        .iterations
        .iter()
        .all(|iteration| iteration.maximum_physical_density_change.is_finite()));
    assert!(result
        .iterations
        .iter()
        .all(|iteration| (1..=10).contains(&iteration.conservative_inner_attempts)));
    if result.termination == FemTopologyTermination::Converged {
        assert!(result.iterations.iter().rev().take(3).all(|iteration| {
            iteration.maximum_physical_density_change <= controls().convergence_tolerance
                && iteration.kkt_residual <= controls().convergence_tolerance
        }));
    }
    assert!(!result.exact_brep && !result.production_step && !result.engineering_accepted);
    assert!(result.result_digest.starts_with("sha256:"));
}

#[test]
fn gcmma_conservative_inner_work_is_invariant_to_load_scale() {
    fn first_iteration_attempts(force_n: f64) -> usize {
        let mesh = two_path_mesh();
        let configured = FemTopologyControls {
            volume_fraction: 0.2,
            maximum_iterations: 1,
            ..controls()
        };
        let loads = [FemTopologyLoadCase {
            id: "scaled-tip-z".into(),
            weight: 1.0,
            rhs_n: tip_load(mesh.nodes.len(), 5, force_n),
        }];
        let constraints = fixed_nodes(&[0, 2, 3]);
        let mut state =
            initialize_simp_state(&mesh, &material(), &loads, &constraints, &configured).unwrap();
        advance_simp_state(
            &mesh,
            &material(),
            &loads,
            &constraints,
            &configured,
            &mut state,
            1,
            || false,
        )
        .unwrap();
        state.iterations[0].conservative_inner_attempts
    }

    let reference = first_iteration_attempts(-10.0);
    let scaled = first_iteration_attempts(-1_000.0);
    assert!(
        scaled <= reference + 1,
        "load scaling inflated GCMMA inner work: reference={reference}, scaled={scaled}"
    );
}

#[test]
fn passive_cells_remain_invariant_and_report_volume_contribution() {
    let mesh = two_path_mesh();
    let configured = FemTopologyControls {
        volume_fraction: 0.9,
        passive_solid_cells: vec![0],
        passive_void_cells: vec![3],
        ..controls()
    };
    let result = optimize_simp(
        &mesh,
        &material(),
        &[FemTopologyLoadCase {
            id: "tip-z".into(),
            weight: 1.0,
            rhs_n: tip_load(mesh.nodes.len(), 5, -10.0),
        }],
        &fixed_nodes(&[0, 2, 3]),
        &configured,
    )
    .unwrap();
    assert_eq!(result.densities[0], 1.0);
    assert_eq!(result.densities[3], configured.minimum_density);
    assert!(result.passive_solid_volume_fraction > 0.0);
    assert!(result.passive_void_volume_fraction > 0.0);
}

#[test]
fn conflicting_passive_regions_fail_before_solve() {
    let mesh = two_path_mesh();
    let error = optimize_simp(
        &mesh,
        &material(),
        &[FemTopologyLoadCase {
            id: "tip-z".into(),
            weight: 1.0,
            rhs_n: tip_load(mesh.nodes.len(), 5, -10.0),
        }],
        &fixed_nodes(&[0, 2, 3]),
        &FemTopologyControls {
            maximum_iterations: 2,
            passive_solid_cells: vec![1],
            passive_void_cells: vec![1],
            ..controls()
        },
    )
    .expect_err("conflicting passive cells");
    assert_eq!(error.field, "passiveCells");
}

#[test]
fn load_order_and_pause_resume_produce_identical_trace_and_digest() {
    let mesh = two_path_mesh();
    let loads = vec![
        FemTopologyLoadCase {
            id: "tip-z".into(),
            weight: 1.0,
            rhs_n: tip_load(mesh.nodes.len(), 5, -10.0),
        },
        FemTopologyLoadCase {
            id: "tip-y".into(),
            weight: 0.35,
            rhs_n: tip_axis_load(mesh.nodes.len(), 5, 1, 4.0),
        },
    ];
    let constraints = fixed_nodes(&[0, 2, 3]);
    let direct = optimize_simp(&mesh, &material(), &loads, &constraints, &controls())
        .expect("direct optimization");
    let reversed = optimize_simp(
        &mesh,
        &material(),
        &loads.iter().cloned().rev().collect::<Vec<_>>(),
        &constraints,
        &controls(),
    )
    .expect("reordered optimization");
    assert_eq!(direct, reversed);

    let mut state =
        initialize_simp_state(&mesh, &material(), &loads, &constraints, &controls()).unwrap();
    let paused = advance_simp_state(
        &mesh,
        &material(),
        &loads,
        &constraints,
        &controls(),
        &mut state,
        3,
        || false,
    )
    .unwrap();
    assert_eq!(paused, FemTopologyTermination::Paused);
    let termination = advance_simp_state(
        &mesh,
        &material(),
        &loads,
        &constraints,
        &controls(),
        &mut state,
        usize::MAX,
        || false,
    )
    .unwrap();
    let resumed = finalize_simp_state(
        &mesh,
        &material(),
        &loads,
        &constraints,
        &controls(),
        &state,
        termination,
    )
    .unwrap();
    assert_eq!(direct, resumed);
}

#[test]
fn completed_outer_iterations_emit_digest_valid_restart_checkpoints() {
    let mesh = two_path_mesh();
    let loads = [FemTopologyLoadCase {
        id: "checkpoint-z".into(),
        weight: 1.0,
        rhs_n: tip_load(mesh.nodes.len(), 5, -10.0),
    }];
    let constraints = fixed_nodes(&[0, 2, 3]);
    let configured = FemTopologyControls {
        maximum_iterations: 2,
        ..controls()
    };
    let mut state =
        initialize_simp_state(&mesh, &material(), &loads, &constraints, &configured).unwrap();
    let mut checkpoints = Vec::new();

    let termination = advance_simp_state_traced_checkpointed(
        &mesh,
        &material(),
        &loads,
        &constraints,
        &configured,
        &mut state,
        2,
        || false,
        |_| {},
        |checkpoint| checkpoints.push(checkpoint.clone()),
    )
    .unwrap();

    assert_eq!(termination, FemTopologyTermination::MaximumIterations);
    assert_eq!(checkpoints.len(), 2);
    for (index, checkpoint) in checkpoints.iter().enumerate() {
        assert_eq!(checkpoint.iterations.len(), index + 1);
        assert_eq!(checkpoint.state_digest, topology_state_digest(checkpoint));
    }
}

#[test]
fn cancellation_and_infeasible_passive_volume_stop_before_first_solve() {
    let mesh = two_path_mesh();
    let loads = [FemTopologyLoadCase {
        id: "tip-z".into(),
        weight: 1.0,
        rhs_n: tip_load(mesh.nodes.len(), 5, -10.0),
    }];
    let constraints = fixed_nodes(&[0, 2, 3]);
    let mut state =
        initialize_simp_state(&mesh, &material(), &loads, &constraints, &controls()).unwrap();
    let termination = advance_simp_state(
        &mesh,
        &material(),
        &loads,
        &constraints,
        &controls(),
        &mut state,
        1,
        || true,
    )
    .unwrap();
    assert_eq!(termination, FemTopologyTermination::Cancelled);
    assert!(state.iterations.is_empty());

    let infeasible = FemTopologyControls {
        volume_fraction: 0.05,
        passive_solid_cells: vec![0],
        ..controls()
    };
    let mut state =
        initialize_simp_state(&mesh, &material(), &loads, &constraints, &infeasible).unwrap();
    let error = advance_simp_state(
        &mesh,
        &material(),
        &loads,
        &constraints,
        &infeasible,
        &mut state,
        1,
        || false,
    )
    .expect_err("passive volume must make target infeasible");
    assert_eq!(error.field, "volumeFraction");
    assert!(state.iterations.is_empty());
}

#[test]
fn cancellation_stops_between_weighted_load_case_solves() {
    let mesh = two_path_mesh();
    let loads = [
        FemTopologyLoadCase {
            id: "tip-y".into(),
            weight: 0.5,
            rhs_n: tip_axis_load(mesh.nodes.len(), 5, 1, 4.0),
        },
        FemTopologyLoadCase {
            id: "tip-z".into(),
            weight: 1.0,
            rhs_n: tip_load(mesh.nodes.len(), 5, -10.0),
        },
    ];
    let constraints = fixed_nodes(&[0, 2, 3]);
    let mut state =
        initialize_simp_state(&mesh, &material(), &loads, &constraints, &controls()).unwrap();
    let mut cancellation_checks = 0;
    let termination = advance_simp_state(
        &mesh,
        &material(),
        &loads,
        &constraints,
        &controls(),
        &mut state,
        1,
        || {
            cancellation_checks += 1;
            cancellation_checks >= 3
        },
    )
    .unwrap();
    assert_eq!(termination, FemTopologyTermination::Cancelled);
    assert!(state.iterations.is_empty());
}

#[test]
fn wall_limit_stops_filter_before_first_solve() {
    let mesh = two_path_mesh();
    let loads = [FemTopologyLoadCase {
        id: "tip-z".into(),
        weight: 1.0,
        rhs_n: tip_load(mesh.nodes.len(), 5, -10.0),
    }];
    let constraints = fixed_nodes(&[0, 2, 3]);
    let configured = FemTopologyControls {
        maximum_wall_time_ms: 1,
        ..controls()
    };
    let mut state =
        initialize_simp_state(&mesh, &material(), &loads, &constraints, &configured).unwrap();
    let mut checks = 0usize;
    let termination = advance_simp_state(
        &mesh,
        &material(),
        &loads,
        &constraints,
        &configured,
        &mut state,
        1,
        || {
            checks += 1;
            if checks == 1 {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            false
        },
    )
    .unwrap();
    assert_eq!(termination, FemTopologyTermination::MaximumWallTime);
    assert!(state.iterations.is_empty());
}

#[test]
fn wall_limit_stops_between_weighted_load_case_solves() {
    let mesh = two_path_mesh();
    let loads = [
        FemTopologyLoadCase {
            id: "tip-y".into(),
            weight: 0.5,
            rhs_n: tip_axis_load(mesh.nodes.len(), 5, 1, 4.0),
        },
        FemTopologyLoadCase {
            id: "tip-z".into(),
            weight: 1.0,
            rhs_n: tip_load(mesh.nodes.len(), 5, -10.0),
        },
    ];
    let constraints = fixed_nodes(&[0, 2, 3]);
    let configured = FemTopologyControls {
        maximum_wall_time_ms: 1,
        ..controls()
    };
    let mut state =
        initialize_simp_state(&mesh, &material(), &loads, &constraints, &configured).unwrap();
    let mut checks = 0;
    let termination = advance_simp_state(
        &mesh,
        &material(),
        &loads,
        &constraints,
        &configured,
        &mut state,
        1,
        || {
            checks += 1;
            if checks == 3 {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            false
        },
    )
    .unwrap();
    assert_eq!(termination, FemTopologyTermination::MaximumWallTime);
    assert!(state.iterations.is_empty());
}

#[test]
fn stopped_state_cannot_be_finalized_with_an_unbounded_extra_solve() {
    let mesh = two_path_mesh();
    let loads = [FemTopologyLoadCase {
        id: "tip-z".into(),
        weight: 1.0,
        rhs_n: tip_load(mesh.nodes.len(), 5, -10.0),
    }];
    let constraints = fixed_nodes(&[0, 2, 3]);
    let configured = controls();
    let state =
        initialize_simp_state(&mesh, &material(), &loads, &constraints, &configured).unwrap();
    let error = finalize_simp_state(
        &mesh,
        &material(),
        &loads,
        &constraints,
        &configured,
        &state,
        FemTopologyTermination::Cancelled,
    )
    .expect_err("cancelled state must publish checkpoint without final solve");
    assert_eq!(error.field, "termination");
}

#[test]
fn topology_result_budget_measures_canonical_binary_payload() {
    let mesh = two_path_mesh();
    let configured = FemTopologyControls {
        maximum_iterations: 2,
        maximum_result_bytes: 512,
        ..controls()
    };
    let result = optimize_simp(
        &mesh,
        &material(),
        &[FemTopologyLoadCase {
            id: "tip-z".into(),
            weight: 1.0,
            rhs_n: tip_load(mesh.nodes.len(), 5, -10.0),
        }],
        &fixed_nodes(&[0, 2, 3]),
        &configured,
    )
    .expect("bounded canonical binary result fits");
    assert_eq!(result.iterations.len(), 2);
}

#[test]
fn topology_checkpoint_digest_excludes_observed_solver_timing() {
    let identity = FemLinearSolverIdentity {
        backend: "accelerate-sparse".into(),
        backend_version: "system".into(),
        factorization: "sparse-llt".into(),
        ordering: "accelerate-default".into(),
        scalar_type: "f64".into(),
        parallelism: "accelerate-managed".into(),
        thread_count: 0,
        factor_time_ms: Some(31.0),
        solve_time_ms: Some(7.0),
        relative_tolerance: 1.0e-8,
    };
    let state = FemTopologyState {
        schema_version: FEM_SCHEMA_VERSION,
        input_digest: "sha256:input".into(),
        design_densities: vec![0.4, 0.6],
        mma87: FemMma87State {
            previous_design_densities: vec![0.4, 0.6],
            previous_previous_design_densities: vec![0.4, 0.6],
            asymptote_widths: vec![0.5, 0.5],
            dual: 1.0,
            objective_lift: 0.0,
            constraint_lift: 0.0,
        },
        initial_compliance: Some(12.0),
        solver_identity: Some(identity),
        iterations: vec![],
        state_digest: String::new(),
    };
    let mut slower_observation = state.clone();
    slower_observation
        .solver_identity
        .as_mut()
        .unwrap()
        .factor_time_ms = Some(44.0);
    slower_observation
        .solver_identity
        .as_mut()
        .unwrap()
        .solve_time_ms = Some(9.0);

    assert_eq!(
        topology_state_digest(&state),
        topology_state_digest(&slower_observation)
    );
}

#[test]
fn resume_rejects_digest_valid_but_semantically_invalid_iteration_trace() {
    let mesh = two_path_mesh();
    let loads = [FemTopologyLoadCase {
        id: "tip-z".into(),
        weight: 1.0,
        rhs_n: tip_load(mesh.nodes.len(), 5, -10.0),
    }];
    let constraints = fixed_nodes(&[0, 2, 3]);
    let configured = controls();
    let mut state =
        initialize_simp_state(&mesh, &material(), &loads, &constraints, &configured).unwrap();
    state.initial_compliance = Some(10.0);
    state.iterations.push(FemTopologyIteration {
        iteration: 2,
        compliance: 10.0,
        volume_fraction: configured.volume_fraction,
        maximum_density_change: 0.1,
        maximum_physical_density_change: 0.1,
        kkt_residual: 0.1,
        conservative_inner_attempts: 1,
    });
    state.state_digest = topology_state_digest(&state);

    let error = advance_simp_state(
        &mesh,
        &material(),
        &loads,
        &constraints,
        &configured,
        &mut state,
        1,
        || false,
    )
    .expect_err("digest-valid malformed trace must not resume");
    assert_eq!(error.field, "topologyState.iterations");
}

fn two_path_mesh() -> FemIndexedTet4Mesh {
    FemIndexedTet4Mesh {
        schema_version: FEM_SCHEMA_VERSION,
        nodes: vec![
            FemPoint3::new(0.0, 0.0, 0.0),
            FemPoint3::new(1.0, 0.0, 0.0),
            FemPoint3::new(0.0, 1.0, 0.0),
            FemPoint3::new(0.0, 0.0, 1.0),
            FemPoint3::new(1.0, 1.0, 0.0),
            FemPoint3::new(2.0, 0.5, 0.5),
            FemPoint3::new(1.0, 0.0, 1.0),
        ],
        cells: vec![[0, 1, 2, 3], [1, 4, 2, 3], [1, 5, 4, 3], [1, 6, 5, 3]],
    }
}

fn fixed_nodes(nodes: &[usize]) -> Vec<FemDirichletConstraint> {
    nodes
        .iter()
        .flat_map(|node| {
            (0..3).map(move |axis| FemDirichletConstraint {
                dof_index: node * 3 + axis,
                value_mm: 0.0,
            })
        })
        .collect()
}

fn tip_load(node_count: usize, node: usize, z_n: f64) -> Vec<f64> {
    let mut rhs = vec![0.0; node_count * 3];
    rhs[node * 3 + 2] = z_n;
    rhs
}

fn tip_axis_load(node_count: usize, node: usize, axis: usize, force_n: f64) -> Vec<f64> {
    let mut rhs = vec![0.0; node_count * 3];
    rhs[node * 3 + axis] = force_n;
    rhs
}

fn controls() -> FemTopologyControls {
    FemTopologyControls {
        volume_fraction: 0.55,
        penalty: 3.0,
        minimum_density: 1.0e-3,
        filter_radius_mm: 1.6,
        move_limit: 0.12,
        convergence_tolerance: 1.0e-4,
        relative_solver_tolerance: 1.0e-8,
        require_parallel_solver: false,
        maximum_iterations: 30,
        maximum_dimension: 64,
        maximum_elements: 10_000,
        maximum_solve_count: 3_000,
        maximum_working_memory_bytes: 512 * 1024 * 1024,
        maximum_result_bytes: 1_000_000,
        maximum_wall_time_ms: 30_000,
        runtime_identity_digest: "sha256:test-faer-runtime".into(),
        passive_solid_cells: vec![],
        passive_void_cells: vec![],
    }
}

fn material() -> FemMaterial {
    FemMaterial {
        schema_version: FEM_SCHEMA_VERSION,
        name: "PETG-CF screening isotropic".into(),
        young_modulus_mpa: 4_000.0,
        poisson_ratio: 0.35,
        density_kg_per_mm3: 1.25e-6,
        yield_strength_mpa: 45.0,
    }
}
