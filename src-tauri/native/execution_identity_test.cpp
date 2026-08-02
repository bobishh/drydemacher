#include "execution_identity.hpp"

#include <cassert>
#include <cmath>
#include <filesystem>
#include <fstream>

namespace fs = std::filesystem;

using ecky::ExecutionIdentityInput;

void given_sha256_when_canonical_identity_hashes_then_digest_is_standard_sha256() {
    assert(ecky::sha256_hex("abc") ==
           "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
}

void given_resolved_float_when_canonicalized_then_negative_zero_is_normalized_and_non_finite_is_rejected() {
    assert(ecky::canonical_f64(0.0) == "f64:0000000000000000");
    assert(ecky::canonical_f64(-0.0) == ecky::canonical_f64(0.0));
    assert(ecky::canonical_f64(1.5) == "f64:3ff8000000000000");

    bool rejected = false;
    try {
        (void)ecky::canonical_f64(std::numeric_limits<double>::infinity());
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);
}

ExecutionIdentityInput fixture() {
    ExecutionIdentityInput input;
    input.cache_schema = "selective-brep-v3";
    input.runner_abi = "runner-abi-2";
    input.runner_binary_digest = "sha256:runner";
    input.occt_runtime = "occt-7.8.1";
    input.tolerance_policy = "fuzzy=1e-5";
    input.tessellation_policy = "linear=0.04;angular=adaptive";
    input.op = "difference";
    input.resolved_args = {"shape:base", "shape:cut"};
    input.normalized_keywords = {"keyword:keep-tools:0", "keyword:fuzzy:f64:3ee4f8b588e368f1"};
    input.selectors = {"face:top"};
    input.ordered_dependency_identities = {"sha256:base", "sha256:cut"};
    input.import_payloads = {"STEP bytes version one"};
    return input;
}

void given_resolved_keywords_and_selectors_when_order_is_nonsemantic_then_identity_is_normalized() {
    const ExecutionIdentityInput base = fixture();
    const std::string identity = ecky::execution_identity(base);

    auto changed = base;
    std::reverse(changed.normalized_keywords.begin(), changed.normalized_keywords.end());
    std::reverse(changed.selectors.begin(), changed.selectors.end());
    assert(identity == ecky::execution_identity(changed));

    changed = base;
    changed.normalized_keywords[0] = "keyword:keep-tools:1";
    assert(identity != ecky::execution_identity(changed));
    changed = base;
    changed.selectors = {"face:bottom"};
    assert(identity != ecky::execution_identity(changed));
}

void given_ordered_dependencies_when_operand_order_changes_then_identity_changes() {
    const ExecutionIdentityInput base = fixture();
    auto reversed = base;
    std::reverse(reversed.ordered_dependency_identities.begin(), reversed.ordered_dependency_identities.end());
    assert(ecky::execution_identity(base) != ecky::execution_identity(reversed));
}

void given_transient_source_slot_node_and_label_metadata_when_semantics_match_then_identity_is_stable() {
    const ExecutionIdentityInput base = fixture();
    const ExecutionIdentityInput replanned_with_new_source_slots_nodes_and_labels = fixture();
    // This type deliberately has no source span, source text, raw slot/node id,
    // or label field. Callers must first lower those transient values to the
    // resolved args and dependency identities above.
    assert(ecky::execution_identity(base) ==
           ecky::execution_identity(replanned_with_new_source_slots_nodes_and_labels));
}

void given_resolved_identity_when_import_runtime_or_policy_changes_then_identity_invalidates() {
    const ExecutionIdentityInput base = fixture();
    const std::string identity = ecky::execution_identity(base);
    auto changed = base;
    changed.import_payloads = {"STEP bytes version two"};
    assert(identity != ecky::execution_identity(changed));
    changed = base;
    changed.occt_runtime = "occt-7.8.2";
    assert(identity != ecky::execution_identity(changed));
    changed = base;
    changed.cache_schema = "selective-brep-v4";
    assert(identity != ecky::execution_identity(changed));
    changed = base;
    changed.runner_abi = "sha256:changed-abi";
    assert(identity != ecky::execution_identity(changed));
    changed = base;
    changed.runner_binary_digest = "sha256:changed-binary";
    assert(identity != ecky::execution_identity(changed));
    changed = base;
    changed.tolerance_policy = "fuzzy=1e-4";
    assert(identity != ecky::execution_identity(changed));
    changed = base;
    changed.tessellation_policy = "linear=0.02;angular=adaptive";
    assert(identity != ecky::execution_identity(changed));
}

void given_unrelated_parameter_when_identity_is_resolved_then_clean_identity_is_stable() {
    const ExecutionIdentityInput clean_part_before = fixture();
    const ExecutionIdentityInput clean_part_after = fixture();
    assert(ecky::execution_identity(clean_part_before) == ecky::execution_identity(clean_part_after));
}

void given_structural_repeat_count_or_dependency_change_when_resolved_graph_changes_then_only_affected_identity_invalidates() {
    const ExecutionIdentityInput clean_part = fixture();
    const ExecutionIdentityInput affected_part = fixture();
    const std::string clean_before = ecky::execution_identity(clean_part);
    const std::string affected_before = ecky::execution_identity(affected_part);

    auto repeat_changed = affected_part;
    repeat_changed.resolved_args.push_back("repeat-count:4");
    assert(clean_before == ecky::execution_identity(clean_part));
    assert(affected_before != ecky::execution_identity(repeat_changed));

    auto dependency_changed = affected_part;
    dependency_changed.ordered_dependency_identities[1] = "sha256:changed-cut";
    assert(clean_before == ecky::execution_identity(clean_part));
    assert(affected_before != ecky::execution_identity(dependency_changed));
}

void given_sibling_failure_when_render_transaction_aborts_then_no_entry_is_published() {
    const fs::path root = fs::temp_directory_path() / "ecky-execution-identity-transaction-test";
    fs::remove_all(root);
    {
        ecky::RenderCacheTransaction transaction(root);
        transaction.stage("commands", "sha256:command-a", "complete-command-a");
        transaction.stage("parts", "sha256:part-a", "complete-part-a");
        transaction.abort();
    }
    assert(!fs::exists(root / "commands" / "sha256:command-a.brepbin"));
    assert(!fs::exists(root / "parts" / "sha256:part-a.brepbin"));
    fs::remove_all(root);
}

void given_successful_render_when_transaction_commits_then_entries_publish_together() {
    const fs::path root = fs::temp_directory_path() / "ecky-execution-identity-commit-test";
    fs::remove_all(root);
    {
        ecky::RenderCacheTransaction transaction(root);
        transaction.stage("commands", "sha256:command-a", "complete-command-a");
        transaction.stage("parts", "sha256:part-a", "complete-part-a");
        transaction.commit();
    }
    assert(fs::exists(root / "commands" / "sha256:command-a.brepbin"));
    assert(fs::exists(root / "parts" / "sha256:part-a.brepbin"));
    fs::remove_all(root);
}

void given_successful_render_when_part_mesh_is_staged_then_it_publishes_with_brep_entries() {
    const fs::path root = fs::temp_directory_path() / "ecky-execution-identity-part-mesh-transaction-test";
    fs::remove_all(root);
    {
        ecky::RenderCacheTransaction transaction(root);
        transaction.stage("commands", "sha256:command-a", "complete-command-a");
        transaction.stage("parts", "sha256:part-a", "complete-part-a");
        transaction.stage("part-meshes", "sha256:mesh-a", "complete-mesh-a");
        transaction.commit();
    }
    assert(fs::exists(root / "part-meshes" / "sha256:mesh-a.brepbin"));
    fs::remove_all(root);
}

int main() {
    given_sha256_when_canonical_identity_hashes_then_digest_is_standard_sha256();
    given_resolved_float_when_canonicalized_then_negative_zero_is_normalized_and_non_finite_is_rejected();
    given_resolved_keywords_and_selectors_when_order_is_nonsemantic_then_identity_is_normalized();
    given_ordered_dependencies_when_operand_order_changes_then_identity_changes();
    given_transient_source_slot_node_and_label_metadata_when_semantics_match_then_identity_is_stable();
    given_resolved_identity_when_import_runtime_or_policy_changes_then_identity_invalidates();
    given_unrelated_parameter_when_identity_is_resolved_then_clean_identity_is_stable();
    given_structural_repeat_count_or_dependency_change_when_resolved_graph_changes_then_only_affected_identity_invalidates();
    given_sibling_failure_when_render_transaction_aborts_then_no_entry_is_published();
    given_successful_render_when_transaction_commits_then_entries_publish_together();
    given_successful_render_when_part_mesh_is_staged_then_it_publishes_with_brep_entries();
}
