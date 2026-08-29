use ecky_cad_lib::services::provider_executable::resolve_provider_executable_from_sources;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_home(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("ecky-{label}-{}-{nonce}", std::process::id()))
}

fn write_executable(path: &Path) {
    fs::create_dir_all(path.parent().expect("parent")).expect("create executable parent");
    fs::write(path, "#!/bin/sh\nexit 0\n").expect("write executable");
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("chmod executable");
    }
}

#[test]
fn packaged_app_resolves_provider_from_login_shell_path() {
    let home = temp_home("provider-login-path");
    let provider_bin = home.join("shell-bin/ecky-test-provider");
    write_executable(&provider_bin);

    let resolved = resolve_provider_executable_from_sources(
        "ecky-test-provider",
        Some("/usr/bin:/bin:/usr/sbin:/sbin"),
        Some(home.join("shell-bin").to_string_lossy().as_ref()),
        Some(&home),
    )
    .expect("provider should resolve through login shell PATH");

    assert_eq!(resolved, provider_bin);
    fs::remove_dir_all(home).expect("cleanup temp home");
}

#[test]
fn packaged_app_resolves_provider_from_user_local_bin() {
    let home = temp_home("provider-local-bin");
    let provider_bin = home.join(".local/bin/ecky-test-provider");
    write_executable(&provider_bin);

    let resolved = resolve_provider_executable_from_sources(
        "ecky-test-provider",
        Some("/usr/bin:/bin:/usr/sbin:/sbin"),
        None,
        Some(&home),
    )
    .expect("provider should resolve through ~/.local/bin");

    assert_eq!(resolved, provider_bin);
    fs::remove_dir_all(home).expect("cleanup temp home");
}
