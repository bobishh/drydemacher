// Standalone native BDD probe. Build with ECKY_DIRECT_OCCT_RUNNER_TEST=1
// scripts/build_direct_occt_runner.sh; no Rust/Cargo harness involved.
#include <cassert>
#include <filesystem>
#include <fstream>
#include <string>

#include <BOPAlgo_PaveFiller.hxx>

#define main direct_occt_runner_program_main
#include "direct_occt_runner.cpp"
#undef main

namespace fs = std::filesystem;

namespace {

Arg number(double value) {
    Arg arg;
    arg.kind = Arg::Kind::Number;
    arg.number_value = value;
    return arg;
}

Arg ref(std::uint64_t value) {
    Arg arg;
    arg.kind = Arg::Kind::Ref;
    arg.ref_value = value;
    return arg;
}

Command command(std::uint64_t output, std::string op, std::vector<Arg> args) {
    Command result;
    result.output = output;
    result.op = std::move(op);
    result.args = std::move(args);
    return result;
}

Keyword keyword(std::string name, Arg value) {
    Keyword result;
    result.name = std::move(name);
    result.kind = Keyword::Kind::Arg;
    result.value = std::move(value);
    return result;
}

Keyword selector_keyword(std::string name, std::string target_id) {
    Keyword result;
    result.name = std::move(name);
    result.kind = Keyword::Kind::Selector;
    result.selector_payload = SelectorPayload{
        SelectorPayloadType::TargetIds, SelectorKind::Edge, {std::move(target_id)}, {}};
    return result;
}

Part branch(std::string id, double final_x) {
    Part part;
    part.part_id = std::move(id);
    part.label = part.part_id;
    part.root = 4;
    part.commands = {
        command(1, "sphere", {number(5)}),
        command(2, "translate", {number(6), number(0), number(0), ref(1)}),
        command(3, "union", {ref(1), ref(2)}),
        command(4, "translate", {number(final_x), number(0), number(0), ref(3)}),
    };
    return part;
}

Plan two_branch_plan(double final_x = 0) {
    Plan plan;
    plan.schema_version = 1;
    plan.plan_id = "native-dag-test";
    plan.parts = {branch("left", final_x), branch("right", final_x + 30)};
    return plan;
}

Part same_semantics_with_new_transient_ids() {
    Part part;
    part.part_id = "replanned-part-id";
    part.label = "new source label";
    part.root = 104;
    part.commands = {
        command(101, "sphere", {number(5)}),
        command(102, "translate", {number(6), number(0), number(0), ref(101)}),
        command(103, "union", {ref(101), ref(102)}),
        command(104, "translate", {number(0), number(0), number(0), ref(103)}),
    };
    return part;
}

double signed_volume(const std::vector<ShapeRecord>& parts) {
    double total = 0;
    for (const ShapeRecord& part : parts) {
        GProp_GProps props;
        BRepGProp::VolumeProperties(part.shape, props);
        total += props.Mass();
    }
    return total;
}

std::array<double, 6> bounds(const std::vector<ShapeRecord>& parts) {
    Bnd_Box box;
    for (const ShapeRecord& part : parts) BRepBndLib::Add(part.shape, box);
    double xmin, ymin, zmin, xmax, ymax, zmax;
    box.Get(xmin, ymin, zmin, xmax, ymax, zmax);
    return {xmin, ymin, zmin, xmax, ymax, zmax};
}

std::string fingerprint(const TopoDS_Shape& shape) {
    std::ostringstream bytes;
    BinTools::Write(shape, bytes, Standard_False, Standard_False, BinTools_FormatVersion_CURRENT);
    return bytes.str();
}

void given_independent_ready_nodes_when_scheduled_then_order_release_budget_and_parity_hold() {
    const Plan plan = two_branch_plan();
    ExecutionContext serial;
    serial.worker_budget = 1;
    const auto serial_parts = evaluate_plan(plan, std::nullopt, nullptr, serial);
    assert(serial.peak_dag_concurrency == 1);
    assert(serial.released_slot_count >= 2);

    ExecutionContext parallel;
    parallel.worker_budget = 2;
    const auto parallel_parts = evaluate_plan(plan, std::nullopt, nullptr, parallel);
    assert(parallel.peak_dag_concurrency == 2);
    assert(parallel.peak_dag_concurrency <= parallel.worker_budget);
    assert(parallel.part_executed_command_ids.at("left") ==
           std::vector<std::string>({"left:1", "left:2", "left:3", "left:4"}));
    assert(parallel.part_executed_command_ids.at("right") ==
           std::vector<std::string>({"right:1", "right:2", "right:3", "right:4"}));
    assert(bounds(serial_parts) == bounds(parallel_parts));
    assert(std::abs(signed_volume(serial_parts) - signed_volume(parallel_parts)) < 1.0e-7);

    Command safe = command(1, "sphere", {number(1)});
    Command barrier = command(2, "union", {ref(1), ref(1)});
    assert(command_has_proven_immutable_inputs(safe));
    assert(!command_has_proven_immutable_inputs(barrier));
}

void given_resolved_runner_plan_when_identity_is_built_then_only_semantics_and_native_runtime_enter() {
    ExecutionContext context;
    context.runner_binary_digest = "sha256:actual-runner-binary";

    const Part original = branch("original-part-id", 0);
    const Part replanned = same_semantics_with_new_transient_ids();
    assert(part_cache_key(original, context) == part_cache_key(replanned, context));

    Command canonical = command(7, "fillet", {number(-0.0), ref(1)});
    canonical.keywords = {
        keyword("radius", number(1.5)),
        selector_keyword("edges", "body:edge:top"),
    };
    const std::map<std::uint64_t, std::string> dependencies = {{1, "sha256:input"}};
    const std::string identity = command_cache_key(canonical, dependencies, context);

    Command reordered = canonical;
    reordered.output = 999;
    std::reverse(reordered.keywords.begin(), reordered.keywords.end());
    reordered.args[0] = number(0.0);
    assert(identity == command_cache_key(reordered, dependencies, context));

    Command changed_selector = canonical;
    changed_selector.keywords[1] = selector_keyword("edges", "body:edge:bottom");
    assert(identity != command_cache_key(changed_selector, dependencies, context));

    const std::map<std::uint64_t, std::string> changed_dependency = {{1, "sha256:changed-input"}};
    assert(identity != command_cache_key(canonical, changed_dependency, context));

    Command grouped_union = command(50, "union", {ref(1), ref(2), ref(3), ref(4)});
    const PartialBooleanGroupPlan right_group{"lid", 50, "union", {2, 3}, 1, 1};
    const std::map<std::uint64_t, std::string> group_dependencies = {
        {1, "left-a"}, {2, "left-b"}, {3, "right-a"}, {4, "right-b"}};
    const std::string group_identity = partial_boolean_group_cache_key(
        grouped_union, right_group, group_dependencies, context);
    grouped_union.output = 999;
    auto changed_left_group = group_dependencies;
    changed_left_group[1] = "changed-left-a";
    assert(group_identity == partial_boolean_group_cache_key(
        grouped_union, right_group, changed_left_group, context));
    auto changed_right_group = group_dependencies;
    changed_right_group[3] = "changed-right-a";
    assert(group_identity != partial_boolean_group_cache_key(
        grouped_union, right_group, changed_right_group, context));

    ExecutionContext changed_runtime;
    changed_runtime.runner_binary_digest = "sha256:changed-runner-binary";
    assert(identity != command_cache_key(canonical, dependencies, changed_runtime));
}

void given_bad_ready_node_when_evaluated_then_failure_stops_publication() {
    Plan bad = two_branch_plan();
    bad.parts[0].commands.back() = command(4, "unsupported-native-op", {ref(3)});
    ExecutionContext context;
    context.worker_budget = 2;
    bool failed = false;
    try {
        (void)evaluate_plan(bad, std::nullopt, nullptr, context);
    } catch (const EvalError&) {
        failed = true;
    }
    assert(failed);
}

void given_singleton_boolean_batch_when_boolean_is_built_then_occt_pool_is_enabled() {
    ExecutionContext singleton;
    singleton.active_dag_nodes = 1;
    assert(boolean_runs_parallel(singleton));

    ExecutionContext competing;
    competing.active_dag_nodes = 2;
    assert(!boolean_runs_parallel(competing));
}

void given_boolean_commands_when_plan_is_evaluated_then_context_records_compact_timings() {
    Plan plan = two_branch_plan();
    ExecutionContext context;
    context.worker_budget = 1;
    (void)evaluate_plan(plan, std::nullopt, nullptr, context);
    assert(!context.command_timing_evidence.empty());
    assert(std::any_of(
        context.command_timing_evidence.begin(), context.command_timing_evidence.end(),
        [](const CommandTimingEvidence& evidence) {
            return evidence.op == "union" && evidence.elapsed_ms >= 0;
        }
    ));
}

void given_four_shared_boolean_operands_when_fused_privately_then_inputs_and_result_topology_hold() {
    const TopoDS_Shape first = BRepPrimAPI_MakeBox(10, 10, 10).Shape();
    const TopoDS_Shape second = translate_shape(first, 6, 0, 0);
    const TopoDS_Shape third = translate_shape(first, 0, 6, 0);
    const TopoDS_Shape fourth = translate_shape(first, 6, 6, 0);
    const std::vector<TopoDS_Shape> operands = {first, second, third, fourth};
    std::vector<std::string> before;
    for (const TopoDS_Shape& operand : operands) before.push_back(fingerprint(operand));

    ExecutionContext context;
    context.active_dag_nodes = 1;
    const TopoDS_Shape result = checked_boolean_shapes<BRepAlgoAPI_Fuse>(
        {private_boolean_operand(operands[0])},
        {private_boolean_operand(operands[1]), private_boolean_operand(operands[2]),
         private_boolean_operand(operands[3])},
        "union", context);

    assert(BRepCheck_Analyzer(result).IsValid());
    GProp_GProps props;
    BRepGProp::VolumeProperties(result, props);
    assert(std::abs(props.Mass() - 2560.0) < 1.0e-7);
    const std::vector<ShapeRecord> result_part = {{"result", "result", ShapeRecord::Kind::Shape, result, {}}};
    const std::array<double, 6> expected_bounds = {0, 0, 0, 16, 16, 10};
    const std::array<double, 6> actual_bounds = bounds(result_part);
    for (std::size_t index = 0; index < actual_bounds.size(); ++index) {
        assert(std::abs(actual_bounds[index] - expected_bounds[index]) < 1.0e-6);
    }
    std::size_t solids = 0;
    for (TopExp_Explorer explorer(result, TopAbs_SOLID); explorer.More(); explorer.Next()) ++solids;
    assert(solids == 1);
    for (std::size_t index = 0; index < operands.size(); ++index) {
        assert(fingerprint(operands[index]) == before[index]);
    }
    assert(boolean_inputs_are_valid_solids(operands));
}

void given_four_operands_when_one_pave_filler_materializes_full_and_subset_fuses_then_parity_holds() {
    const TopoDS_Shape first = BRepPrimAPI_MakeBox(10, 10, 10).Shape();
    const TopoDS_Shape second = translate_shape(first, 6, 0, 0);
    const TopoDS_Shape third = translate_shape(first, 0, 6, 0);
    const TopoDS_Shape fourth = translate_shape(first, 6, 6, 0);
    const std::vector<TopoDS_Shape> operands = {first, second, third, fourth};
    std::vector<std::string> before;
    TopTools_ListOfShape prepared;
    std::vector<TopoDS_Shape> prepared_shapes;
    for (const TopoDS_Shape& operand : operands) {
        before.push_back(fingerprint(operand));
        prepared_shapes.push_back(private_boolean_operand(operand));
        prepared.Append(prepared_shapes.back());
    }

    BOPAlgo_PaveFiller filler;
    filler.SetArguments(prepared);
    filler.SetFuzzyValue(1.0e-5);
    filler.SetRunParallel(Standard_False);
    std::uint32_t intersection_perform_count = 0;
    filler.Perform();
    ++intersection_perform_count;
    assert(!filler.HasErrors());

    const auto materialize_fuse = [&filler](const TopoDS_Shape& object,
                                            const TopTools_ListOfShape& tools) {
        BOPAlgo_Builder builder;
        builder.SetArguments(filler.Arguments());
        builder.PerformWithFiller(filler);
        assert(!builder.HasErrors());
        TopTools_ListOfShape objects;
        objects.Append(object);
        builder.BuildBOP(objects, tools, BOPAlgo_FUSE, Message_ProgressRange());
        assert(!builder.HasErrors());
        return builder.Shape();
    };

    TopTools_ListOfShape full_tools;
    full_tools.Append(prepared_shapes[1]);
    full_tools.Append(prepared_shapes[2]);
    full_tools.Append(prepared_shapes[3]);
    const TopoDS_Shape full = materialize_fuse(prepared_shapes[0], full_tools);

    TopTools_ListOfShape subset_tools;
    subset_tools.Append(prepared_shapes[3]);
    const TopoDS_Shape subset = materialize_fuse(prepared_shapes[2], subset_tools);
    assert(BRepCheck_Analyzer(subset).IsValid());

    ExecutionContext baseline_context;
    const TopoDS_Shape baseline = fuse_shapes(operands, baseline_context);
    assert(BRepCheck_Analyzer(full).IsValid());
    const std::vector<ShapeRecord> full_part = {{"full", "full", ShapeRecord::Kind::Shape, full, {}}};
    const std::vector<ShapeRecord> baseline_part = {{"baseline", "baseline", ShapeRecord::Kind::Shape, baseline, {}}};
    assert(bounds(full_part) == bounds(baseline_part));
    assert(std::abs(signed_volume(full_part) - signed_volume(baseline_part)) < 1.0e-7);
    ExecutionContext subset_baseline_context;
    const TopoDS_Shape subset_baseline = fuse_shapes(
        std::vector<TopoDS_Shape>{operands[2], operands[3]}, subset_baseline_context);
    const std::vector<ShapeRecord> subset_part = {{"subset", "subset", ShapeRecord::Kind::Shape, subset, {}}};
    const std::vector<ShapeRecord> subset_baseline_part = {
        {"subset-baseline", "subset-baseline", ShapeRecord::Kind::Shape, subset_baseline, {}}};
    assert(bounds(subset_part) == bounds(subset_baseline_part));
    assert(std::abs(signed_volume(subset_part) - signed_volume(subset_baseline_part)) < 1.0e-7);
    assert(intersection_perform_count == 1);
    for (std::size_t index = 0; index < operands.size(); ++index) {
        assert(fingerprint(operands[index]) == before[index]);
    }
}

void given_level_b_cache_when_root_changes_then_hit_prunes_dependencies_and_corruption_recomputes() {
    const fs::path root = fs::temp_directory_path() / "ecky-direct-occt-native-cache-test";
    fs::remove_all(root);
    const Plan cold = [] {
        Plan plan = two_branch_plan(0);
        plan.parts.resize(1);
        return plan;
    }();
    ExecutionContext cold_context;
    cold_context.worker_budget = 2;
    cold_context.runner_binary_digest = "sha256:native-test";
    {
        ecky::RenderCacheTransaction transaction(root);
        (void)evaluate_plan(cold, root, &transaction, cold_context);
        transaction.commit();
    }
    fs::remove_all(root / "parts");

    Plan changed = cold;
    changed.parts[0].commands.back().args[0] = number(11);
    ExecutionContext hit_context;
    hit_context.worker_budget = 2;
    hit_context.runner_binary_digest = "sha256:native-test";
    {
        ecky::RenderCacheTransaction transaction(root);
        (void)evaluate_plan(changed, root, &transaction, hit_context);
        transaction.commit();
    }
    assert(hit_context.command_cache_evidence.size() == 1);
    assert(hit_context.command_cache_evidence.front().cache_hit);
    assert(hit_context.part_executed_command_ids.at("left") ==
           std::vector<std::string>({"left:4"}));

    fs::remove_all(root / "parts");
    for (const fs::directory_entry& entry : fs::directory_iterator(root / "commands")) {
        if (entry.path().extension() == ".brepbin") {
            std::ofstream corrupt(entry.path(), std::ios::binary | std::ios::trunc);
            corrupt << "corrupt";
        }
    }
    ExecutionContext corrupt_context;
    corrupt_context.worker_budget = 2;
    corrupt_context.runner_binary_digest = "sha256:native-test";
    {
        ecky::RenderCacheTransaction transaction(root);
        (void)evaluate_plan(changed, root, &transaction, corrupt_context);
        transaction.commit();
    }
    assert(corrupt_context.cache_rejection_count >= 1);
    assert(corrupt_context.part_executed_command_ids.at("left").size() == 4);
    fs::remove_all(root);
}

void given_localized_edit_when_other_part_cache_is_clean_then_zero_clean_commands_and_topology_hold() {
    const fs::path root = fs::temp_directory_path() / "ecky-direct-occt-native-localized-test";
    fs::remove_all(root);
    const Plan cold = two_branch_plan();
    ExecutionContext cold_context;
    cold_context.worker_budget = 2;
    cold_context.runner_binary_digest = "sha256:native-test";
    std::vector<ShapeRecord> cold_parts;
    {
        ecky::RenderCacheTransaction transaction(root);
        cold_parts = evaluate_plan(cold, root, &transaction, cold_context);
        transaction.commit();
    }
    Plan changed = cold;
    changed.parts[1].commands.back().args[0] = number(41);
    ExecutionContext warm_context;
    warm_context.worker_budget = 2;
    warm_context.runner_binary_digest = "sha256:native-test";
    std::vector<ShapeRecord> warm_parts;
    {
        ecky::RenderCacheTransaction transaction(root);
        warm_parts = evaluate_plan(changed, root, &transaction, warm_context);
        transaction.commit();
    }
    assert(warm_context.part_cache_hits.at("left"));
    assert(!warm_context.part_cache_hits.at("right"));
    assert(warm_context.part_executed_commands.find("left") ==
           warm_context.part_executed_commands.end());
    // Right root changed, but its admitted Boolean remains a Level B hit.
    assert(warm_context.part_executed_commands.at("right") == 1);
    const std::vector<ShapeRecord> cold_left = {cold_parts.front()};
    const std::vector<ShapeRecord> warm_left = {warm_parts.front()};
    assert(bounds(cold_left) == bounds(warm_left));
    assert(std::abs(signed_volume(cold_left) - signed_volume(warm_left)) < 1.0e-7);
    fs::remove_all(root);
}

void given_render_failure_when_cache_staged_then_transaction_publishes_zero_entries() {
    const fs::path root = fs::temp_directory_path() / "ecky-direct-occt-native-cache-abort-test";
    fs::remove_all(root);
    Plan bad = two_branch_plan();
    bad.parts.resize(1);
    bad.parts[0].commands.push_back(command(5, "unsupported-native-op", {ref(4)}));
    bad.parts[0].root = 5;
    ExecutionContext context;
    context.worker_budget = 2;
    context.runner_binary_digest = "sha256:native-test";
    bool failed = false;
    try {
        ecky::RenderCacheTransaction transaction(root);
        (void)evaluate_plan(bad, root, &transaction, context);
        transaction.commit();
    } catch (const EvalError&) {
        failed = true;
    }
    assert(failed);
    assert(!fs::exists(root / "commands"));
    assert(!fs::exists(root / "parts"));
    fs::remove_all(root);
}

Plan partial_union_plan(double first_width) {
    Plan plan;
    plan.schema_version = 1;
    plan.plan_id = "partial-union-test";
    Part part;
    part.part_id = "lid";
    part.label = "lid";
    part.root = 9;
    part.commands = {
        command(1, "box", {number(first_width), number(10), number(10)}),
        command(2, "box", {number(10), number(10), number(10)}),
        command(3, "box", {number(10), number(10), number(10)}),
        command(4, "box", {number(10), number(10), number(10)}),
        command(5, "translate", {number(6), number(0), number(0), ref(2)}),
        command(6, "translate", {number(0), number(6), number(0), ref(3)}),
        command(7, "translate", {number(6), number(6), number(0), ref(4)}),
        command(9, "union", {ref(1), ref(5), ref(6), ref(7)}),
    };
    plan.parts = {part};
    plan.partial_boolean_groups = {
        {"lid", 9, "union", {0, 1}, 0, 1},
        {"lid", 9, "union", {2, 3}, 1, 1},
    };
    return plan;
}

void given_partial_union_cache_when_one_half_changes_then_other_half_hits_without_four_way_fill() {
    const fs::path root = fs::temp_directory_path() / "ecky-direct-occt-partial-union-cache-test";
    fs::remove_all(root);
    ExecutionContext cold_context;
    cold_context.worker_budget = 1;
    cold_context.runner_binary_digest = "sha256:native-test";
    {
        ecky::RenderCacheTransaction transaction(root);
        (void)evaluate_plan(partial_union_plan(10), root, &transaction, cold_context);
        transaction.commit();
    }
    assert(cold_context.partial_boolean_cache_miss_count == 2);
    assert(cold_context.partial_boolean_cache_write_count == 2);
    assert(cold_context.four_way_intersection_count == 1);
    fs::remove_all(root / "parts");
    fs::remove_all(root / "commands");

    ExecutionContext warm_context;
    warm_context.worker_budget = 1;
    warm_context.runner_binary_digest = "sha256:native-test";
    {
        ecky::RenderCacheTransaction transaction(root);
        (void)evaluate_plan(partial_union_plan(11), root, &transaction, warm_context);
        transaction.commit();
    }
    assert(warm_context.partial_boolean_cache_hit_count == 1);
    assert(warm_context.partial_boolean_cache_miss_count == 1);
    assert(warm_context.partial_boolean_cache_write_count == 1);
    assert(warm_context.four_way_intersection_count == 0);
    fs::remove_all(root);
}

void given_partial_union_side_outputs_when_render_fails_then_none_publish_and_corruption_recomputes() {
    const fs::path failed_root = fs::temp_directory_path() / "ecky-direct-occt-partial-union-abort-test";
    fs::remove_all(failed_root);
    Plan bad = partial_union_plan(10);
    bad.parts[0].commands.push_back(command(10, "unsupported-native-op", {ref(9)}));
    bad.parts[0].root = 10;
    ExecutionContext failed_context;
    failed_context.worker_budget = 1;
    failed_context.runner_binary_digest = "sha256:native-test";
    bool failed = false;
    try {
        ecky::RenderCacheTransaction transaction(failed_root);
        (void)evaluate_plan(bad, failed_root, &transaction, failed_context);
        transaction.commit();
    } catch (const EvalError&) {
        failed = true;
    }
    assert(failed);
    assert(!fs::exists(failed_root / "partial-booleans"));
    fs::remove_all(failed_root);

    const fs::path corrupt_root = fs::temp_directory_path() / "ecky-direct-occt-partial-union-corrupt-test";
    fs::remove_all(corrupt_root);
    ExecutionContext cold_context;
    cold_context.worker_budget = 1;
    cold_context.runner_binary_digest = "sha256:native-test";
    {
        ecky::RenderCacheTransaction transaction(corrupt_root);
        (void)evaluate_plan(partial_union_plan(10), corrupt_root, &transaction, cold_context);
        transaction.commit();
    }
    fs::remove_all(corrupt_root / "parts");
    fs::remove_all(corrupt_root / "commands");
    const fs::path partial_dir = corrupt_root / "partial-booleans";
    const fs::path artifact = std::find_if(
        fs::directory_iterator(partial_dir), fs::directory_iterator(),
        [](const fs::directory_entry& entry) { return entry.path().extension() == ".brepbin"; })
        ->path();
    std::ofstream(artifact, std::ios::binary | std::ios::trunc) << "corrupt";

    ExecutionContext repair_context;
    repair_context.worker_budget = 1;
    repair_context.runner_binary_digest = "sha256:native-test";
    {
        ecky::RenderCacheTransaction transaction(corrupt_root);
        (void)evaluate_plan(partial_union_plan(10), corrupt_root, &transaction, repair_context);
        transaction.commit();
    }
    assert(repair_context.cache_rejection_count >= 1);
    assert(repair_context.partial_boolean_cache_hit_count == 1);
    assert(repair_context.partial_boolean_cache_miss_count == 1);
    assert(repair_context.partial_boolean_cache_write_count == 1);
    assert(repair_context.four_way_intersection_count == 0);
    fs::remove_all(corrupt_root);
}

}  // namespace

int main() {
    given_independent_ready_nodes_when_scheduled_then_order_release_budget_and_parity_hold();
    given_resolved_runner_plan_when_identity_is_built_then_only_semantics_and_native_runtime_enter();
    given_bad_ready_node_when_evaluated_then_failure_stops_publication();
    given_singleton_boolean_batch_when_boolean_is_built_then_occt_pool_is_enabled();
    given_boolean_commands_when_plan_is_evaluated_then_context_records_compact_timings();
    given_four_shared_boolean_operands_when_fused_privately_then_inputs_and_result_topology_hold();
    given_four_operands_when_one_pave_filler_materializes_full_and_subset_fuses_then_parity_holds();
    given_level_b_cache_when_root_changes_then_hit_prunes_dependencies_and_corruption_recomputes();
    given_localized_edit_when_other_part_cache_is_clean_then_zero_clean_commands_and_topology_hold();
    given_render_failure_when_cache_staged_then_transaction_publishes_zero_entries();
    given_partial_union_cache_when_one_half_changes_then_other_half_hits_without_four_way_fill();
    given_partial_union_side_outputs_when_render_fails_then_none_publish_and_corruption_recomputes();
}
