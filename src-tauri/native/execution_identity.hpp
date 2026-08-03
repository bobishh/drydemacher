#pragma once

#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <sstream>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ecky {

// Resolved inputs only. Caller owns lowering parameters and imported files into
// these values before identity generation; source spans and plan slot IDs never enter.
struct ExecutionIdentityInput {
    std::string cache_schema;
    std::string runner_abi;
    std::string runner_binary_digest;
    std::string occt_runtime;
    std::string tolerance_policy;
    std::string tessellation_policy;
    std::string op;
    // Positional argument order is semantic. Keywords and selector payloads
    // are independent named fields and normalize independently below.
    std::vector<std::string> resolved_args;
    std::vector<std::string> normalized_keywords;
    std::vector<std::string> selectors;
    std::vector<std::string> ordered_dependency_identities;
    std::vector<std::string> import_payloads;
};

class Sha256 {
public:
    Sha256() : state_{0x6a09e667U, 0xbb67ae85U, 0x3c6ef372U, 0xa54ff53aU,
                      0x510e527fU, 0x9b05688cU, 0x1f83d9abU, 0x5be0cd19U} {}

    void update(const std::string& value) { update(reinterpret_cast<const std::uint8_t*>(value.data()), value.size()); }

    std::string finish_hex() {
        const std::uint64_t bits = total_bytes_ * 8;
        std::uint8_t one = 0x80;
        update(&one, 1);
        std::uint8_t zero = 0;
        while (buffer_size_ != 56) update(&zero, 1);
        std::uint8_t length[8];
        for (int i = 7; i >= 0; --i) length[7 - i] = static_cast<std::uint8_t>(bits >> (i * 8));
        update(length, sizeof(length));
        std::ostringstream out;
        for (std::uint32_t word : state_) out << std::hex << std::setw(8) << std::setfill('0') << word;
        return out.str();
    }

private:
    static std::uint32_t rotate_right(std::uint32_t value, std::uint32_t bits) {
        return (value >> bits) | (value << (32 - bits));
    }
    void update(const std::uint8_t* bytes, std::size_t size) {
        total_bytes_ += size;
        while (size) {
            const std::size_t copied = std::min(size, buffer_.size() - buffer_size_);
            for (std::size_t i = 0; i < copied; ++i) buffer_[buffer_size_ + i] = bytes[i];
            buffer_size_ += copied; bytes += copied; size -= copied;
            if (buffer_size_ == buffer_.size()) { transform(); buffer_size_ = 0; }
        }
    }
    void transform() {
        static constexpr std::array<std::uint32_t, 64> k = {
            0x428a2f98U,0x71374491U,0xb5c0fbcfU,0xe9b5dba5U,0x3956c25bU,0x59f111f1U,0x923f82a4U,0xab1c5ed5U,
            0xd807aa98U,0x12835b01U,0x243185beU,0x550c7dc3U,0x72be5d74U,0x80deb1feU,0x9bdc06a7U,0xc19bf174U,
            0xe49b69c1U,0xefbe4786U,0x0fc19dc6U,0x240ca1ccU,0x2de92c6fU,0x4a7484aaU,0x5cb0a9dcU,0x76f988daU,
            0x983e5152U,0xa831c66dU,0xb00327c8U,0xbf597fc7U,0xc6e00bf3U,0xd5a79147U,0x06ca6351U,0x14292967U,
            0x27b70a85U,0x2e1b2138U,0x4d2c6dfcU,0x53380d13U,0x650a7354U,0x766a0abbU,0x81c2c92eU,0x92722c85U,
            0xa2bfe8a1U,0xa81a664bU,0xc24b8b70U,0xc76c51a3U,0xd192e819U,0xd6990624U,0xf40e3585U,0x106aa070U,
            0x19a4c116U,0x1e376c08U,0x2748774cU,0x34b0bcb5U,0x391c0cb3U,0x4ed8aa4aU,0x5b9cca4fU,0x682e6ff3U,
            0x748f82eeU,0x78a5636fU,0x84c87814U,0x8cc70208U,0x90befffaU,0xa4506cebU,0xbef9a3f7U,0xc67178f2U};
        std::array<std::uint32_t, 64> words{};
        for (std::size_t i = 0; i < 16; ++i) words[i] = (std::uint32_t(buffer_[i*4]) << 24) | (std::uint32_t(buffer_[i*4+1]) << 16) | (std::uint32_t(buffer_[i*4+2]) << 8) | buffer_[i*4+3];
        for (std::size_t i = 16; i < words.size(); ++i) {
            const std::uint32_t s0 = rotate_right(words[i-15], 7) ^ rotate_right(words[i-15], 18) ^ (words[i-15] >> 3);
            const std::uint32_t s1 = rotate_right(words[i-2], 17) ^ rotate_right(words[i-2], 19) ^ (words[i-2] >> 10);
            words[i] = words[i-16] + s0 + words[i-7] + s1;
        }
        auto [a,b,c,d,e,f,g,h] = state_;
        for (std::size_t i = 0; i < words.size(); ++i) {
            const std::uint32_t s1 = rotate_right(e, 6) ^ rotate_right(e, 11) ^ rotate_right(e, 25);
            const std::uint32_t choose = (e & f) ^ (~e & g);
            const std::uint32_t t1 = h + s1 + choose + k[i] + words[i];
            const std::uint32_t s0 = rotate_right(a, 2) ^ rotate_right(a, 13) ^ rotate_right(a, 22);
            const std::uint32_t majority = (a & b) ^ (a & c) ^ (b & c);
            const std::uint32_t t2 = s0 + majority;
            h=g; g=f; f=e; e=d+t1; d=c; c=b; b=a; a=t1+t2;
        }
        state_[0]+=a; state_[1]+=b; state_[2]+=c; state_[3]+=d; state_[4]+=e; state_[5]+=f; state_[6]+=g; state_[7]+=h;
    }
    std::array<std::uint32_t, 8> state_;
    std::array<std::uint8_t, 64> buffer_{};
    std::size_t buffer_size_ = 0;
    std::uint64_t total_bytes_ = 0;
};

inline void append_field(Sha256& hash, const std::string& value) {
    hash.update(std::to_string(value.size())); hash.update(":"); hash.update(value); hash.update(";");
}
inline void append_fields(Sha256& hash, const std::vector<std::string>& values) {
    append_field(hash, std::to_string(values.size()));
    for (const std::string& value : values) append_field(hash, value);
}
inline std::string sha256_hex(const std::string& value) { Sha256 hash; hash.update(value); return hash.finish_hex(); }

// Resolved numeric identity is the IEEE-754 payload after normalizing -0.0.
// Non-finite values have no executable Direct OCCT semantic and must fail
// before any cache lookup can turn them into a false hit.
inline std::string canonical_f64(double value) {
    if (!std::isfinite(value)) throw std::invalid_argument("execution identity rejects non-finite resolved number");
    if (value == 0.0) value = 0.0;
    std::uint64_t bits = 0;
    static_assert(sizeof(bits) == sizeof(value));
    std::memcpy(&bits, &value, sizeof(bits));
    std::ostringstream out;
    out << "f64:" << std::hex << std::setw(16) << std::setfill('0') << bits;
    return out.str();
}

inline std::vector<std::string> normalized_unordered_fields(std::vector<std::string> values) {
    std::sort(values.begin(), values.end());
    return values;
}

inline std::string execution_identity(const ExecutionIdentityInput& input) {
    Sha256 hash;
    append_field(hash, "ecky-direct-occt-execution-identity-v1");
    append_field(hash, input.cache_schema); append_field(hash, input.runner_abi);
    append_field(hash, input.runner_binary_digest); append_field(hash, input.occt_runtime);
    append_field(hash, input.tolerance_policy); append_field(hash, input.tessellation_policy);
    append_field(hash, input.op); append_fields(hash, input.resolved_args);
    append_fields(hash, normalized_unordered_fields(input.normalized_keywords));
    append_fields(hash, normalized_unordered_fields(input.selectors));
    append_fields(hash, input.ordered_dependency_identities);
    std::vector<std::string> import_digests; import_digests.reserve(input.import_payloads.size());
    for (const std::string& payload : input.import_payloads) import_digests.push_back("sha256:" + sha256_hex(payload));
    append_fields(hash, import_digests);
    return "sha256:" + hash.finish_hex();
}

class RenderCacheTransaction {
public:
    explicit RenderCacheTransaction(std::filesystem::path root)
        : root_(std::move(root)),
          staging_(root_ / (".render-cache-staging-" +
              std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()))) {}
    ~RenderCacheTransaction() { if (!committed_) abort(); }

    void stage(const std::string& kind, const std::string& identity, const std::string& bytes) {
        if (!supported_kind(kind)) throw std::invalid_argument("unsupported selective cache kind");
        if (identity.empty() || identity.find('/') != std::string::npos || identity.find('\\') != std::string::npos) throw std::invalid_argument("unsafe cache identity");
        const std::filesystem::path dir = staging_ / kind;
        std::filesystem::create_directories(dir);
        const std::filesystem::path artifact = dir / (identity + ".brepbin");
        const std::filesystem::path metadata = dir / (identity + ".meta");
        std::ofstream output(artifact, std::ios::binary); output.write(bytes.data(), static_cast<std::streamsize>(bytes.size()));
        if (!output.good()) throw std::runtime_error("cannot stage cache artifact");
        std::ofstream meta(metadata); meta << "direct-occt-selective-geometry-v4 " << identity << " analyticBrep sha256:" << sha256_hex(bytes) << " " << bytes.size() << '\n';
        if (!meta.good()) throw std::runtime_error("cannot stage cache metadata");
    }

    const std::filesystem::path& staging_root() const { return staging_; }

    void commit() {
        for (const std::string& kind : {"commands", "parts", "part-meshes", "partial-booleans"}) {
            const std::filesystem::path staged = staging_ / kind;
            if (!std::filesystem::is_directory(staged)) continue;
            const std::filesystem::path destination = root_ / kind;
            std::filesystem::create_directories(destination);
            for (const auto& entry : std::filesystem::directory_iterator(staged)) {
                std::filesystem::rename(entry.path(), destination / entry.path().filename());
            }
        }
        std::filesystem::remove_all(staging_); committed_ = true;
    }
    void abort() noexcept { std::error_code ignored; std::filesystem::remove_all(staging_, ignored); }

private:
    static bool supported_kind(const std::string& kind) {
        return kind == "commands" || kind == "parts" || kind == "part-meshes" ||
            kind == "partial-booleans";
    }

    std::filesystem::path root_;
    std::filesystem::path staging_;
    bool committed_ = false;
};

} // namespace ecky
