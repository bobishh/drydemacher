#pragma once

#include <array>
#include <chrono>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <limits>
#include <map>
#include <mutex>
#include <optional>
#include <sstream>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include "execution_identity.hpp"

namespace ecky {

// Bump whenever triangle encoding, coordinate policy, or tessellation settings change.
inline constexpr char kPartMeshPolicyIdentity[] = "direct-occt-part-mesh-v2|linear=0.04|angular=adaptive";
inline constexpr char kPartMeshCacheSchema[] = "direct-occt-part-mesh-cache-v3";

struct MeshPoint {
    double x = 0.0;
    double y = 0.0;
    double z = 0.0;
};

struct MeshTriangle {
    std::array<MeshPoint, 3> vertices{};
};

struct PartMesh {
    std::string part_id;
    std::vector<MeshTriangle> triangles;

    std::size_t resident_bytes() const {
        return sizeof(PartMesh) + part_id.capacity() + triangles.capacity() * sizeof(MeshTriangle);
    }
};

// Shared renderer admission point. Caller holds the returned lease while OCCT
// tessellation and triangle collection own memory. This module never guesses a
// budget; scheduler supplies one process-wide budget for BRep, mesh, and export.
class PartMeshMemoryBudget {
public:
    class Lease {
    public:
        Lease() = default;
        Lease(PartMeshMemoryBudget* budget, std::size_t bytes) : budget_(budget), bytes_(bytes) {}
        Lease(const Lease&) = delete;
        Lease& operator=(const Lease&) = delete;
        Lease(Lease&& other) noexcept : budget_(other.budget_), bytes_(other.bytes_) {
            other.budget_ = nullptr;
            other.bytes_ = 0;
        }
        Lease& operator=(Lease&& other) noexcept {
            if (this != &other) {
                release();
                budget_ = other.budget_;
                bytes_ = other.bytes_;
                other.budget_ = nullptr;
                other.bytes_ = 0;
            }
            return *this;
        }
        ~Lease() { release(); }

    private:
        void release() {
            if (budget_ != nullptr) {
                budget_->release(bytes_);
                budget_ = nullptr;
            }
        }
        PartMeshMemoryBudget* budget_ = nullptr;
        std::size_t bytes_ = 0;
    };

    explicit PartMeshMemoryBudget(std::size_t limit_bytes) : limit_bytes_(limit_bytes) {}

    std::optional<Lease> try_reserve(std::size_t bytes) {
        std::lock_guard<std::mutex> lock(mutex_);
        if (bytes > limit_bytes_ - reserved_bytes_) return std::nullopt;
        reserved_bytes_ += bytes;
        return Lease(this, bytes);
    }

    std::size_t reserved_bytes() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return reserved_bytes_;
    }

private:
    void release(std::size_t bytes) {
        std::lock_guard<std::mutex> lock(mutex_);
        reserved_bytes_ -= bytes;
    }

    mutable std::mutex mutex_;
    std::size_t limit_bytes_ = 0;
    std::size_t reserved_bytes_ = 0;
};

inline std::string part_mesh_digest(const std::string& input) {
    return "sha256:" + sha256_hex(input);
}

inline std::string part_mesh_identity(const std::string& part_brep_identity, const std::string& policy_identity) {
    return part_mesh_digest(std::string(kPartMeshCacheSchema) + "|" + policy_identity + "|" + part_brep_identity);
}

inline PartMesh assemble_preview_mesh(const std::vector<PartMesh>& parts) {
    PartMesh preview;
    preview.part_id = "preview";
    std::size_t triangle_count = 0;
    for (const PartMesh& part : parts) triangle_count += part.triangles.size();
    preview.triangles.reserve(triangle_count);
    // Plan order is output order. Never use worker completion order here.
    for (const PartMesh& part : parts) {
        preview.triangles.insert(preview.triangles.end(), part.triangles.begin(), part.triangles.end());
    }
    return preview;
}

class PartMeshCache {
public:
    explicit PartMeshCache(std::filesystem::path root) : root_(std::move(root)) {
        std::filesystem::create_directories(root_);
    }

    std::optional<PartMesh> read(const std::string& identity) const {
        const std::filesystem::path artifact = artifact_path(identity);
        const std::filesystem::path metadata = metadata_path(identity);
        try {
            std::ifstream meta(metadata);
            std::string schema, stored_identity, stored_digest;
            std::uint64_t stored_size = 0;
            if (!(meta >> schema >> stored_identity >> stored_digest >> stored_size) ||
                schema != kPartMeshCacheSchema || stored_identity != identity ||
                !std::filesystem::is_regular_file(artifact) ||
                std::filesystem::file_size(artifact) != stored_size) {
                invalidate(artifact, metadata);
                return std::nullopt;
            }
            const std::string bytes = read_bytes(artifact);
            if (part_mesh_digest(bytes) != stored_digest) {
                invalidate(artifact, metadata);
                return std::nullopt;
            }
            return decode(bytes);
        } catch (...) {
            invalidate(artifact, metadata);
            return std::nullopt;
        }
    }

    void write(const std::string& identity, const PartMesh& mesh) const {
        const std::string bytes = encode(mesh);
        const std::filesystem::path artifact = artifact_path(identity);
        const std::filesystem::path metadata = metadata_path(identity);
        const std::string suffix = ".tmp-" + std::to_string(
            std::chrono::steady_clock::now().time_since_epoch().count());
        const std::filesystem::path artifact_tmp = artifact.string() + suffix;
        const std::filesystem::path metadata_tmp = metadata.string() + suffix;
        write_bytes(artifact_tmp, bytes);
        std::ofstream meta(metadata_tmp);
        if (!meta) throw std::runtime_error("cannot write part mesh metadata");
        meta << kPartMeshCacheSchema << ' ' << identity << ' ' << part_mesh_digest(bytes) << ' '
             << bytes.size() << '\n';
        meta.close();
        std::error_code error;
        std::filesystem::rename(artifact_tmp, artifact, error);
        if (error) {
            std::filesystem::remove(artifact_tmp, error);
            std::filesystem::remove(metadata_tmp, error);
            throw std::runtime_error("cannot publish part mesh artifact");
        }
        std::filesystem::rename(metadata_tmp, metadata, error);
        if (error) {
            std::filesystem::remove(metadata_tmp, error);
            std::filesystem::remove(artifact, error);
            throw std::runtime_error("cannot publish part mesh metadata");
        }
    }

private:
    static std::string read_bytes(const std::filesystem::path& path) {
        std::ifstream input(path, std::ios::binary);
        if (!input) throw std::runtime_error("cannot read part mesh artifact");
        std::ostringstream bytes;
        bytes << input.rdbuf();
        return bytes.str();
    }

    static void write_bytes(const std::filesystem::path& path, const std::string& bytes) {
        std::ofstream output(path, std::ios::binary);
        if (!output) throw std::runtime_error("cannot write part mesh artifact");
        output.write(bytes.data(), static_cast<std::streamsize>(bytes.size()));
        if (!output.good()) throw std::runtime_error("cannot finish part mesh artifact");
    }

    static void append_u64(std::string& bytes, std::uint64_t value) {
        for (unsigned shift = 0; shift != 64; shift += 8) bytes.push_back(static_cast<char>(value >> shift));
    }

    static std::uint64_t take_u64(const std::string& bytes, std::size_t& offset) {
        if (bytes.size() - offset < 8) throw std::runtime_error("truncated part mesh artifact");
        std::uint64_t value = 0;
        for (unsigned shift = 0; shift != 64; shift += 8) {
            value |= static_cast<std::uint64_t>(static_cast<unsigned char>(bytes[offset++])) << shift;
        }
        return value;
    }

    static void append_double(std::string& bytes, double value) {
        static_assert(sizeof(double) == sizeof(std::uint64_t));
        std::uint64_t raw = 0;
        std::memcpy(&raw, &value, sizeof(raw));
        append_u64(bytes, raw);
    }

    static double take_double(const std::string& bytes, std::size_t& offset) {
        const std::uint64_t raw = take_u64(bytes, offset);
        double value = 0.0;
        std::memcpy(&value, &raw, sizeof(value));
        return value;
    }

    static std::string encode(const PartMesh& mesh) {
        std::string bytes("ECKYPM01", 8);
        append_u64(bytes, mesh.part_id.size());
        bytes += mesh.part_id;
        append_u64(bytes, mesh.triangles.size());
        for (const MeshTriangle& triangle : mesh.triangles) {
            for (const MeshPoint& point : triangle.vertices) {
                append_double(bytes, point.x);
                append_double(bytes, point.y);
                append_double(bytes, point.z);
            }
        }
        return bytes;
    }

    static PartMesh decode(const std::string& bytes) {
        if (bytes.size() < 8 || bytes.compare(0, 8, "ECKYPM01") != 0) {
            throw std::runtime_error("invalid part mesh artifact schema");
        }
        std::size_t offset = 8;
        const std::uint64_t id_size = take_u64(bytes, offset);
        if (id_size > bytes.size() - offset) throw std::runtime_error("invalid part mesh id");
        PartMesh mesh;
        mesh.part_id.assign(bytes.data() + offset, static_cast<std::size_t>(id_size));
        offset += static_cast<std::size_t>(id_size);
        const std::uint64_t triangle_count = take_u64(bytes, offset);
        if (triangle_count > (bytes.size() - offset) / (9 * sizeof(double))) {
            throw std::runtime_error("invalid part mesh triangle count");
        }
        mesh.triangles.resize(static_cast<std::size_t>(triangle_count));
        for (MeshTriangle& triangle : mesh.triangles) {
            for (MeshPoint& point : triangle.vertices) {
                point.x = take_double(bytes, offset);
                point.y = take_double(bytes, offset);
                point.z = take_double(bytes, offset);
            }
        }
        if (offset != bytes.size()) throw std::runtime_error("trailing part mesh bytes");
        return mesh;
    }

    static void invalidate(const std::filesystem::path& artifact, const std::filesystem::path& metadata) {
        std::error_code error;
        std::filesystem::remove(artifact, error);
        std::filesystem::remove(metadata, error);
    }

    std::filesystem::path artifact_path(const std::string& identity) const { return root_ / (identity + ".partmesh"); }
    std::filesystem::path metadata_path(const std::string& identity) const { return root_ / (identity + ".meta"); }

    std::filesystem::path root_;
};

}  // namespace ecky
