#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitVertex {
    pub position: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct SplitTriangleOutput {
    pub vertices: Vec<SplitVertex>,
    pub triangles: Vec<[usize; 3]>,
    pub cut_edge: [usize; 2],
    pub cut_path: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitTriangleError {
    NonFiniteInput,
    DegenerateTriangle,
    DegenerateCut,
    EndpointOffBoundary,
    SeedOutsideTriangle,
    SeedAmbiguous,
    IntersectionMismatch,
    InvalidOutputPolygon,
    PolylineTooShort,
    SelfIntersectingCut,
    TriangulationFailed,
}

#[derive(Clone, Copy)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Clone, Copy)]
struct Vec2 {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy)]
struct Basis {
    origin: Vec3,
    u: Vec3,
    v: Vec3,
    n: Vec3,
}

#[derive(Clone, Copy)]
struct ProjectedTriangle {
    world: [Vec3; 3],
    proj: [Vec2; 3],
    basis: Basis,
}

impl Vec3 {
    fn new(p: [f64; 3]) -> Self {
        Self {
            x: p[0],
            y: p[1],
            z: p[2],
        }
    }

    fn to_array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    fn scale(self, s: f64) -> Self {
        Self {
            x: self.x * s,
            y: self.y * s,
            z: self.z * s,
        }
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }
}

impl Basis {
    fn from_triangle(world: [Vec3; 3], eps: f64) -> Result<(Self, [Vec2; 3]), SplitTriangleError> {
        let e0 = world[1].sub(world[0]);
        let e1 = world[2].sub(world[0]);
        let n = e0.cross(e1);
        let n_len = n.norm();
        if !(n_len.is_finite()) || n_len <= eps {
            return Err(SplitTriangleError::DegenerateTriangle);
        }

        let u_len = e0.norm();
        if !(u_len.is_finite()) || u_len <= eps {
            return Err(SplitTriangleError::DegenerateTriangle);
        }

        let u = e0.scale(1.0 / u_len);
        let v_raw = n.scale(1.0 / n_len).cross(u);
        let v_len = v_raw.norm();
        if !(v_len.is_finite()) || v_len <= eps {
            return Err(SplitTriangleError::DegenerateTriangle);
        }
        let v = v_raw.scale(1.0 / v_len);
        let basis = Self {
            origin: world[0],
            u,
            v,
            n: n.scale(1.0 / n_len),
        };
        Ok((
            basis,
            [
                basis.project(world[0]),
                basis.project(world[1]),
                basis.project(world[2]),
            ],
        ))
    }

    fn project(self, p: Vec3) -> Vec2 {
        let d = p.sub(self.origin);
        Vec2 {
            x: d.dot(self.u),
            y: d.dot(self.v),
        }
    }

    fn signed_distance(self, p: Vec3) -> f64 {
        p.sub(self.origin).dot(self.n)
    }
}

fn finite_vec3(p: [f64; 3]) -> bool {
    p[0].is_finite() && p[1].is_finite() && p[2].is_finite()
}

fn finite_triangle(triangle: [[f64; 3]; 3]) -> bool {
    finite_vec3(triangle[0]) && finite_vec3(triangle[1]) && finite_vec3(triangle[2])
}

fn barycentric(proj: [Vec2; 3], p: Vec2) -> Result<[f64; 3], SplitTriangleError> {
    let a = proj[0];
    let b = proj[1];
    let c = proj[2];
    let denom = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
    if !denom.is_finite() || denom.abs() <= 1e-15 {
        return Err(SplitTriangleError::DegenerateTriangle);
    }
    let u = ((b.y - c.y) * (p.x - c.x) + (c.x - b.x) * (p.y - c.y)) / denom;
    let v = ((c.y - a.y) * (p.x - c.x) + (a.x - c.x) * (p.y - c.y)) / denom;
    let w = 1.0 - u - v;
    Ok([u, v, w])
}

fn boundary_index(bary: [f64; 3], eps: f64) -> Result<usize, SplitTriangleError> {
    if bary.iter().any(|value| *value < -eps) {
        return Err(SplitTriangleError::EndpointOffBoundary);
    }
    bary.iter()
        .position(|value| value.abs() <= eps)
        .ok_or(SplitTriangleError::EndpointOffBoundary)
}

fn in_triangle(bary: [f64; 3], eps: f64) -> bool {
    bary[0] >= -eps && bary[1] >= -eps && bary[2] >= -eps
}

fn same_point(a: [f64; 3], b: [f64; 3], eps: f64) -> bool {
    (a[0] - b[0]).abs() <= eps && (a[1] - b[1]).abs() <= eps && (a[2] - b[2]).abs() <= eps
}

fn push_unique(vertices: &mut Vec<SplitVertex>, position: [f64; 3], eps: f64) -> usize {
    for (idx, existing) in vertices.iter().enumerate() {
        if same_point(existing.position, position, eps) {
            return idx;
        }
    }
    vertices.push(SplitVertex { position });
    vertices.len() - 1
}

#[cfg(test)]
fn retained_area(vertices: &[SplitVertex], triangles: &[[usize; 3]]) -> f64 {
    let mut area = 0.0;
    for tri in triangles {
        let a = Vec3::new(vertices[tri[0]].position);
        let b = Vec3::new(vertices[tri[1]].position);
        let c = Vec3::new(vertices[tri[2]].position);
        area += b.sub(a).cross(c.sub(a)).norm() * 0.5;
    }
    area
}

fn boundary_parameter(bary: [f64; 3], eps: f64) -> Result<f64, SplitTriangleError> {
    boundary_index(bary, eps)?;
    if bary[0] >= 1.0 - eps {
        return Ok(0.0);
    }
    if bary[1] >= 1.0 - eps {
        return Ok(1.0);
    }
    if bary[2] >= 1.0 - eps {
        return Ok(2.0);
    }
    if bary[2].abs() <= eps {
        Ok(bary[1].clamp(0.0, 1.0))
    } else if bary[0].abs() <= eps {
        Ok(1.0 + bary[2].clamp(0.0, 1.0))
    } else if bary[1].abs() <= eps {
        Ok(2.0 + bary[0].clamp(0.0, 1.0))
    } else {
        Err(SplitTriangleError::EndpointOffBoundary)
    }
}

fn append_forward_boundary(
    polygon: &mut Vec<[f64; 3]>,
    triangle: &ProjectedTriangle,
    from_parameter: f64,
    to_parameter: f64,
    eps: f64,
) {
    let mut target = to_parameter;
    if target <= from_parameter + eps {
        target += 3.0;
    }
    let mut vertex_parameter = from_parameter.floor() + 1.0;
    while vertex_parameter < target - eps {
        let index = (vertex_parameter as usize) % 3;
        let point = triangle.world[index].to_array();
        if !polygon
            .last()
            .map(|last| same_point(*last, point, eps))
            .unwrap_or(false)
        {
            polygon.push(point);
        }
        vertex_parameter += 1.0;
    }
}

fn dedupe_polygon(polygon: &mut Vec<[f64; 3]>, eps: f64) {
    polygon.dedup_by(|left, right| same_point(*left, *right, eps));
    if polygon.len() > 1 && same_point(polygon[0], *polygon.last().unwrap(), eps) {
        polygon.pop();
    }
}

fn project_polygon(triangle: &ProjectedTriangle, polygon: &[[f64; 3]]) -> Vec<Vec2> {
    polygon
        .iter()
        .map(|point| triangle.basis.project(Vec3::new(*point)))
        .collect()
}

fn orient2d(a: Vec2, b: Vec2, c: Vec2) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn point_on_segment(point: Vec2, start: Vec2, end: Vec2, eps: f64) -> bool {
    if orient2d(start, end, point).abs() > eps {
        return false;
    }
    point.x >= start.x.min(end.x) - eps
        && point.x <= start.x.max(end.x) + eps
        && point.y >= start.y.min(end.y) - eps
        && point.y <= start.y.max(end.y) + eps
}

fn segments_intersect(a: Vec2, b: Vec2, c: Vec2, d: Vec2, eps: f64) -> bool {
    let ab_c = orient2d(a, b, c);
    let ab_d = orient2d(a, b, d);
    let cd_a = orient2d(c, d, a);
    let cd_b = orient2d(c, d, b);
    if ((ab_c > eps && ab_d < -eps) || (ab_c < -eps && ab_d > eps))
        && ((cd_a > eps && cd_b < -eps) || (cd_a < -eps && cd_b > eps))
    {
        return true;
    }
    (ab_c.abs() <= eps && point_on_segment(c, a, b, eps))
        || (ab_d.abs() <= eps && point_on_segment(d, a, b, eps))
        || (cd_a.abs() <= eps && point_on_segment(a, c, d, eps))
        || (cd_b.abs() <= eps && point_on_segment(b, c, d, eps))
}

fn polyline_self_intersects(polyline: &[Vec2], eps: f64) -> bool {
    for first in 0..polyline.len().saturating_sub(1) {
        for second in (first + 2)..polyline.len().saturating_sub(1) {
            if segments_intersect(
                polyline[first],
                polyline[first + 1],
                polyline[second],
                polyline[second + 1],
                eps,
            ) {
                return true;
            }
        }
    }
    false
}

fn point_in_polygon(point: Vec2, polygon: &[Vec2], eps: f64) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let a = polygon[previous];
        let b = polygon[current];
        if point_on_segment(point, a, b, eps) {
            return true;
        }
        let crosses = (a.y > point.y) != (b.y > point.y);
        if crosses {
            let x = (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x;
            if x > point.x {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

fn polygon_signed_area(polygon: &[Vec2]) -> f64 {
    let mut area = 0.0;
    for index in 0..polygon.len() {
        let next = (index + 1) % polygon.len();
        area += polygon[index].x * polygon[next].y - polygon[next].x * polygon[index].y;
    }
    area * 0.5
}

fn point_in_triangle(point: Vec2, a: Vec2, b: Vec2, c: Vec2, eps: f64) -> bool {
    orient2d(a, b, point) >= -eps && orient2d(b, c, point) >= -eps && orient2d(c, a, point) >= -eps
}

fn triangulate_polygon(polygon: &[Vec2], eps: f64) -> Result<Vec<[usize; 3]>, SplitTriangleError> {
    if polygon.len() < 3 || polygon_signed_area(polygon).abs() <= eps {
        return Err(SplitTriangleError::InvalidOutputPolygon);
    }
    let mut remaining = (0..polygon.len()).collect::<Vec<_>>();
    if polygon_signed_area(polygon) < 0.0 {
        remaining.reverse();
    }
    let mut triangles = Vec::with_capacity(polygon.len() - 2);
    while remaining.len() > 3 {
        let mut clipped = false;
        for cursor in 0..remaining.len() {
            let previous = remaining[(cursor + remaining.len() - 1) % remaining.len()];
            let current = remaining[cursor];
            let next = remaining[(cursor + 1) % remaining.len()];
            let turn = orient2d(polygon[previous], polygon[current], polygon[next]);
            if turn.abs() <= eps {
                remaining.remove(cursor);
                clipped = true;
                break;
            }
            if turn < eps {
                continue;
            }
            let contains_vertex = remaining.iter().copied().any(|candidate| {
                candidate != previous
                    && candidate != current
                    && candidate != next
                    && point_in_triangle(
                        polygon[candidate],
                        polygon[previous],
                        polygon[current],
                        polygon[next],
                        eps,
                    )
            });
            if contains_vertex {
                continue;
            }
            triangles.push([previous, current, next]);
            remaining.remove(cursor);
            clipped = true;
            break;
        }
        if !clipped {
            return Err(SplitTriangleError::TriangulationFailed);
        }
    }
    if remaining.len() == 3
        && orient2d(
            polygon[remaining[0]],
            polygon[remaining[1]],
            polygon[remaining[2]],
        ) > eps
    {
        triangles.push([remaining[0], remaining[1], remaining[2]]);
    }
    if triangles.is_empty() {
        return Err(SplitTriangleError::TriangulationFailed);
    }
    Ok(triangles)
}

pub fn split_triangle_by_segment(
    triangle: [[f64; 3]; 3],
    cut_a: [f64; 3],
    cut_b: [f64; 3],
    seed: [f64; 3],
) -> Result<SplitTriangleOutput, SplitTriangleError> {
    split_triangle_by_polyline(triangle, &[cut_a, cut_b], seed)
}

pub fn split_triangle_by_polyline(
    triangle: [[f64; 3]; 3],
    cut_points: &[[f64; 3]],
    seed: [f64; 3],
) -> Result<SplitTriangleOutput, SplitTriangleError> {
    let eps = 1e-9;
    if cut_points.len() < 2 {
        return Err(SplitTriangleError::PolylineTooShort);
    }
    if !finite_triangle(triangle)
        || !finite_vec3(seed)
        || cut_points.iter().any(|point| !finite_vec3(*point))
    {
        return Err(SplitTriangleError::NonFiniteInput);
    }
    if cut_points
        .windows(2)
        .any(|pair| same_point(pair[0], pair[1], eps))
    {
        return Err(SplitTriangleError::DegenerateCut);
    }

    let world = [
        Vec3::new(triangle[0]),
        Vec3::new(triangle[1]),
        Vec3::new(triangle[2]),
    ];
    let (basis, proj) = Basis::from_triangle(world, eps)?;
    let tri = ProjectedTriangle { world, proj, basis };

    if tri.basis.signed_distance(Vec3::new(seed)).abs() > eps {
        return Err(SplitTriangleError::SeedOutsideTriangle);
    }
    if cut_points
        .iter()
        .any(|point| tri.basis.signed_distance(Vec3::new(*point)).abs() > eps)
    {
        return Err(SplitTriangleError::EndpointOffBoundary);
    }

    let seed2 = tri.basis.project(Vec3::new(seed));
    let seed_bary = barycentric(tri.proj, seed2)?;
    if !in_triangle(seed_bary, eps) {
        return Err(SplitTriangleError::SeedOutsideTriangle);
    }

    let projected_cut = cut_points
        .iter()
        .map(|point| tri.basis.project(Vec3::new(*point)))
        .collect::<Vec<_>>();
    let cut_barycentrics = projected_cut
        .iter()
        .map(|point| barycentric(tri.proj, *point))
        .collect::<Result<Vec<_>, _>>()?;
    if cut_barycentrics
        .iter()
        .any(|weights| !in_triangle(*weights, eps))
    {
        return Err(SplitTriangleError::EndpointOffBoundary);
    }
    let start_parameter = boundary_parameter(cut_barycentrics[0], eps)?;
    let end_parameter = boundary_parameter(
        *cut_barycentrics
            .last()
            .expect("polyline has at least two points"),
        eps,
    )?;
    if polyline_self_intersects(&projected_cut, eps) {
        return Err(SplitTriangleError::SelfIntersectingCut);
    }
    if projected_cut
        .windows(2)
        .any(|segment| point_on_segment(seed2, segment[0], segment[1], eps))
    {
        return Err(SplitTriangleError::SeedAmbiguous);
    }

    let mut first_candidate = cut_points.to_vec();
    append_forward_boundary(
        &mut first_candidate,
        &tri,
        end_parameter,
        start_parameter,
        eps,
    );
    let mut second_candidate = cut_points.iter().rev().copied().collect::<Vec<_>>();
    append_forward_boundary(
        &mut second_candidate,
        &tri,
        start_parameter,
        end_parameter,
        eps,
    );
    dedupe_polygon(&mut first_candidate, eps);
    dedupe_polygon(&mut second_candidate, eps);

    let first_projected = project_polygon(&tri, &first_candidate);
    let second_projected = project_polygon(&tri, &second_candidate);
    let first_contains = point_in_polygon(seed2, &first_projected, eps);
    let second_contains = point_in_polygon(seed2, &second_projected, eps);
    let clipped = match (first_contains, second_contains) {
        (true, false) => first_candidate,
        (false, true) => second_candidate,
        _ => return Err(SplitTriangleError::SeedAmbiguous),
    };

    let mut vertices = Vec::with_capacity(clipped.len());
    for point in clipped {
        push_unique(&mut vertices, point, eps);
    }
    let projected_vertices = project_polygon(
        &tri,
        &vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect::<Vec<_>>(),
    );
    let triangles = triangulate_polygon(&projected_vertices, eps)?;
    let cut_path = cut_points
        .iter()
        .map(|point| {
            vertices
                .iter()
                .position(|vertex| same_point(vertex.position, *point, eps))
                .ok_or(SplitTriangleError::IntersectionMismatch)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let cut_edge = [cut_path[0], *cut_path.last().expect("non-empty cut path")];

    Ok(SplitTriangleOutput {
        vertices,
        triangles,
        cut_edge,
        cut_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area_of(output: &SplitTriangleOutput) -> f64 {
        retained_area(&output.vertices, &output.triangles)
    }

    #[test]
    fn coarse_triangle_interior_crossing() {
        let tri = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let out = split_triangle_by_segment(tri, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.1, 0.1, 0.0])
            .unwrap();
        assert!((area_of(&out) - 0.5).abs() <= 1e-9);
        assert_eq!(out.triangles.len(), 1);
        assert_eq!(
            out.cut_edge[0],
            out.vertices
                .iter()
                .position(|v| v.position == [1.0, 0.0, 0.0])
                .unwrap()
        );
        assert_eq!(
            out.cut_edge[1],
            out.vertices
                .iter()
                .position(|v| v.position == [0.0, 1.0, 0.0])
                .unwrap()
        );
    }

    #[test]
    fn opposite_seed_retains_complementary_area() {
        let tri = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let out = split_triangle_by_segment(tri, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.4, 0.4, 0.0])
            .unwrap();
        assert!((area_of(&out) - 1.5).abs() <= 1e-9);
        assert_eq!(out.triangles.len(), 2);
    }

    #[test]
    fn deterministic_repeat() {
        let tri = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let a = split_triangle_by_segment(tri, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.1, 0.1, 0.0])
            .unwrap();
        let b = split_triangle_by_segment(tri, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.1, 0.1, 0.0])
            .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn invalid_endpoint_rejected() {
        let tri = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let err =
            split_triangle_by_segment(tri, [1.0, 0.25, 0.0], [0.0, 1.0, 0.0], [0.1, 0.1, 0.0])
                .unwrap_err();
        assert_eq!(err, SplitTriangleError::EndpointOffBoundary);
    }

    #[test]
    fn v_shaped_polyline_retains_corner_and_preserves_cut_vertices() {
        let tri = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let cut = [[1.0, 0.0, 0.0], [0.35, 0.35, 0.0], [0.0, 1.0, 0.0]];
        let out = split_triangle_by_polyline(tri, &cut, [0.1, 0.1, 0.0]).unwrap();
        assert_eq!(out.cut_path.len(), 3);
        for (index, point) in cut.iter().enumerate() {
            assert_eq!(out.vertices[out.cut_path[index]].position, *point);
        }
        assert!(retained_area(&out.vertices, &out.triangles) > 0.0);
        assert!(retained_area(&out.vertices, &out.triangles) < 2.0);
    }

    #[test]
    fn self_intersecting_polyline_is_rejected() {
        let tri = [[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [0.0, 4.0, 0.0]];
        let cut = [
            [2.0, 0.0, 0.0],
            [0.5, 2.0, 0.0],
            [2.0, 1.0, 0.0],
            [0.0, 2.0, 0.0],
        ];
        let error = split_triangle_by_polyline(tri, &cut, [0.1, 0.1, 0.0]).unwrap_err();
        assert_eq!(error, SplitTriangleError::SelfIntersectingCut);
    }
}
