use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::direct_occt::{OcctArg, OcctParameterKind, OcctPlan};
use super::direct_occt_runner;
use super::direct_occt_sdk::{DirectOcctSdkLayout, NativeExportOutcome};
use crate::contracts::{
    AppError, AppResult, AuthoringError, AuthoringReason, AuthoringResult, DesignParams, ParamValue,
};
use crate::ecky_core_ir::{CoreParameterValue, CoreProgram};
use crate::models::PathResolver;

/// Compatibility facade for callers that do not own an application resolver.
/// Production rendering uses [`export_core_program_step_stl_with_params_runner_first`].
struct LocalRunnerResolver;

impl PathResolver for LocalRunnerResolver {
    fn app_config_dir(&self) -> PathBuf {
        std::env::temp_dir().join("ecky-direct-occt-config")
    }

    fn app_data_dir(&self) -> PathBuf {
        std::env::temp_dir().join("ecky-direct-occt-data")
    }

    fn resource_path(&self, _path: &str) -> Option<PathBuf> {
        None
    }
}

pub fn export_core_program_step_stl(
    program: &CoreProgram,
    layout: &DirectOcctSdkLayout,
    output_dir: impl AsRef<Path>,
) -> AppResult<NativeExportOutcome> {
    export_core_program_step_stl_with_params(program, &DesignParams::new(), layout, output_dir)
}

pub fn export_core_program_step_stl_with_params(
    program: &CoreProgram,
    parameters: &DesignParams,
    layout: &DirectOcctSdkLayout,
    output_dir: impl AsRef<Path>,
) -> AppResult<NativeExportOutcome> {
    export_core_program_step_stl_with_params_runner_first(
        program,
        parameters,
        layout,
        output_dir,
        &LocalRunnerResolver,
    )
}

pub fn export_core_program_step_stl_with_params_runner_first(
    program: &CoreProgram,
    parameters: &DesignParams,
    _layout: &DirectOcctSdkLayout,
    output_dir: impl AsRef<Path>,
    app: &dyn PathResolver,
) -> AppResult<NativeExportOutcome> {
    let parameters = effective_program_parameters(program, parameters);
    let plan = super::direct_occt::plan_core_program_with_params(program, &parameters)?;
    export_resolved_plan(&plan, &parameters, output_dir, app)
}

pub fn export_plan_step_stl(
    plan: &OcctPlan,
    layout: &DirectOcctSdkLayout,
    output_dir: impl AsRef<Path>,
) -> AppResult<NativeExportOutcome> {
    export_plan_step_stl_with_params(plan, &DesignParams::new(), layout, output_dir)
}

pub fn export_plan_step_stl_with_params(
    plan: &OcctPlan,
    parameters: &DesignParams,
    _layout: &DirectOcctSdkLayout,
    output_dir: impl AsRef<Path>,
) -> AppResult<NativeExportOutcome> {
    export_resolved_plan(plan, parameters, output_dir, &LocalRunnerResolver)
}

fn export_resolved_plan(
    plan: &OcctPlan,
    parameters: &DesignParams,
    output_dir: impl AsRef<Path>,
    app: &dyn PathResolver,
) -> AppResult<NativeExportOutcome> {
    let resolved_plan = resolve_plan_parameters(plan, parameters)?;
    direct_occt_runner::run_plan_step_stl_if_available(&resolved_plan, output_dir, app)?.ok_or_else(
        || AppError::render("Direct OCCT runner unavailable; generated-C++ fallback was removed."),
    )
}

fn backend_validation(message: impl Into<String>) -> AuthoringError {
    AuthoringError::backend(AuthoringReason::Type, message)
}

fn effective_program_parameters(program: &CoreProgram, overrides: &DesignParams) -> DesignParams {
    let mut parameters = DesignParams::new();
    for parameter in &program.parameters {
        parameters.insert(
            parameter.key.clone(),
            match &parameter.default_value {
                CoreParameterValue::Number(value) => ParamValue::Number(*value),
                CoreParameterValue::Boolean(value) => ParamValue::Boolean(*value),
                CoreParameterValue::Text(value)
                | CoreParameterValue::Choice(value)
                | CoreParameterValue::Image(value) => ParamValue::String(value.clone()),
            },
        );
    }
    parameters.extend(overrides.clone());
    parameters
}

fn resolve_plan_parameters(
    plan: &OcctPlan,
    parameters: &DesignParams,
) -> AuthoringResult<OcctPlan> {
    let mut env = BTreeMap::new();
    for parameter in &plan.parameters {
        let value = parameters.get(&parameter.key).ok_or_else(|| {
            backend_validation(format!(
                "Direct OCCT runner missing runtime parameter `{}`.",
                parameter.key
            ))
        })?;
        let valid = matches!(
            (parameter.kind, value),
            (OcctParameterKind::Number, ParamValue::Number(_))
                | (OcctParameterKind::Boolean, ParamValue::Boolean(_))
                | (OcctParameterKind::Text, ParamValue::String(_))
                | (OcctParameterKind::Choice, ParamValue::String(_))
                | (OcctParameterKind::Choice, ParamValue::Number(_))
                | (OcctParameterKind::Image, ParamValue::String(_))
        );
        if !valid {
            return Err(backend_validation(format!(
                "Direct OCCT runner parameter `{}` has incompatible value kind {}.",
                parameter.key,
                value.kind()
            )));
        }
        let arg = match value {
            ParamValue::Number(value) => OcctArg::Number(*value),
            ParamValue::Boolean(value) => OcctArg::Boolean(*value),
            ParamValue::String(value) => OcctArg::Text(value.clone()),
            ParamValue::Null => {
                return Err(backend_validation(
                    "Direct OCCT runner does not support null runtime parameters.",
                ))
            }
        };
        env.insert(parameter.key.clone(), arg);
    }

    let mut resolved = plan.clone();
    for part in &mut resolved.parts {
        for command in &mut part.commands {
            for arg in &mut command.args {
                *arg = resolve_arg(arg, &env)?;
            }
            for keyword in &mut command.keywords {
                *keyword.source_arg_mut() = resolve_arg(keyword.source_arg(), &env)?;
            }
        }
    }
    Ok(resolved)
}

fn resolve_arg(arg: &OcctArg, env: &BTreeMap<String, OcctArg>) -> AuthoringResult<OcctArg> {
    match arg {
        OcctArg::Param(key) => env.get(key).cloned().ok_or_else(|| {
            backend_validation(format!(
                "Direct OCCT runner could not resolve runtime parameter `{key}`."
            ))
        }),
        OcctArg::List(items) => Ok(OcctArg::List(
            items
                .iter()
                .map(|item| resolve_arg(item, env))
                .collect::<AuthoringResult<Vec<_>>>()?,
        )),
        other => Ok(other.clone()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn runner_only_executor_has_no_generated_cpp_emitter() {
        let source = include_str!("direct_occt_executor.rs");
        assert!(!source.contains("emit_plan_export_source"));
        assert!(!source.contains("run_native_export_source"));
    }
}
