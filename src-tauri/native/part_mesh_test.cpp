#include "part_mesh.hpp"

#include <cassert>
#include <filesystem>
#include <fstream>

namespace fs = std::filesystem;

namespace {

ecky::PartMesh triangle_mesh(const std::string& part_id, double x) {
    ecky::PartMesh mesh;
    mesh.part_id = part_id;
    ecky::MeshTriangle triangle;
    triangle.vertices[0] = {x, 0.0, 0.0};
    triangle.vertices[1] = {x + 1.0, 0.0, 0.0};
    triangle.vertices[2] = {x, 1.0, 0.0};
    mesh.triangles.push_back(triangle);
    return mesh;
}

void given_two_final_part_meshes_when_preview_and_parts_are_written_then_each_part_is_reused_once() {
    const std::vector<ecky::PartMesh> parts = {
        triangle_mesh("left", 0.0),
        triangle_mesh("right", 10.0),
    };
    const ecky::PartMesh preview = ecky::assemble_preview_mesh(parts);

    assert(preview.triangles.size() == 2);
    assert(preview.triangles[0].vertices[0].x == 0.0);
    assert(preview.triangles[1].vertices[0].x == 10.0);
    assert(ecky::part_mesh_identity("left-brep", ecky::kPartMeshPolicyIdentity) ==
           ecky::part_mesh_identity("left-brep", ecky::kPartMeshPolicyIdentity));
    assert(ecky::part_mesh_identity("left-brep", ecky::kPartMeshPolicyIdentity) !=
           ecky::part_mesh_identity("right-brep", ecky::kPartMeshPolicyIdentity));
}

void given_a_valid_cached_mesh_when_read_then_policy_and_digest_must_match() {
    const fs::path root = fs::temp_directory_path() / "ecky-part-mesh-cache-test";
    std::error_code error;
    fs::remove_all(root, error);
    fs::create_directories(root);

    const ecky::PartMesh mesh = triangle_mesh("body", 0.0);
    const std::string identity = ecky::part_mesh_identity("body-brep", ecky::kPartMeshPolicyIdentity);
    ecky::PartMeshCache cache(root);
    cache.write(identity, mesh);
    const std::optional<ecky::PartMesh> hit = cache.read(identity);
    assert(hit.has_value());
    assert(hit->part_id == "body");
    assert(hit->triangles.size() == 1);

    std::ofstream tamper(root / (identity + ".partmesh"), std::ios::app | std::ios::binary);
    tamper.put('x');
    tamper.close();
    assert(!cache.read(identity).has_value());
    fs::remove_all(root, error);
}

}  // namespace

int main() {
    given_two_final_part_meshes_when_preview_and_parts_are_written_then_each_part_is_reused_once();
    given_a_valid_cached_mesh_when_read_then_policy_and_digest_must_match();
}
