//! Partition analysis for hybrid poly BRep rendering.
//!
//! Walks a Core IR part tree and classifies how it should be rendered:
//!
//! - [`PureOcct`][PartRenderStrategy::PureOcct]: no mesh-only ops → render
//!   entirely through OCCT exact BRep.
//! - [`PureMesh`][PartRenderStrategy::PureMesh]: mesh-only ops present but no
//!   BRep-required op consumes their displaced output → render entirely through
//!   the Rust mesh renderer.
//! - [`Hybrid`][PartRenderStrategy::Hybrid]: mesh-only ops present AND a
//!   BRep-required op (boolean, chamfer, fillet, …) consumes the displaced
//!   output → render through the hybrid poly BRep pipeline (exact BRep →
//!   tessellate → mesh displacement → poly BRep → OCCT hybrid boolean).

use crate::ecky_core_ir::{
    CoreNode, CoreNodeKind, CoreOperation, CoreProgram, CoreSurfaceOp, NodeId,
};

// ---------------------------------------------------------------------------
// Public data model
// ---------------------------------------------------------------------------

/// How a single part should be rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartRenderStrategy {
    /// No mesh-only ops in this part.
    PureOcct,
    /// Mesh-only ops exist, but every downstream consumer is mesh-safe
    /// (translate, rotate, scale, mirror, group, …).
    PureMesh,
    /// A BRep-required op consumes the output of a mesh-only op.
    Hybrid,
}

/// Result of partitioning one part's tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartPartition {
    pub strategy: PartRenderStrategy,
    /// Node IDs of every mesh-only op (the boundaries).
    pub boundary_node_ids: Vec<NodeId>,
    /// True when at least one BRep-required op sits above a boundary.
    pub has_post_boundary_brep_op: bool,
}

impl PartPartition {
    pub fn is_hybrid(&self) -> bool {
        self.strategy == PartRenderStrategy::Hybrid
    }
}

// ---------------------------------------------------------------------------
// Top-level entry points
// ---------------------------------------------------------------------------

/// Classify every part in a program. Returns one [`PartPartition`] per part,
/// in the same order as `program.parts`.
pub fn analyze_program(program: &CoreProgram) -> Vec<PartPartition> {
    program.parts.iter().map(|p| analyze_part(&p.root)).collect()
}

/// Classify a single part by walking its root node tree.
pub fn analyze_part(root: &CoreNode) -> PartPartition {
    let analysis = analyze_node(root);
    let strategy = if analysis.boundary_node_ids.is_empty() {
        PartRenderStrategy::PureOcct
    } else if analysis.has_post_boundary_brep_op {
        PartRenderStrategy::Hybrid
    } else {
        PartRenderStrategy::PureMesh
    };
    PartPartition {
        strategy,
        boundary_node_ids: analysis.boundary_node_ids,
        has_post_boundary_brep_op: analysis.has_post_boundary_brep_op,
    }
}

// ---------------------------------------------------------------------------
// Internal: per-node recursive analysis (bottom-up)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct NodeAnalysis {
    /// Node IDs of mesh-only ops found in this subtree.
    boundary_node_ids: Vec<NodeId>,
    /// True if this subtree contains a BRep-required op whose result is
    /// influenced by a mesh-only op (directly or transitively).
    has_post_boundary_brep_op: bool,
    /// True if this node's RESULT is mesh-displaced (it IS a mesh-only op, or
    /// it consumes a mesh-displaced result).
    post_boundary: bool,
    /// True if this subtree contains at least one mesh-only op.
    has_mesh_op: bool,
}

fn analyze_node(node: &CoreNode) -> NodeAnalysis {
    match &node.kind {
        // Leaf-like nodes: no ops, no children.
        CoreNodeKind::Literal(_)
        | CoreNodeKind::Reference(_)
        | CoreNodeKind::Range { .. } => NodeAnalysis::default(),

        CoreNodeKind::Build { bindings, result } => {
            let mut combined = collect_children(
                bindings
                    .iter()
                    .map(|b| &b.value)
                    .chain(std::iter::once(result.as_ref())),
            );
            apply_op_post_boundary(node, None, &mut combined);
            combined
        }

        CoreNodeKind::Let { bindings, body } => {
            let mut combined = collect_children(
                bindings
                    .iter()
                    .map(|b| &b.value)
                    .chain(std::iter::once(body.as_ref())),
            );
            apply_op_post_boundary(node, None, &mut combined);
            combined
        }

        CoreNodeKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let mut combined =
                collect_children([condition.as_ref(), then_branch.as_ref(), else_branch.as_ref()]);
            apply_op_post_boundary(node, None, &mut combined);
            combined
        }

        CoreNodeKind::Call { op, args, keywords } => {
            let kw_nodes = keywords.iter().map(|kw| kw.source_node());
            let mut combined = collect_children(args.iter().chain(kw_nodes));
            apply_op_post_boundary(node, Some(op), &mut combined);
            combined
        }

        CoreNodeKind::Map { sources, body, .. } => {
            let mut combined =
                collect_children(sources.iter().chain(std::iter::once(body.as_ref())));
            apply_op_post_boundary(node, None, &mut combined);
            combined
        }

        CoreNodeKind::Apply { op, args, list } => {
            let mut combined = collect_children(args.iter().chain(std::iter::once(list.as_ref())));
            apply_op_post_boundary(node, Some(op), &mut combined);
            combined
        }

        CoreNodeKind::List(items) | CoreNodeKind::Group(items) => {
            let mut combined = collect_children(items.iter());
            apply_op_post_boundary(node, None, &mut combined);
            combined
        }
    }
}

/// Analyse a slice of child nodes and merge their results.
fn collect_children<'a>(children: impl IntoIterator<Item = &'a CoreNode>) -> NodeAnalysis {
    let mut combined = NodeAnalysis::default();
    for child in children {
        let ca = analyze_node(child);
        combined.boundary_node_ids.extend(ca.boundary_node_ids);
        combined.has_post_boundary_brep_op |= ca.has_post_boundary_brep_op;
        // A parent is post-boundary if ANY child is post-boundary (the parent
        // consumes a mesh-displaced result).
        combined.post_boundary |= ca.post_boundary;
        combined.has_mesh_op |= ca.has_mesh_op;
    }
    combined
}

/// Apply the op-level classification after children have been analysed.
///
/// Sets `post_boundary` / `has_mesh_op` when this node IS a mesh-only op,
/// and sets `has_post_boundary_brep_op` when this node is a BRep-required op
/// whose input is already post-boundary.
fn apply_op_post_boundary(
    node: &CoreNode,
    op: Option<&CoreOperation>,
    combined: &mut NodeAnalysis,
) {
    let Some(op) = op else {
        return;
    };

    let is_mesh_only = operation_is_mesh_only(op);
    let is_brep_required = operation_requires_brep(op);

    if is_mesh_only {
        combined.boundary_node_ids.push(node.id);
        combined.has_mesh_op = true;
        combined.post_boundary = true;
    }

    // The critical check: this node is a BRep-required op AND its result
    // depends on a mesh-displaced input → it needs hybrid rendering.
    if is_brep_required && combined.post_boundary {
        combined.has_post_boundary_brep_op = true;
    }
}

// ---------------------------------------------------------------------------
// Op classification helpers
// ---------------------------------------------------------------------------

/// True for ops that can ONLY be evaluated by the mesh renderer (wall-pattern,
/// future import-mesh, relief-from-image, …).
fn operation_is_mesh_only(op: &CoreOperation) -> bool {
    match op {
        CoreOperation::Custom(name) => crate::ecky_ir::is_ecky_rust_only_cad_head(name),
        _ => false,
    }
}

/// True for ops that require exact BRep topology to produce reliable results
/// and therefore trigger hybrid rendering when they consume mesh-displaced
/// input.
///
/// - Booleans (difference, union, intersection, xor): CSG over displaced
///   meshes is unreliable in the mesh renderer.
/// - Chamfer / fillet: edge operations that need real topology.
/// - Shell / offset: require face/surface data.
fn operation_requires_brep(op: &CoreOperation) -> bool {
    match op {
        CoreOperation::Boolean(_) => true,
        CoreOperation::Surface(
            CoreSurfaceOp::Chamfer
            | CoreSurfaceOp::Fillet
            | CoreSurfaceOp::Shell
            | CoreSurfaceOp::Offset
            | CoreSurfaceOp::OffsetRounded,
        ) => true,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecky_scheme::try_compile_to_core_program;

    fn partition_of(source: &str) -> PartRenderStrategy {
        let program = try_compile_to_core_program(source)
            .expect("compiled path")
            .expect("program");
        let partitions = analyze_program(&program);
        assert_eq!(partitions.len(), 1, "expected exactly one part");
        partitions[0].strategy
    }

    // -----------------------------------------------------------------------
    // Pure OCCT (no mesh-only ops)
    // -----------------------------------------------------------------------

    #[test]
    fn pure_occt_part_has_no_boundary() {
        let strategy = partition_of(
            r#"(model
                (part body
                  (difference (extrude (circle 10) 20) (cylinder 3 30))))"#,
        );
        assert_eq!(strategy, PartRenderStrategy::PureOcct);
    }

    #[test]
    fn pure_occt_part_with_chamfer_and_fillet() {
        let strategy = partition_of(
            r#"(model
                (part body
                  (chamfer 2 (fillet 3 (extrude (rectangle 20 20) 10)))))"#,
        );
        assert_eq!(strategy, PartRenderStrategy::PureOcct);
    }

    // -----------------------------------------------------------------------
    // Pure mesh (mesh-only ops, but no BRep-required consumer)
    // -----------------------------------------------------------------------

    #[test]
    fn wall_pattern_alone_is_pure_mesh() {
        let strategy = partition_of(
            r#"(model
                (part body
                  (wall-pattern (:mode ribs :depth 0.4 :uFreq 8)
                    (extrude (circle 5) 18))))"#,
        );
        assert_eq!(strategy, PartRenderStrategy::PureMesh);
    }

    #[test]
    fn wall_pattern_then_translate_is_pure_mesh() {
        let strategy = partition_of(
            r#"(model
                (part body
                  (translate 0 0 5
                    (wall-pattern (:mode gyroid :depth 0.6)
                      (extrude (circle 10) 20)))))"#,
        );
        assert_eq!(strategy, PartRenderStrategy::PureMesh);
    }

    #[test]
    fn wall_pattern_then_scale_is_pure_mesh() {
        let strategy = partition_of(
            r#"(model
                (part body
                  (scale 1.2
                    (wall-pattern (:mode cellular :depth 0.5)
                      (extrude (rectangle 20 20) 10)))))"#,
        );
        assert_eq!(strategy, PartRenderStrategy::PureMesh);
    }

    #[test]
    fn chained_wall_patterns_are_pure_mesh() {
        let strategy = partition_of(
            r#"(model
                (part body
                  (wall-pattern (:mode ribs :depth 0.3)
                    (wall-pattern (:mode gyroid :depth 0.4)
                      (extrude (circle 5) 18)))))"#,
        );
        assert_eq!(strategy, PartRenderStrategy::PureMesh);
    }

    // -----------------------------------------------------------------------
    // Hybrid (mesh-only op consumed by BRep-required op)
    // -----------------------------------------------------------------------

    #[test]
    fn wall_pattern_then_difference_is_hybrid() {
        let strategy = partition_of(
            r#"(model
                (part body
                  (difference
                    (wall-pattern (:mode ribs :depth 0.4)
                      (extrude (circle 10) 20))
                    (cylinder 3 30))))"#,
        );
        assert_eq!(strategy, PartRenderStrategy::Hybrid);
    }

    #[test]
    fn wall_pattern_then_chamfer_is_hybrid() {
        let strategy = partition_of(
            r#"(model
                (part body
                  (chamfer 2
                    (wall-pattern (:mode gyroid :depth 0.6)
                      (extrude (circle 10) 20)))))"#,
        );
        assert_eq!(strategy, PartRenderStrategy::Hybrid);
    }

    #[test]
    fn wall_pattern_then_fillet_is_hybrid() {
        let strategy = partition_of(
            r#"(model
                (part body
                  (fillet 2
                    (wall-pattern (:mode cellular :depth 0.5)
                      (extrude (circle 10) 20)))))"#,
        );
        assert_eq!(strategy, PartRenderStrategy::Hybrid);
    }

    #[test]
    fn wall_pattern_then_union_is_hybrid() {
        let strategy = partition_of(
            r#"(model
                (part body
                  (union
                    (wall-pattern (:mode ribs :depth 0.4)
                      (extrude (circle 10) 20))
                    (box 5 5 5))))"#,
        );
        assert_eq!(strategy, PartRenderStrategy::Hybrid);
    }

    // -----------------------------------------------------------------------
    // Wall-pattern in a sibling branch (cutter side of difference)
    // -----------------------------------------------------------------------

    #[test]
    fn wall_pattern_in_cutter_branch_is_hybrid() {
        // The difference consumes wall-pattern output from arg1 (the cutter).
        // This is still hybrid because difference needs to boolean against
        // a poly face shape.
        let strategy = partition_of(
            r#"(model
                (part body
                  (difference
                    (extrude (circle 10) 20)
                    (wall-pattern (:mode ribs :depth 0.4)
                      (cylinder 3 30)))))"#,
        );
        assert_eq!(strategy, PartRenderStrategy::Hybrid);
    }

    // -----------------------------------------------------------------------
    // Multi-part programs
    // -----------------------------------------------------------------------

    #[test]
    fn multi_part_program_classifies_each_part_independently() {
        let program = try_compile_to_core_program(
            r#"(model
                (part exact-body (extrude (circle 10) 20))
                (part mesh-body
                  (wall-pattern (:mode ribs :depth 0.4)
                    (extrude (circle 5) 18)))
                (part hybrid-body
                  (difference
                    (wall-pattern (:mode gyroid :depth 0.6)
                      (extrude (circle 8) 15))
                    (cylinder 2 20))))"#,
        )
        .expect("compiled path")
        .expect("program");

        let partitions = analyze_program(&program);
        assert_eq!(partitions.len(), 3);
        assert_eq!(partitions[0].strategy, PartRenderStrategy::PureOcct);
        assert_eq!(partitions[1].strategy, PartRenderStrategy::PureMesh);
        assert_eq!(partitions[2].strategy, PartRenderStrategy::Hybrid);
    }

    // -----------------------------------------------------------------------
    // Boundary node IDs
    // -----------------------------------------------------------------------

    #[test]
    fn hybrid_partition_records_boundary_node_ids() {
        let program = try_compile_to_core_program(
            r#"(model
                (part body
                  (difference
                    (wall-pattern (:mode ribs :depth 0.4)
                      (extrude (circle 10) 20))
                    (cylinder 3 30))))"#,
        )
        .expect("compiled path")
        .expect("program");

        let partitions = analyze_program(&program);
        assert_eq!(partitions[0].boundary_node_ids.len(), 1);
        assert!(partitions[0].has_post_boundary_brep_op);
    }
}
