use serde::{Deserialize, Serialize};
use specta::Type;
use std::fmt;

const CAP_EPSILON: f64 = 1.0e-10;
pub const SURFACE_TRIM_CAP_MODES: &[&str] = &["open", "flat", "surface-fill"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SurfaceTrimCapMode {
    Open,
    Flat,
    SurfaceFill,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FlatCapFit {
    pub centroid: [f64; 3],
    pub normal: [f64; 3],
    pub basis_u: [f64; 3],
    pub basis_v: [f64; 3],
    pub projected: Vec<[f64; 2]>,
    pub max_planarity_deviation: f64,
    pub rms_planarity_deviation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimCapReport {
    pub mode: SurfaceTrimCapMode,
    pub boundary_point_count: u64,
    pub added_vertex_count: u64,
    pub added_triangle_count: u64,
    pub max_planarity_deviation: Option<f64>,
    pub rms_planarity_deviation: Option<f64>,
    pub explicitly_open: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimCapOutput {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub report: SurfaceTrimCapReport,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceTrimCapError {
    TooFewBoundaryPoints,
    NonFinitePoint {
        point_index: u64,
    },
    InvalidTolerance,
    BoundaryVertexOutOfBounds {
        loop_index: u64,
        vertex_index: u64,
    },
    DuplicateBoundaryVertex {
        vertex_index: u64,
    },
    BoundaryEdgeMissing {
        from: u64,
        to: u64,
    },
    BoundaryEdgeNonManifold {
        from: u64,
        to: u64,
        incident_count: u64,
    },
    BoundaryOrientationInconsistent {
        from: u64,
        to: u64,
    },
    DegenerateBoundary,
    SelfIntersectingProjection,
    PlanarityToleranceExceeded {
        max_deviation: f64,
        rms_deviation: f64,
        tolerance: f64,
    },
    SurfaceFillFoldover {
        triangle_index: u64,
    },
    IndexOverflow,
}

impl fmt::Display for SurfaceTrimCapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlanarityToleranceExceeded {
                max_deviation,
                rms_deviation,
                tolerance,
            } => write!(
                formatter,
                "PlanarityToleranceExceeded maxDeviation={max_deviation:.6} rmsDeviation={rms_deviation:.6} tolerance={tolerance:.6}"
            ),
            Self::SurfaceFillFoldover { triangle_index } => {
                write!(formatter, "SurfaceFillFoldover triangleIndex={triangle_index}")
            }
            other => write!(formatter, "{other:?}"),
        }
    }
}

impl std::error::Error for SurfaceTrimCapError {}

pub fn fit_flat_cap(
    boundary: &[[f64; 3]],
    tolerance: f64,
) -> Result<FlatCapFit, SurfaceTrimCapError> {
    if boundary.len() < 3 {
        return Err(SurfaceTrimCapError::TooFewBoundaryPoints);
    }
    if !tolerance.is_finite() || tolerance < 0.0 {
        return Err(SurfaceTrimCapError::InvalidTolerance);
    }
    for (index, point) in boundary.iter().enumerate() {
        if point.iter().any(|component| !component.is_finite()) {
            return Err(SurfaceTrimCapError::NonFinitePoint {
                point_index: index as u64,
            });
        }
    }

    let centroid = mean(boundary);
    let covariance = covariance(boundary, centroid);
    let (eigenvalues, eigenvectors) = jacobi_eigen(covariance);
    let mut order = [0usize, 1, 2];
    order.sort_by(|left, right| eigenvalues[*left].total_cmp(&eigenvalues[*right]));
    let largest = eigenvalues[order[2]].abs().max(CAP_EPSILON);
    if eigenvalues[order[1]].abs() <= largest * CAP_EPSILON {
        return Err(SurfaceTrimCapError::DegenerateBoundary);
    }

    let mut normal = normalize([
        eigenvectors[0][order[0]],
        eigenvectors[1][order[0]],
        eigenvectors[2][order[0]],
    ])?;
    let sign_index = (0..3)
        .max_by(|left, right| normal[*left].abs().total_cmp(&normal[*right].abs()))
        .unwrap_or(0);
    if normal[sign_index] < 0.0 {
        normal = scale(normal, -1.0);
    }

    let reference_axis = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        .into_iter()
        .min_by(|left, right| {
            dot(*left, normal)
                .abs()
                .total_cmp(&dot(*right, normal).abs())
        })
        .unwrap_or([1.0, 0.0, 0.0]);
    let basis_u = normalize(cross(reference_axis, normal))?;
    let basis_v = normalize(cross(normal, basis_u))?;

    let mut projected = Vec::with_capacity(boundary.len());
    let mut max_deviation = 0.0f64;
    let mut squared_deviation_sum = 0.0;
    for point in boundary {
        let centered = sub(*point, centroid);
        let distance = dot(centered, normal);
        max_deviation = max_deviation.max(distance.abs());
        squared_deviation_sum += distance * distance;
        projected.push([dot(centered, basis_u), dot(centered, basis_v)]);
    }
    let rms_deviation = (squared_deviation_sum / boundary.len() as f64).sqrt();
    if max_deviation > tolerance {
        return Err(SurfaceTrimCapError::PlanarityToleranceExceeded {
            max_deviation,
            rms_deviation,
            tolerance,
        });
    }

    Ok(FlatCapFit {
        centroid,
        normal,
        basis_u,
        basis_v,
        projected,
        max_planarity_deviation: max_deviation,
        rms_planarity_deviation: rms_deviation,
    })
}

pub fn apply_surface_trim_cap(
    vertices: &[[f64; 3]],
    triangles: &[[u32; 3]],
    boundary_loop: &[u32],
    mode: SurfaceTrimCapMode,
    flat_tolerance: f64,
) -> Result<SurfaceTrimCapOutput, SurfaceTrimCapError> {
    let boundary = resolve_boundary(vertices, boundary_loop)?;
    let mut output_vertices = vertices.to_vec();
    let mut output_triangles = triangles.to_vec();

    let (added_vertices, added_triangles, max_deviation, rms_deviation, explicitly_open) =
        match mode {
            SurfaceTrimCapMode::Open => (0, 0, None, None, true),
            SurfaceTrimCapMode::Flat => {
                let fit = fit_flat_cap(&boundary, flat_tolerance)?;
                if polygon_self_intersects(&fit.projected) {
                    return Err(SurfaceTrimCapError::SelfIntersectingProjection);
                }
                for (boundary_index, point) in boundary_loop.iter().zip(boundary.iter()) {
                    let centered = sub(*point, fit.centroid);
                    let distance = dot(centered, fit.normal);
                    let projected3d = sub(*point, scale(fit.normal, distance));
                    output_vertices[*boundary_index as usize] = projected3d;
                }
                let cap_boundary = oriented_cap_boundary(triangles, boundary_loop)?;
                let projected = cap_boundary
                    .iter()
                    .map(|index| {
                        let centered = sub(output_vertices[*index as usize], fit.centroid);
                        [dot(centered, fit.basis_u), dot(centered, fit.basis_v)]
                    })
                    .collect::<Vec<_>>();
                let local_triangles = triangulate_polygon(&projected)?;
                for triangle in &local_triangles {
                    output_triangles.push([
                        cap_boundary[triangle[0]],
                        cap_boundary[triangle[1]],
                        cap_boundary[triangle[2]],
                    ]);
                }
                (
                    0,
                    local_triangles.len() as u64,
                    Some(fit.max_planarity_deviation),
                    Some(fit.rms_planarity_deviation),
                    false,
                )
            }
            SurfaceTrimCapMode::SurfaceFill => {
                let fit = fit_flat_cap(&boundary, f64::MAX)?;
                if polygon_self_intersects(&fit.projected) {
                    return Err(SurfaceTrimCapError::SelfIntersectingProjection);
                }
                let cap_boundary = oriented_cap_boundary(triangles, boundary_loop)?;
                let cap_points = cap_boundary
                    .iter()
                    .map(|index| vertices[*index as usize])
                    .collect::<Vec<_>>();
                let projected = cap_points
                    .iter()
                    .map(|point| {
                        let centered = sub(*point, fit.centroid);
                        [dot(centered, fit.basis_u), dot(centered, fit.basis_v)]
                    })
                    .collect::<Vec<_>>();
                let local_triangles = triangulate_polygon(&projected)?;
                let mut reference_normal = None::<[f64; 3]>;
                for (triangle_index, triangle) in local_triangles.iter().enumerate() {
                    let indices = [
                        cap_boundary[triangle[0]],
                        cap_boundary[triangle[1]],
                        cap_boundary[triangle[2]],
                    ];
                    let normal = normalize(cross(
                        sub(vertices[indices[1] as usize], vertices[indices[0] as usize]),
                        sub(vertices[indices[2] as usize], vertices[indices[0] as usize]),
                    ))
                    .map_err(|_| {
                        SurfaceTrimCapError::SurfaceFillFoldover {
                            triangle_index: triangle_index as u64,
                        }
                    })?;
                    if let Some(reference) = reference_normal {
                        if dot(reference, normal) <= CAP_EPSILON {
                            return Err(SurfaceTrimCapError::SurfaceFillFoldover {
                                triangle_index: triangle_index as u64,
                            });
                        }
                    } else {
                        reference_normal = Some(normal);
                    }
                    output_triangles.push(indices);
                }
                (0, local_triangles.len() as u64, None, None, false)
            }
        };

    Ok(SurfaceTrimCapOutput {
        vertices: output_vertices,
        triangles: output_triangles,
        report: SurfaceTrimCapReport {
            mode,
            boundary_point_count: boundary_loop.len() as u64,
            added_vertex_count: added_vertices,
            added_triangle_count: added_triangles,
            max_planarity_deviation: max_deviation,
            rms_planarity_deviation: rms_deviation,
            explicitly_open,
        },
    })
}

fn resolve_boundary(
    vertices: &[[f64; 3]],
    boundary_loop: &[u32],
) -> Result<Vec<[f64; 3]>, SurfaceTrimCapError> {
    if boundary_loop.len() < 3 {
        return Err(SurfaceTrimCapError::TooFewBoundaryPoints);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut boundary = Vec::with_capacity(boundary_loop.len());
    for (loop_index, vertex_index) in boundary_loop.iter().copied().enumerate() {
        let point = vertices.get(vertex_index as usize).copied().ok_or(
            SurfaceTrimCapError::BoundaryVertexOutOfBounds {
                loop_index: loop_index as u64,
                vertex_index: vertex_index as u64,
            },
        )?;
        if !seen.insert(vertex_index) {
            return Err(SurfaceTrimCapError::DuplicateBoundaryVertex {
                vertex_index: vertex_index as u64,
            });
        }
        if point.iter().any(|component| !component.is_finite()) {
            return Err(SurfaceTrimCapError::NonFinitePoint {
                point_index: loop_index as u64,
            });
        }
        boundary.push(point);
    }
    Ok(boundary)
}

fn oriented_cap_boundary(
    triangles: &[[u32; 3]],
    boundary_loop: &[u32],
) -> Result<Vec<u32>, SurfaceTrimCapError> {
    let mut follows_shell = None::<bool>;
    for index in 0..boundary_loop.len() {
        let from = boundary_loop[index];
        let to = boundary_loop[(index + 1) % boundary_loop.len()];
        let mut forward = 0u64;
        let mut reverse = 0u64;
        for triangle in triangles {
            for edge in [
                [triangle[0], triangle[1]],
                [triangle[1], triangle[2]],
                [triangle[2], triangle[0]],
            ] {
                if edge == [from, to] {
                    forward += 1;
                } else if edge == [to, from] {
                    reverse += 1;
                }
            }
        }
        let incident_count = forward + reverse;
        if incident_count == 0 {
            return Err(SurfaceTrimCapError::BoundaryEdgeMissing {
                from: from as u64,
                to: to as u64,
            });
        }
        if incident_count != 1 {
            return Err(SurfaceTrimCapError::BoundaryEdgeNonManifold {
                from: from as u64,
                to: to as u64,
                incident_count,
            });
        }
        let current_follows_shell = forward == 1;
        if follows_shell.is_some_and(|expected| expected != current_follows_shell) {
            return Err(SurfaceTrimCapError::BoundaryOrientationInconsistent {
                from: from as u64,
                to: to as u64,
            });
        }
        follows_shell = Some(current_follows_shell);
    }

    if follows_shell == Some(true) {
        Ok(boundary_loop.iter().rev().copied().collect())
    } else {
        Ok(boundary_loop.to_vec())
    }
}

fn jacobi_eigen(mut matrix: [[f64; 3]; 3]) -> ([f64; 3], [[f64; 3]; 3]) {
    let mut vectors = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..32 {
        let pairs = [(0usize, 1usize), (0, 2), (1, 2)];
        let (p, q) = pairs
            .into_iter()
            .max_by(|(ap, aq), (bp, bq)| matrix[*ap][*aq].abs().total_cmp(&matrix[*bp][*bq].abs()))
            .unwrap_or((0, 1));
        if matrix[p][q].abs() <= CAP_EPSILON {
            break;
        }
        let angle = 0.5 * (2.0 * matrix[p][q]).atan2(matrix[q][q] - matrix[p][p]);
        let cosine = angle.cos();
        let sine = angle.sin();
        let app = matrix[p][p];
        let aqq = matrix[q][q];
        let apq = matrix[p][q];
        matrix[p][p] = cosine * cosine * app - 2.0 * sine * cosine * apq + sine * sine * aqq;
        matrix[q][q] = sine * sine * app + 2.0 * sine * cosine * apq + cosine * cosine * aqq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;
        for index in 0..3 {
            if index != p && index != q {
                let aip = matrix[index][p];
                let aiq = matrix[index][q];
                matrix[index][p] = cosine * aip - sine * aiq;
                matrix[p][index] = matrix[index][p];
                matrix[index][q] = sine * aip + cosine * aiq;
                matrix[q][index] = matrix[index][q];
            }
            let vip = vectors[index][p];
            let viq = vectors[index][q];
            vectors[index][p] = cosine * vip - sine * viq;
            vectors[index][q] = sine * vip + cosine * viq;
        }
    }
    ([matrix[0][0], matrix[1][1], matrix[2][2]], vectors)
}

fn covariance(points: &[[f64; 3]], centroid: [f64; 3]) -> [[f64; 3]; 3] {
    let mut result = [[0.0; 3]; 3];
    for point in points {
        let delta = sub(*point, centroid);
        for row in 0..3 {
            for column in 0..3 {
                result[row][column] += delta[row] * delta[column];
            }
        }
    }
    result
}

fn triangulate_polygon(points: &[[f64; 2]]) -> Result<Vec<[usize; 3]>, SurfaceTrimCapError> {
    if points.len() < 3 {
        return Err(SurfaceTrimCapError::TooFewBoundaryPoints);
    }
    let orientation = polygon_area(points).signum();
    if orientation == 0.0 {
        return Err(SurfaceTrimCapError::DegenerateBoundary);
    }
    let mut remaining = (0..points.len()).collect::<Vec<_>>();
    let mut triangles = Vec::with_capacity(points.len() - 2);
    while remaining.len() > 3 {
        let mut found = false;
        for cursor in 0..remaining.len() {
            let previous = remaining[(cursor + remaining.len() - 1) % remaining.len()];
            let current = remaining[cursor];
            let next = remaining[(cursor + 1) % remaining.len()];
            if orientation * orient2d(points[previous], points[current], points[next])
                <= CAP_EPSILON
            {
                continue;
            }
            if remaining.iter().copied().any(|candidate| {
                candidate != previous
                    && candidate != current
                    && candidate != next
                    && point_in_triangle(
                        points[candidate],
                        points[previous],
                        points[current],
                        points[next],
                        orientation,
                    )
            }) {
                continue;
            }
            triangles.push([previous, current, next]);
            remaining.remove(cursor);
            found = true;
            break;
        }
        if !found {
            return Err(SurfaceTrimCapError::DegenerateBoundary);
        }
    }
    triangles.push([remaining[0], remaining[1], remaining[2]]);
    Ok(triangles)
}

fn polygon_self_intersects(points: &[[f64; 2]]) -> bool {
    for first in 0..points.len() {
        let first_next = (first + 1) % points.len();
        for second in (first + 1)..points.len() {
            let second_next = (second + 1) % points.len();
            if first == second || first_next == second || second_next == first {
                continue;
            }
            if segments_intersect(
                points[first],
                points[first_next],
                points[second],
                points[second_next],
            ) {
                return true;
            }
        }
    }
    false
}

fn segments_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let values = [
        orient2d(a, b, c),
        orient2d(a, b, d),
        orient2d(c, d, a),
        orient2d(c, d, b),
    ];
    values[0] * values[1] < -CAP_EPSILON && values[2] * values[3] < -CAP_EPSILON
}

fn point_in_triangle(
    point: [f64; 2],
    a: [f64; 2],
    b: [f64; 2],
    c: [f64; 2],
    orientation: f64,
) -> bool {
    orientation * orient2d(a, b, point) >= -CAP_EPSILON
        && orientation * orient2d(b, c, point) >= -CAP_EPSILON
        && orientation * orient2d(c, a, point) >= -CAP_EPSILON
}

fn orient2d(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn polygon_area(points: &[[f64; 2]]) -> f64 {
    let mut area = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        area += points[index][0] * points[next][1] - points[next][0] * points[index][1];
    }
    area * 0.5
}

fn mean(points: &[[f64; 3]]) -> [f64; 3] {
    let mut total = [0.0; 3];
    for point in points {
        total[0] += point[0];
        total[1] += point[1];
        total[2] += point[2];
    }
    scale(total, 1.0 / points.len() as f64)
}

fn normalize(value: [f64; 3]) -> Result<[f64; 3], SurfaceTrimCapError> {
    let length = dot(value, value).sqrt();
    if !length.is_finite() || length <= CAP_EPSILON {
        return Err(SurfaceTrimCapError::DegenerateBoundary);
    }
    Ok(scale(value, 1.0 / length))
}

fn sub(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn scale(value: [f64; 3], amount: f64) -> [f64; 3] {
    [value[0] * amount, value[1] * amount, value[2] * amount]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_fit_is_deterministic_for_tilted_plane() {
        let points = [
            [0.0, 0.0, 1.0],
            [2.0, 0.0, 2.0],
            [2.0, 2.0, 3.0],
            [0.0, 2.0, 2.0],
        ];
        let first = fit_flat_cap(&points, 1.0e-8).unwrap();
        let second = fit_flat_cap(&points, 1.0e-8).unwrap();
        assert_eq!(first, second);
        assert!(first.max_planarity_deviation <= 1.0e-8);
    }

    #[test]
    fn open_cap_reports_explicit_boundary_without_geometry() {
        let vertices = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let output =
            apply_surface_trim_cap(&vertices, &[], &[0, 1, 2], SurfaceTrimCapMode::Open, 0.0)
                .unwrap();
        assert!(output.report.explicitly_open);
        assert!(output.triangles.is_empty());
    }

    #[test]
    fn flat_cap_reports_excessive_planarity_deviation_without_fallback() {
        let vertices = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 2.0, 1.0],
            [0.0, 2.0, 0.0],
        ];
        let error = apply_surface_trim_cap(
            &vertices,
            &[],
            &[0, 1, 2, 3],
            SurfaceTrimCapMode::Flat,
            0.01,
        )
        .expect_err("non-planar flat cap");
        let SurfaceTrimCapError::PlanarityToleranceExceeded {
            max_deviation,
            rms_deviation,
            tolerance,
        } = error
        else {
            panic!("unexpected error: {error:?}");
        };
        assert!(max_deviation > tolerance);
        assert!(rms_deviation > 0.0);
        assert_eq!(tolerance, 0.01);
    }

    #[test]
    fn flat_cap_rejects_self_intersecting_projection() {
        let vertices = [
            [0.0, 0.0, 0.0],
            [2.0, 2.0, 0.0],
            [0.0, 2.0, 0.0],
            [2.0, 0.0, 0.0],
        ];
        let error = apply_surface_trim_cap(
            &vertices,
            &[],
            &[0, 1, 2, 3],
            SurfaceTrimCapMode::Flat,
            0.01,
        )
        .expect_err("bow-tie cap");
        assert_eq!(error, SurfaceTrimCapError::SelfIntersectingProjection);
    }

    #[test]
    fn flat_cap_projects_boundary_vertices_onto_fitted_plane_and_keeps_indices() {
        let vertices = [
            [0.0, 0.0, 0.000_000_03],
            [2.0, 0.0, -0.000_000_02],
            [2.0, 2.0, 0.000_000_01],
            [0.0, 2.0, -0.000_000_04],
            [0.25, 0.25, 0.5],
            [0.75, 0.25, 0.25],
            [0.5, 0.75, -0.25],
        ];
        let triangles = [[4, 5, 6], [0, 4, 1], [1, 4, 2], [2, 4, 3], [3, 4, 0]];
        let boundary_loop = [0, 1, 2, 3];
        let output = apply_surface_trim_cap(
            &vertices,
            &triangles,
            &boundary_loop,
            SurfaceTrimCapMode::Flat,
            1.0e-6,
        )
        .expect("flat cap");

        assert_eq!(output.triangles[0], [4, 5, 6]);
        assert!(output.triangles[triangles.len()..]
            .iter()
            .all(|triangle| triangle.iter().all(|index| boundary_loop.contains(index))));

        let projected_boundary = boundary_loop
            .iter()
            .map(|index| output.vertices[*index as usize])
            .collect::<Vec<_>>();
        let plane_normal = normalize(cross(
            sub(projected_boundary[1], projected_boundary[0]),
            sub(projected_boundary[2], projected_boundary[0]),
        ))
        .expect("projected boundary plane");
        let plane_origin = projected_boundary[0];
        for point in &projected_boundary {
            let deviation = dot(sub(*point, plane_origin), plane_normal).abs();
            assert!(deviation <= 1.0e-12, "boundary not coplanar: {deviation}");
        }

        assert!(projected_boundary
            .iter()
            .zip(vertices.iter())
            .any(|(projected, original)| projected != original));
    }

    #[test]
    fn surface_fill_preserves_nonplanar_boundary_vertices() {
        let vertices = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.2],
            [2.0, 2.0, 0.0],
            [0.0, 2.0, -0.2],
            [1.0, 1.0, -2.0],
        ];
        let triangles = [[0, 4, 1], [1, 4, 2], [2, 4, 3], [3, 4, 0]];
        let output = apply_surface_trim_cap(
            &vertices,
            &triangles,
            &[0, 1, 2, 3],
            SurfaceTrimCapMode::SurfaceFill,
            0.01,
        )
        .expect("surface fill");
        assert_eq!(output.vertices, vertices);
        assert_eq!(output.report.added_vertex_count, 0);
        assert_eq!(output.report.added_triangle_count, 2);
        assert!(!output.report.explicitly_open);
    }

    #[test]
    fn surface_fill_reports_foldover_without_flat_fallback() {
        let vertices = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 1.5],
            [2.0, 2.0, 0.0],
            [0.0, 2.0, 1.5],
            [1.0, 1.0, -2.0],
        ];
        let triangles = [[0, 4, 1], [1, 4, 2], [2, 4, 3], [3, 4, 0]];
        let error = apply_surface_trim_cap(
            &vertices,
            &triangles,
            &[0, 1, 2, 3],
            SurfaceTrimCapMode::SurfaceFill,
            0.01,
        )
        .expect_err("folded fill");

        assert!(matches!(
            error,
            SurfaceTrimCapError::SurfaceFillFoldover { .. }
        ), "unexpected error: {error:?}");
    }
}
