#[test]
fn models_module_does_not_reexport_transport_contracts() {
    let models_source = include_str!("../src/models.rs");
    assert!(
        !models_source.contains("pub use crate::contracts::*"),
        "crate::models must not re-export crate::contracts; import transport DTOs from crate::contracts explicitly"
    );
}
