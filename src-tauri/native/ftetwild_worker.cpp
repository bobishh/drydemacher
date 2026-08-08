// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at https://mozilla.org/MPL/2.0/.
//
// Ecky fTetWild structured-array worker. This file is shipped with corresponding
// source and is intentionally isolated from the Tauri process.

#include <floattetwild/AABBWrapper.h>
#include <floattetwild/FloatTetDelaunay.h>
#include <floattetwild/LocalOperations.h>
#include <floattetwild/Logger.hpp>
#include <floattetwild/Mesh.hpp>
#include <floattetwild/MeshIO.hpp>
#include <floattetwild/MeshImprovement.h>
#include <floattetwild/Simplification.h>
#include <floattetwild/TriangleInsertion.h>
#include <floattetwild/Types.hpp>

#include <geogram/basic/common.h>

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <limits>
#include <map>
#include <numeric>
#include <set>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace {

using floatTetWild::AABBWrapper;
using floatTetWild::Mesh;
using floatTetWild::Parameters;
using floatTetWild::Vector3;
using floatTetWild::Vector3i;
using json = nlohmann::json;

constexpr const char* kProtocol = "ecky-ftetwild-worker-v1";
constexpr int kSurfaceTagBase = 1024;

struct LocalRefinement {
    std::set<std::uint32_t> groups;
    double target_edge_length_mm = 0.0;
};

struct WorkerInput {
    std::string request_id;
    std::vector<Vector3> vertices;
    std::vector<Vector3i> triangles;
    std::vector<int> face_tags;
    std::uint32_t face_group_count = 0;
    double target_edge_length_mm = 0.0;
    double envelope_mm = 0.0;
    double minimum_scaled_jacobian = 0.0;
    std::uint64_t maximum_nodes = 0;
    std::uint64_t maximum_tet4_cells = 0;
    std::uint64_t maximum_result_bytes = 0;
    std::vector<LocalRefinement> local_refinements;
};

struct SourceTriangle {
    std::array<Vector3, 3> points;
    std::uint32_t group = 0;
};

double finite_positive(const json& value, const char* field)
{
    const double parsed = value.get<double>();
    if (!std::isfinite(parsed) || parsed <= 0.0) {
        throw std::runtime_error(std::string(field) + " must be finite and positive");
    }
    return parsed;
}

std::uint64_t positive_u64(const json& value, const char* field)
{
    const auto parsed = value.get<std::uint64_t>();
    if (parsed == 0) {
        throw std::runtime_error(std::string(field) + " must be positive");
    }
    return parsed;
}

WorkerInput parse_request(const json& root)
{
    if (root.at("schemaVersion").get<std::uint32_t>() != 1) {
        throw std::runtime_error("unsupported schemaVersion");
    }
    if (root.at("workerProtocol").get<std::string>() != kProtocol) {
        throw std::runtime_error("workerProtocol mismatch");
    }
    WorkerInput input;
    input.request_id = root.at("requestId").get<std::string>();
    if (input.request_id.empty()) {
        throw std::runtime_error("requestId must not be empty");
    }
    const auto vertices = root.at("verticesMm").get<std::vector<double>>();
    const auto triangles = root.at("triangles").get<std::vector<std::uint32_t>>();
    const auto groups = root.at("triangleFaceGroupIndices").get<std::vector<std::uint32_t>>();
    input.face_group_count = root.at("faceGroupCount").get<std::uint32_t>();
    if (input.face_group_count == 0
        || input.face_group_count
            > static_cast<std::uint32_t>(std::numeric_limits<int>::max() - kSurfaceTagBase)) {
        throw std::runtime_error("faceGroupCount exceeds wide upstream tag range");
    }
    if (vertices.size() < 12 || vertices.size() % 3 != 0) {
        throw std::runtime_error("verticesMm must contain at least four xyz points");
    }
    if (triangles.size() < 12 || triangles.size() % 3 != 0
        || groups.size() != triangles.size() / 3) {
        throw std::runtime_error("triangle arrays have invalid cardinality");
    }
    input.vertices.reserve(vertices.size() / 3);
    for (std::size_t index = 0; index < vertices.size(); index += 3) {
        if (!std::isfinite(vertices[index]) || !std::isfinite(vertices[index + 1])
            || !std::isfinite(vertices[index + 2])) {
            throw std::runtime_error("verticesMm contains a non-finite coordinate");
        }
        input.vertices.emplace_back(vertices[index], vertices[index + 1], vertices[index + 2]);
    }
    input.triangles.reserve(triangles.size() / 3);
    input.face_tags.reserve(groups.size());
    for (std::size_t index = 0; index < triangles.size(); index += 3) {
        for (std::size_t axis = 0; axis < 3; ++axis) {
            if (triangles[index + axis] >= input.vertices.size()
                || triangles[index + axis] > static_cast<std::uint32_t>(std::numeric_limits<int>::max())) {
                throw std::runtime_error("triangle contains an out-of-range vertex index");
            }
        }
        const auto group = groups[index / 3];
        if (group >= input.face_group_count) {
            throw std::runtime_error("triangle contains an out-of-range face group");
        }
        input.triangles.emplace_back(
            static_cast<int>(triangles[index]),
            static_cast<int>(triangles[index + 1]),
            static_cast<int>(triangles[index + 2]));
        input.face_tags.push_back(static_cast<int>(group) + kSurfaceTagBase);
    }

    const auto& control = root.at("control");
    if (control.at("elementOrder").get<std::uint32_t>() != 1
        || control.at("deterministicThreadCount").get<std::uint32_t>() != 1
        || control.at("allowHoleFilling").get<bool>()) {
        throw std::runtime_error("worker requires Tet4, one deterministic thread, and no hole filling");
    }
    input.target_edge_length_mm =
        finite_positive(control.at("targetEdgeLengthMm"), "targetEdgeLengthMm");
    input.envelope_mm = finite_positive(control.at("envelopeMm"), "envelopeMm");
    input.minimum_scaled_jacobian =
        finite_positive(control.at("minimumScaledJacobian"), "minimumScaledJacobian");
    if (input.minimum_scaled_jacobian > 1.0) {
        throw std::runtime_error("minimumScaledJacobian must not exceed 1");
    }
    input.maximum_nodes = positive_u64(control.at("maximumNodes"), "maximumNodes");
    input.maximum_tet4_cells =
        positive_u64(control.at("maximumTet4Cells"), "maximumTet4Cells");
    input.maximum_result_bytes =
        positive_u64(control.at("maximumResultBytes"), "maximumResultBytes");
    for (const auto& refinement_json : control.at("localRefinements")) {
        LocalRefinement refinement;
        refinement.target_edge_length_mm = finite_positive(
            refinement_json.at("targetEdgeLengthMm"), "local targetEdgeLengthMm");
        if (refinement.target_edge_length_mm > input.target_edge_length_mm) {
            throw std::runtime_error("local targetEdgeLengthMm exceeds global target");
        }
        for (const auto group : refinement_json.at("faceGroupIndices").get<std::vector<std::uint32_t>>()) {
            if (group >= input.face_group_count || !refinement.groups.insert(group).second) {
                throw std::runtime_error("local refinement face groups are invalid or duplicate");
            }
        }
        if (refinement.groups.empty()) {
            throw std::runtime_error("local refinement must target a face group");
        }
        input.local_refinements.push_back(std::move(refinement));
    }
    return input;
}

double point_segment_distance_squared(const Vector3& point, const Vector3& from, const Vector3& to)
{
    const Vector3 edge = to - from;
    const double length_squared = edge.squaredNorm();
    if (length_squared == 0.0) {
        return (point - from).squaredNorm();
    }
    const double parameter = std::max(0.0, std::min(1.0, (point - from).dot(edge) / length_squared));
    return (point - (from + parameter * edge)).squaredNorm();
}

double point_triangle_distance_squared(const Vector3& point, const std::array<Vector3, 3>& triangle)
{
    const Vector3 ab = triangle[1] - triangle[0];
    const Vector3 ac = triangle[2] - triangle[0];
    const Vector3 normal = ab.cross(ac);
    const double normal_squared = normal.squaredNorm();
    if (normal_squared <= std::numeric_limits<double>::epsilon()) {
        return std::min({
            point_segment_distance_squared(point, triangle[0], triangle[1]),
            point_segment_distance_squared(point, triangle[1], triangle[2]),
            point_segment_distance_squared(point, triangle[2], triangle[0]),
        });
    }
    const Vector3 projected = point - normal * ((point - triangle[0]).dot(normal) / normal_squared);
    const Vector3 v0 = triangle[1] - triangle[0];
    const Vector3 v1 = triangle[2] - triangle[0];
    const Vector3 v2 = projected - triangle[0];
    const double d00 = v0.dot(v0);
    const double d01 = v0.dot(v1);
    const double d11 = v1.dot(v1);
    const double d20 = v2.dot(v0);
    const double d21 = v2.dot(v1);
    const double denominator = d00 * d11 - d01 * d01;
    const double v = (d11 * d20 - d01 * d21) / denominator;
    const double w = (d00 * d21 - d01 * d20) / denominator;
    const double u = 1.0 - v - w;
    if (u >= 0.0 && v >= 0.0 && w >= 0.0) {
        return (point - projected).squaredNorm();
    }
    return std::min({
        point_segment_distance_squared(point, triangle[0], triangle[1]),
        point_segment_distance_squared(point, triangle[1], triangle[2]),
        point_segment_distance_squared(point, triangle[2], triangle[0]),
    });
}

std::vector<SourceTriangle> source_triangles(const WorkerInput& input)
{
    std::vector<SourceTriangle> result;
    result.reserve(input.triangles.size());
    for (std::size_t index = 0; index < input.triangles.size(); ++index) {
        const auto& triangle = input.triangles[index];
        result.push_back({
            {input.vertices[triangle[0]], input.vertices[triangle[1]], input.vertices[triangle[2]]},
            static_cast<std::uint32_t>(input.face_tags[index] - kSurfaceTagBase),
        });
    }
    return result;
}

double squared_distance_to_groups(
    const Vector3& point,
    const std::vector<SourceTriangle>& surface,
    const std::set<std::uint32_t>& groups)
{
    double minimum = std::numeric_limits<double>::infinity();
    for (const auto& triangle : surface) {
        if (groups.find(triangle.group) != groups.end()) {
            minimum = std::min(minimum, point_triangle_distance_squared(point, triangle.points));
        }
    }
    return minimum;
}

std::uint32_t reconcile_source_group(
    const Vector3& point,
    const std::vector<SourceTriangle>& source_surface,
    double envelope_mm)
{
    double minimum = std::numeric_limits<double>::infinity();
    double other_group_minimum = std::numeric_limits<double>::infinity();
    std::uint32_t group = 0;
    for (const auto& source : source_surface) {
        const double distance = point_triangle_distance_squared(point, source.points);
        if (distance < minimum) {
            if (source.group != group) {
                other_group_minimum = minimum;
            }
            minimum = distance;
            group = source.group;
        } else if (source.group != group) {
            other_group_minimum = std::min(other_group_minimum, distance);
        }
    }
    if (!std::isfinite(minimum) || std::sqrt(minimum) > envelope_mm) {
        throw std::runtime_error("native exterior facet exceeds source envelope during tag reconciliation");
    }
    const double ambiguity_tolerance = std::max(1.0e-20, envelope_mm * envelope_mm * 1.0e-8);
    if (other_group_minimum - minimum <= ambiguity_tolerance) {
        throw std::runtime_error("native exterior facet has ambiguous source face-group ownership");
    }
    return group;
}

Mesh tetrahedralize(
    const WorkerInput& request,
    const std::vector<SourceTriangle>& source_surface,
    std::uint64_t& insertion_count)
{
    std::vector<Vector3> vertices = request.vertices;
    std::vector<Vector3i> triangles = request.triangles;
    std::vector<int> tags = request.face_tags;
    GEO::Mesh surface_mesh;
    floatTetWild::MeshIO::load_mesh(vertices, triangles, surface_mesh, tags);
    AABBWrapper tree(surface_mesh);

    Parameters params;
    params.is_quiet = true;
    params.log_level = 6;
    params.not_sort_input = true;
    params.num_threads = 1;
    params.ideal_edge_length_abs = request.target_edge_length_mm;
    params.eps_rel = request.envelope_mm / tree.get_sf_diag();
    const double minimum_size = std::accumulate(
        request.local_refinements.begin(),
        request.local_refinements.end(),
        request.target_edge_length_mm,
        [](double current, const LocalRefinement& refinement) {
            return std::min(current, refinement.target_edge_length_mm);
        });
    params.min_edge_len_rel = minimum_size / tree.get_sf_diag();
    params.use_input_for_wn = true;
    if (!request.local_refinements.empty()) {
        params.apply_sizing_field = true;
        params.get_sizing_field_value = [source_surface, refinements = request.local_refinements,
                                            global_size = request.target_edge_length_mm](const Vector3& point) {
            double target = global_size;
            for (const auto& refinement : refinements) {
                const double distance_squared =
                    squared_distance_to_groups(point, source_surface, refinement.groups);
                const double influence = 2.0 * refinement.target_edge_length_mm;
                if (distance_squared <= influence * influence) {
                    target = std::min(target, refinement.target_edge_length_mm);
                }
            }
            return target;
        };
    }
    if (!params.init(tree.get_sf_diag())) {
        throw std::runtime_error("fTetWild parameter initialization failed");
    }

    Mesh mesh;
    mesh.params = params;
    // Simplification may collapse an entire small CAD face group. Preserve the
    // exact tagged analysis boundary; volume optimization remains enabled.
    floatTetWild::simplify(vertices, triangles, tags, tree, mesh.params, true);
    tree.init_b_mesh_and_tree(vertices, triangles, mesh);
    std::vector<bool> inserted(triangles.size(), false);
    floatTetWild::FloatTetDelaunay::tetrahedralize(vertices, triangles, tree, mesh, inserted);
    floatTetWild::insert_triangles(vertices, triangles, tags, mesh, inserted, tree, false);
    insertion_count = static_cast<std::uint64_t>(std::count(inserted.begin(), inserted.end(), true));
    floatTetWild::optimization(vertices, triangles, tags, inserted, mesh, tree, {{1, 1, 1, 1}});
    floatTetWild::filter_outside(mesh, vertices, triangles);
    if (mesh.get_t_num() <= 0 || mesh.get_v_num() <= 0) {
        throw std::runtime_error("fTetWild produced an empty interior mesh");
    }
    return mesh;
}

struct BoundaryOwner {
    std::size_t tet_index = 0;
    int local_face = 0;
    std::array<int, 3> vertices = {};
};

json build_response(
    const WorkerInput& request,
    const std::vector<SourceTriangle>& source_surface,
    const Mesh& mesh,
    std::uint64_t insertion_count)
{
    std::vector<int> old_to_new(mesh.tet_vertices.size(), -1);
    std::vector<double> nodes;
    for (std::size_t index = 0; index < mesh.tet_vertices.size(); ++index) {
        if (mesh.tet_vertices[index].is_removed) {
            continue;
        }
        if (nodes.size() / 3 >= request.maximum_nodes) {
            throw std::runtime_error("maximumNodes exceeded during native extraction");
        }
        old_to_new[index] = static_cast<int>(nodes.size() / 3);
        const auto& point = mesh.tet_vertices[index].pos;
        nodes.insert(nodes.end(), {point[0], point[1], point[2]});
    }

    std::vector<std::uint32_t> cells;
    std::vector<bool> active(mesh.tets.size(), false);
    for (std::size_t index = 0; index < mesh.tets.size(); ++index) {
        if (mesh.tets[index].is_removed) {
            continue;
        }
        if (cells.size() / 4 >= request.maximum_tet4_cells) {
            throw std::runtime_error("maximumTet4Cells exceeded during native extraction");
        }
        active[index] = true;
        for (int local = 0; local < 4; ++local) {
            const int mapped = old_to_new[mesh.tets[index][local]];
            if (mapped < 0) {
                throw std::runtime_error("active Tet4 references a removed vertex");
            }
            cells.push_back(static_cast<std::uint32_t>(mapped));
        }
    }

    std::map<std::array<int, 3>, std::vector<BoundaryOwner>> owners;
    for (std::size_t tet_index = 0; tet_index < mesh.tets.size(); ++tet_index) {
        if (!active[tet_index]) {
            continue;
        }
        const auto& tet = mesh.tets[tet_index];
        for (int local_face = 0; local_face < 4; ++local_face) {
            std::array<int, 3> face = {
                tet[(local_face + 1) % 4],
                tet[(local_face + 2) % 4],
                tet[(local_face + 3) % 4],
            };
            auto key = face;
            std::sort(key.begin(), key.end());
            owners[key].push_back({tet_index, local_face, face});
        }
    }

    std::vector<std::uint32_t> boundary;
    std::vector<std::uint32_t> boundary_groups;
    std::set<std::uint32_t> used_boundary_nodes;
    std::set<std::uint32_t> used_boundary_groups;
    for (const auto& entry : owners) {
        if (entry.second.size() == 2) {
            continue;
        }
        if (entry.second.size() != 1) {
            throw std::runtime_error("native Tet4 mesh contains a non-manifold facet");
        }
        const auto& owner = entry.second.front();
        const int propagated_tag = mesh.tets[owner.tet_index].surface_tags[owner.local_face];
        if (propagated_tag < kSurfaceTagBase
            || propagated_tag >= kSurfaceTagBase + static_cast<int>(request.face_group_count)) {
            throw std::runtime_error("native exterior facet lost its source face-group tag");
        }
        Vector3 centroid = Vector3::Zero();
        for (const int old_index : owner.vertices) {
            const int mapped = old_to_new[old_index];
            boundary.push_back(static_cast<std::uint32_t>(mapped));
            used_boundary_nodes.insert(static_cast<std::uint32_t>(mapped));
            centroid += mesh.tet_vertices[old_index].pos;
        }
        centroid /= 3.0;
        const auto reconciled_group =
            reconcile_source_group(centroid, source_surface, request.envelope_mm);
        boundary_groups.push_back(reconciled_group);
        used_boundary_groups.insert(reconciled_group);
    }
    if (boundary.empty()) {
        throw std::runtime_error("native Tet4 mesh contains no exterior facets");
    }
    if (used_boundary_groups.size() != request.face_group_count) {
        throw std::runtime_error("native exterior facets do not cover every source face group");
    }

    double maximum_boundary_deviation_mm = 0.0;
    for (const auto node_index : used_boundary_nodes) {
        const Vector3 point(nodes[node_index * 3], nodes[node_index * 3 + 1], nodes[node_index * 3 + 2]);
        double minimum_squared = std::numeric_limits<double>::infinity();
        for (const auto& source : source_surface) {
            minimum_squared =
                std::min(minimum_squared, point_triangle_distance_squared(point, source.points));
        }
        maximum_boundary_deviation_mm =
            std::max(maximum_boundary_deviation_mm, std::sqrt(minimum_squared));
    }

    json response = {
        {"schemaVersion", 1},
        {"workerProtocol", kProtocol},
        {"requestId", request.request_id},
        {"nodesMm", nodes},
        {"tet4Cells", cells},
        {"boundaryTriangles", boundary},
        {"boundaryFaceGroupIndices", boundary_groups},
        {"faceGroupCount", request.face_group_count},
        {"insertionCount", insertion_count},
        {"maximumBoundaryDeviationMm", maximum_boundary_deviation_mm},
        {"threadCount", 1},
    };
    const std::string encoded = response.dump();
    if (encoded.size() > request.maximum_result_bytes) {
        throw std::runtime_error("maximumResultBytes exceeded during native extraction");
    }
    return response;
}

std::pair<std::string, std::string> parse_cli(int argc, char** argv)
{
    std::string request_path;
    std::string response_path;
    std::string protocol;
    for (int index = 1; index < argc; ++index) {
        const std::string argument = argv[index];
        if (index + 1 >= argc) {
            throw std::runtime_error("worker argument is missing a value");
        }
        const std::string value = argv[++index];
        if (argument == "--request") {
            request_path = value;
        } else if (argument == "--response") {
            response_path = value;
        } else if (argument == "--protocol") {
            protocol = value;
        } else {
            throw std::runtime_error("unsupported worker argument: " + argument);
        }
    }
    if (protocol != kProtocol || request_path.empty() || response_path.empty()) {
        throw std::runtime_error("worker requires exact protocol, request, and response arguments");
    }
    return {request_path, response_path};
}

json read_json(const std::string& path)
{
    std::ifstream stream(path);
    if (!stream) {
        throw std::runtime_error("could not open request file");
    }
    json value;
    stream >> value;
    return value;
}

void write_json(const std::string& path, const json& value)
{
    std::ofstream stream(path, std::ios::binary | std::ios::trunc);
    if (!stream) {
        throw std::runtime_error("could not open response file");
    }
    stream << value.dump();
    stream.flush();
    if (!stream) {
        throw std::runtime_error("could not write complete response file");
    }
}

} // namespace

int main(int argc, char** argv)
{
    try {
#ifndef _WIN32
        setenv("GEO_NO_SIGNAL_HANDLER", "1", 1);
#endif
        const auto paths = parse_cli(argc, argv);
        const WorkerInput request = parse_request(read_json(paths.first));
        GEO::initialize();
        floatTetWild::Logger::init(false);
        const auto source_surface = source_triangles(request);
        std::uint64_t insertion_count = 0;
        const Mesh mesh = tetrahedralize(request, source_surface, insertion_count);
        write_json(paths.second, build_response(request, source_surface, mesh, insertion_count));
        return EXIT_SUCCESS;
    } catch (const std::exception& error) {
        std::cerr << "ecky fTetWild worker failed: " << error.what() << std::endl;
        return EXIT_FAILURE;
    }
}
