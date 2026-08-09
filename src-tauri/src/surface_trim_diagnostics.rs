use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const POSITION_EPSILON: f64 = 1.0e-9;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimMeshDiagnostics {
    pub retained_area: f64,
    pub output_vertex_count: u64,
    pub output_triangle_count: u64,
    pub duplicate_position_count: u64,
    pub boundary_edge_count: u64,
    pub non_manifold_edge_count: u64,
    pub orientation_mismatch_count: u64,
    pub invalid_cut_vertex_degree_count: u64,
    pub closed_boundary_loops: Vec<Vec<u32>>,
    pub open_boundary_chains: Vec<Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceTrimDiagnosticsError {
    NonFiniteVertex {
        vertex_index: u64,
    },
    TriangleVertexOutOfBounds {
        triangle_index: u64,
        vertex_index: u64,
    },
    DegenerateTriangle {
        triangle_index: u64,
    },
    CutVertexOutOfBounds {
        edge_index: u64,
        vertex_index: u64,
    },
    DegenerateCutEdge {
        edge_index: u64,
    },
    DuplicateCutEdge {
        edge: [u32; 2],
    },
}

impl fmt::Display for SurfaceTrimDiagnosticsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SurfaceTrimDiagnosticsError {}

pub fn analyze_surface_trim_mesh(
    vertices: &[[f64; 3]],
    triangles: &[[u32; 3]],
    cut_edges: &[[u32; 2]],
) -> Result<SurfaceTrimMeshDiagnostics, SurfaceTrimDiagnosticsError> {
    for (index, vertex) in vertices.iter().enumerate() {
        if vertex.iter().any(|component| !component.is_finite()) {
            return Err(SurfaceTrimDiagnosticsError::NonFiniteVertex {
                vertex_index: index as u64,
            });
        }
    }

    let mut retained_area = 0.0;
    let mut mesh_edges = BTreeMap::<(u32, u32), Vec<bool>>::new();
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        let mut points = [[0.0; 3]; 3];
        for (corner, vertex_index) in triangle.iter().copied().enumerate() {
            points[corner] = *vertices.get(vertex_index as usize).ok_or(
                SurfaceTrimDiagnosticsError::TriangleVertexOutOfBounds {
                    triangle_index: triangle_index as u64,
                    vertex_index: vertex_index as u64,
                },
            )?;
        }
        let area = triangle_area(points);
        if !area.is_finite() || area <= POSITION_EPSILON {
            return Err(SurfaceTrimDiagnosticsError::DegenerateTriangle {
                triangle_index: triangle_index as u64,
            });
        }
        retained_area += area;
        for (from, to) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            let edge = ordered_edge(from, to);
            mesh_edges.entry(edge).or_default().push(from < to);
        }
    }

    let boundary_edge_count = mesh_edges
        .values()
        .filter(|incidents| incidents.len() == 1)
        .count() as u64;
    let non_manifold_edge_count = mesh_edges
        .values()
        .filter(|incidents| incidents.len() > 2)
        .count() as u64;
    let orientation_mismatch_count = mesh_edges
        .values()
        .filter(|incidents| incidents.len() == 2 && incidents[0] == incidents[1])
        .count() as u64;

    let duplicate_position_count = duplicate_position_count(vertices);
    let (closed_boundary_loops, open_boundary_chains, invalid_cut_vertex_degree_count) =
        analyze_cut_graph(vertices.len(), cut_edges)?;

    Ok(SurfaceTrimMeshDiagnostics {
        retained_area,
        output_vertex_count: vertices.len() as u64,
        output_triangle_count: triangles.len() as u64,
        duplicate_position_count,
        boundary_edge_count,
        non_manifold_edge_count,
        orientation_mismatch_count,
        invalid_cut_vertex_degree_count,
        closed_boundary_loops,
        open_boundary_chains,
    })
}

fn analyze_cut_graph(
    vertex_count: usize,
    cut_edges: &[[u32; 2]],
) -> Result<(Vec<Vec<u32>>, Vec<Vec<u32>>, u64), SurfaceTrimDiagnosticsError> {
    let mut unused = BTreeSet::new();
    let mut adjacency = BTreeMap::<u32, BTreeSet<u32>>::new();
    for (edge_index, edge) in cut_edges.iter().copied().enumerate() {
        for vertex in edge {
            if vertex as usize >= vertex_count {
                return Err(SurfaceTrimDiagnosticsError::CutVertexOutOfBounds {
                    edge_index: edge_index as u64,
                    vertex_index: vertex as u64,
                });
            }
        }
        if edge[0] == edge[1] {
            return Err(SurfaceTrimDiagnosticsError::DegenerateCutEdge {
                edge_index: edge_index as u64,
            });
        }
        let ordered = ordered_edge(edge[0], edge[1]);
        if !unused.insert(ordered) {
            return Err(SurfaceTrimDiagnosticsError::DuplicateCutEdge {
                edge: [ordered.0, ordered.1],
            });
        }
        adjacency.entry(edge[0]).or_default().insert(edge[1]);
        adjacency.entry(edge[1]).or_default().insert(edge[0]);
    }

    let invalid_degree_count = adjacency
        .values()
        .filter(|neighbors| neighbors.len() > 2)
        .count() as u64;
    let mut loops = Vec::new();
    let mut chains = Vec::new();
    while let Some(edge) = unused.iter().next().copied() {
        let start = smallest_open_endpoint(&unused, &adjacency).unwrap_or(edge.0);
        let mut path = vec![start];
        let mut current = start;
        let mut closed = false;
        loop {
            let next = adjacency
                .get(&current)
                .into_iter()
                .flatten()
                .copied()
                .find(|neighbor| unused.contains(&ordered_edge(current, *neighbor)));
            let Some(next) = next else { break };
            unused.remove(&ordered_edge(current, next));
            current = next;
            if current == start {
                closed = true;
                break;
            }
            path.push(current);
        }
        if closed {
            loops.push(canonicalize_loop(path));
        } else {
            chains.push(canonicalize_chain(path));
        }
    }
    loops.sort();
    chains.sort();
    Ok((loops, chains, invalid_degree_count))
}

fn smallest_open_endpoint(
    unused: &BTreeSet<(u32, u32)>,
    adjacency: &BTreeMap<u32, BTreeSet<u32>>,
) -> Option<u32> {
    adjacency
        .iter()
        .filter(|(vertex, neighbors)| {
            neighbors.len() == 1
                && neighbors
                    .iter()
                    .any(|neighbor| unused.contains(&ordered_edge(**vertex, *neighbor)))
        })
        .map(|(vertex, _)| *vertex)
        .next()
}

fn canonicalize_chain(mut chain: Vec<u32>) -> Vec<u32> {
    let mut reversed = chain.iter().rev().copied().collect::<Vec<_>>();
    if reversed < chain {
        std::mem::swap(&mut chain, &mut reversed);
    }
    chain
}

fn canonicalize_loop(loop_vertices: Vec<u32>) -> Vec<u32> {
    let minimum = loop_vertices.iter().copied().min().unwrap_or(0);
    let forward_start = loop_vertices
        .iter()
        .position(|vertex| *vertex == minimum)
        .unwrap_or(0);
    let forward = rotate_loop(&loop_vertices, forward_start);
    let reversed_source = loop_vertices.iter().rev().copied().collect::<Vec<_>>();
    let reversed_start = reversed_source
        .iter()
        .position(|vertex| *vertex == minimum)
        .unwrap_or(0);
    let reversed = rotate_loop(&reversed_source, reversed_start);
    forward.min(reversed)
}

fn rotate_loop(vertices: &[u32], start: usize) -> Vec<u32> {
    vertices[start..]
        .iter()
        .chain(vertices[..start].iter())
        .copied()
        .collect()
}

fn duplicate_position_count(vertices: &[[f64; 3]]) -> u64 {
    let mut duplicates = 0u64;
    for right in 0..vertices.len() {
        if vertices[..right]
            .iter()
            .any(|left| squared_distance(*left, vertices[right]) <= POSITION_EPSILON.powi(2))
        {
            duplicates += 1;
        }
    }
    duplicates
}

fn ordered_edge(left: u32, right: u32) -> (u32, u32) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn triangle_area(points: [[f64; 3]; 3]) -> f64 {
    let ab = sub(points[1], points[0]);
    let ac = sub(points[2], points[0]);
    norm(cross(ab, ac)) * 0.5
}

fn squared_distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    let delta = sub(left, right);
    delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]
}

fn sub(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn norm(value: [f64; 3]) -> f64 {
    (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_mesh_reports_area_and_consistent_orientation() {
        let vertices = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let triangles = [[0, 1, 2], [0, 2, 3]];
        let report = analyze_surface_trim_mesh(&vertices, &triangles, &[]).unwrap();
        assert!((report.retained_area - 1.0).abs() <= POSITION_EPSILON);
        assert_eq!(report.orientation_mismatch_count, 0);
        assert_eq!(report.boundary_edge_count, 4);
    }

    #[test]
    fn closed_cut_loop_is_canonical() {
        let vertices = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let triangles = [[0, 1, 2], [0, 2, 3]];
        let report =
            analyze_surface_trim_mesh(&vertices, &triangles, &[[2, 3], [0, 1], [3, 0], [1, 2]])
                .unwrap();
        assert_eq!(report.closed_boundary_loops, vec![vec![0, 1, 2, 3]]);
        assert!(report.open_boundary_chains.is_empty());
    }
}
