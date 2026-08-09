use crate::surface_trim_cut::{split_triangle_by_polyline, SplitVertex};
use std::collections::BTreeSet;
use std::fmt;

const INSERTED_VERTEX_EPSILON: f64 = 1.0e-9;

#[derive(Debug, Clone, PartialEq)]
pub struct TriangleCutInstruction {
    pub triangle_index: u64,
    pub ordered_cut_points: Vec<[f64; 3]>,
    pub keep_seed: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceTrimMeshOutput {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub cut_edges: Vec<[u32; 2]>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceTrimMeshError {
    NonFiniteVertex {
        vertex_index: u64,
    },
    NonFiniteInstructionPoint {
        triangle_index: u64,
        point_index: u64,
    },
    NonFiniteKeepSeed {
        triangle_index: u64,
    },
    TriangleIndexOutOfBounds {
        triangle_index: u64,
    },
    TriangleVertexOutOfBounds {
        triangle_index: u64,
        corner_index: usize,
        vertex_index: u64,
    },
    InstructionNotStrictlyAscending {
        previous: u64,
        current: u64,
    },
    DuplicateInstructionTriangleIndex {
        triangle_index: u64,
    },
    Overflow,
    SplitError {
        message: String,
    },
}

impl fmt::Display for SurfaceTrimMeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteVertex { vertex_index } => {
                write!(f, "non-finite vertex at index {vertex_index}")
            }
            Self::NonFiniteInstructionPoint {
                triangle_index,
                point_index,
            } => write!(
                f,
                "non-finite cut point at triangle {triangle_index} point {point_index}"
            ),
            Self::NonFiniteKeepSeed { triangle_index } => {
                write!(f, "non-finite keep seed for triangle {triangle_index}")
            }
            Self::TriangleIndexOutOfBounds { triangle_index } => {
                write!(f, "triangle index out of bounds: {triangle_index}")
            }
            Self::TriangleVertexOutOfBounds {
                triangle_index,
                corner_index,
                vertex_index,
            } => write!(
                f,
                "triangle {triangle_index} corner {corner_index} references vertex {vertex_index} out of bounds"
            ),
            Self::InstructionNotStrictlyAscending {
                previous,
                current,
            } => write!(
                f,
                "instruction triangle indices not strictly ascending: {previous} then {current}"
            ),
            Self::DuplicateInstructionTriangleIndex { triangle_index } => {
                write!(f, "duplicate instruction triangle index: {triangle_index}")
            }
            Self::Overflow => f.write_str("index overflow"),
            Self::SplitError { message } => f.write_str(message),
        }
    }
}

impl std::error::Error for SurfaceTrimMeshError {}

pub fn compose_surface_trim_mesh(
    vertices: &[[f64; 3]],
    triangles: &[[u32; 3]],
    retained_uncut: &BTreeSet<u64>,
    instructions: &[TriangleCutInstruction],
) -> Result<SurfaceTrimMeshOutput, SurfaceTrimMeshError> {
    validate_input_vertices(vertices)?;
    validate_triangles(vertices, triangles)?;
    validate_instructions(vertices.len(), triangles.len(), instructions)?;

    let input_vertex_count = vertices.len();
    let mut output_vertices = vertices.to_vec();
    let mut output_triangles = Vec::new();
    let mut output_cut_edges = Vec::new();

    let mut instruction_cursor = 0usize;
    for (triangle_index, source_triangle) in triangles.iter().enumerate() {
        let triangle_index_u64 = triangle_index as u64;
        let source_positions = [
            vertices[source_triangle[0] as usize],
            vertices[source_triangle[1] as usize],
            vertices[source_triangle[2] as usize],
        ];

        if instruction_cursor < instructions.len()
            && instructions[instruction_cursor].triangle_index == triangle_index_u64
        {
            let instruction = &instructions[instruction_cursor];
            instruction_cursor += 1;

            let split = split_triangle_by_polyline(
                source_positions,
                &instruction.ordered_cut_points,
                instruction.keep_seed,
            )
            .map_err(|error| SurfaceTrimMeshError::SplitError {
                message: format!("{error:?}"),
            })?;

            let mut local_to_global = Vec::with_capacity(split.vertices.len());
            for local_vertex in &split.vertices {
                let global_vertex = resolve_vertex(
                    local_vertex,
                    &source_positions,
                    *source_triangle,
                    &mut output_vertices,
                    input_vertex_count,
                )?;
                local_to_global.push(global_vertex);
            }

            for local_triangle in &split.triangles {
                output_triangles.push(remap_triangle(*local_triangle, &local_to_global)?);
            }

            for local_edge in split.cut_path.windows(2) {
                output_cut_edges.push(remap_edge(
                    [local_edge[0], local_edge[1]],
                    &local_to_global,
                )?);
            }
        } else if retained_uncut.contains(&triangle_index_u64) {
            output_triangles.push(*source_triangle);
        }
    }

    Ok(SurfaceTrimMeshOutput {
        vertices: output_vertices,
        triangles: output_triangles,
        cut_edges: output_cut_edges,
    })
}

fn validate_input_vertices(vertices: &[[f64; 3]]) -> Result<(), SurfaceTrimMeshError> {
    for (vertex_index, vertex) in vertices.iter().enumerate() {
        if !vertex.iter().all(|component| component.is_finite()) {
            return Err(SurfaceTrimMeshError::NonFiniteVertex {
                vertex_index: vertex_index as u64,
            });
        }
    }

    Ok(())
}

fn validate_triangles(
    vertices: &[[f64; 3]],
    triangles: &[[u32; 3]],
) -> Result<(), SurfaceTrimMeshError> {
    let vertex_count = vertices.len() as u64;
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        let triangle_index = triangle_index as u64;
        for (corner_index, vertex_index) in triangle.iter().enumerate() {
            if u64::from(*vertex_index) >= vertex_count {
                return Err(SurfaceTrimMeshError::TriangleVertexOutOfBounds {
                    triangle_index,
                    corner_index,
                    vertex_index: u64::from(*vertex_index),
                });
            }
        }
    }

    Ok(())
}

fn validate_instructions(
    vertex_count: usize,
    triangle_count: usize,
    instructions: &[TriangleCutInstruction],
) -> Result<(), SurfaceTrimMeshError> {
    let triangle_limit = triangle_count as u64;
    let mut previous_triangle_index = None;

    for instruction in instructions {
        if instruction.triangle_index >= triangle_limit {
            return Err(SurfaceTrimMeshError::TriangleIndexOutOfBounds {
                triangle_index: instruction.triangle_index,
            });
        }

        if let Some(previous) = previous_triangle_index {
            if instruction.triangle_index == previous {
                return Err(SurfaceTrimMeshError::DuplicateInstructionTriangleIndex {
                    triangle_index: instruction.triangle_index,
                });
            }

            if instruction.triangle_index < previous {
                return Err(SurfaceTrimMeshError::InstructionNotStrictlyAscending {
                    previous,
                    current: instruction.triangle_index,
                });
            }
        }

        previous_triangle_index = Some(instruction.triangle_index);

        for (point_index, point) in instruction.ordered_cut_points.iter().enumerate() {
            if !point.iter().all(|component| component.is_finite()) {
                return Err(SurfaceTrimMeshError::NonFiniteInstructionPoint {
                    triangle_index: instruction.triangle_index,
                    point_index: point_index as u64,
                });
            }
        }

        if !instruction
            .keep_seed
            .iter()
            .all(|component| component.is_finite())
        {
            return Err(SurfaceTrimMeshError::NonFiniteKeepSeed {
                triangle_index: instruction.triangle_index,
            });
        }
    }

    if vertex_count > u32::MAX as usize {
        return Err(SurfaceTrimMeshError::Overflow);
    }

    Ok(())
}

fn resolve_vertex(
    local_vertex: &SplitVertex,
    source_positions: &[[f64; 3]; 3],
    source_triangle: [u32; 3],
    output_vertices: &mut Vec<[f64; 3]>,
    input_vertex_count: usize,
) -> Result<u32, SurfaceTrimMeshError> {
    if local_vertex.position == source_positions[0] {
        return Ok(source_triangle[0]);
    }

    if local_vertex.position == source_positions[1] {
        return Ok(source_triangle[1]);
    }

    if local_vertex.position == source_positions[2] {
        return Ok(source_triangle[2]);
    }

    if let Some(existing_index) =
        find_inserted_vertex(output_vertices, input_vertex_count, &local_vertex.position)
    {
        return Ok(existing_index);
    }

    if output_vertices.len() >= u32::MAX as usize {
        return Err(SurfaceTrimMeshError::Overflow);
    }

    output_vertices.push(local_vertex.position);
    let new_index = output_vertices.len() - 1;
    u32::try_from(new_index).map_err(|_| SurfaceTrimMeshError::Overflow)
}

fn find_inserted_vertex(
    vertices: &[[f64; 3]],
    input_vertex_count: usize,
    needle: &[f64; 3],
) -> Option<u32> {
    vertices
        .get(input_vertex_count..)?
        .iter()
        .enumerate()
        .find_map(|(offset, candidate)| {
            if point_within_epsilon(candidate, needle) {
                u32::try_from(input_vertex_count + offset).ok()
            } else {
                None
            }
        })
}

fn remap_triangle(
    local_triangle: [usize; 3],
    local_to_global: &[u32],
) -> Result<[u32; 3], SurfaceTrimMeshError> {
    Ok([
        remap_local_index(local_triangle[0], local_to_global)?,
        remap_local_index(local_triangle[1], local_to_global)?,
        remap_local_index(local_triangle[2], local_to_global)?,
    ])
}

fn remap_edge(
    local_edge: [usize; 2],
    local_to_global: &[u32],
) -> Result<[u32; 2], SurfaceTrimMeshError> {
    Ok([
        remap_local_index(local_edge[0], local_to_global)?,
        remap_local_index(local_edge[1], local_to_global)?,
    ])
}

fn remap_local_index(
    local_index: usize,
    local_to_global: &[u32],
) -> Result<u32, SurfaceTrimMeshError> {
    local_to_global
        .get(local_index)
        .copied()
        .ok_or(SurfaceTrimMeshError::Overflow)
}

fn point_within_epsilon(lhs: &[f64; 3], rhs: &[f64; 3]) -> bool {
    let epsilon_sq = INSERTED_VERTEX_EPSILON * INSERTED_VERTEX_EPSILON;
    let dx = lhs[0] - rhs[0];
    let dy = lhs[1] - rhs[1];
    let dz = lhs[2] - rhs[2];
    dx * dx + dy * dy + dz * dz <= epsilon_sq
}
