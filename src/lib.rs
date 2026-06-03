use extism_pdk::*;
use proto_pdk::*;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn host_log(input: Json<HostLogInput>);
    fn get_env_var(key: String) -> String;
    fn set_env_var(name: String, value: String);
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
    fn from_virtual_path(input: String) -> String;
    fn to_virtual_path(input: String) -> Json<VirtualPath>;
}

static SDKMAN_API_VERSION_URL: &str =
    "https://api.sdkman.io/2/broker/download/sdkman/version/stable";
static SDKMAN_INSTALL_URL: &str = "https://get.sdkman.io";

/// Register the SDKMAN tool with proto.
#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        name: "SDKMAN".into(),
        type_of: PluginType::VersionManager,
        self_upgrade_commands: vec!["selfupdate".into()],
        ..RegisterToolOutput::default()
    }))
}

/// Resolve version by querying the SDKMAN API for the stable version.
#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    let mut output = ResolveVersionOutput::default();

    // For "latest" or any alias, resolve from the API
    if input.initial.is_latest()
        || matches!(&input.initial, UnresolvedVersionSpec::Alias(alias) if alias == "stable")
    {
        let version_text = fetch_text(SDKMAN_API_VERSION_URL)?;
        let version = version_text.trim();

        host_log!(
            debug,
            "Resolved SDKMAN stable version from API: {}",
            version
        );

        output.version = Some(VersionSpec::parse(version)?);
    }

    Ok(Json(output))
}

/// Load available versions from the SDKMAN API.
/// SDKMAN only publishes a single "stable" version, so we query
/// the API endpoint and return that as the sole available version.
#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let version_text = fetch_text(SDKMAN_API_VERSION_URL)?;
    let version = version_text.trim();
    let spec = VersionSpec::parse(version)?;

    let output = LoadVersionsOutput::from_versions(vec![spec]);

    Ok(Json(output))
}

/// Native install: download and execute the SDKMAN install script.
/// This function runs `curl -s https://get.sdkman.io | bash` on the host.
#[plugin_fn]
pub fn native_install(
    Json(input): Json<NativeInstallInput>,
) -> FnResult<Json<NativeInstallOutput>> {
    let env = get_host_environment()?;

    // SDKMAN only supports Linux and macOS
    if env.os == HostOS::Windows {
        return Ok(Json(NativeInstallOutput {
            installed: false,
            error: Some("SDKMAN is not supported on Windows natively. Use WSL instead.".into()),
            ..NativeInstallOutput::default()
        }));
    }

    // Check for required dependencies
    if !command_exists(&env, "curl") {
        return Ok(Json(NativeInstallOutput {
            installed: false,
            error: Some("curl is required to install SDKMAN but was not found on PATH.".into()),
            ..NativeInstallOutput::default()
        }));
    }

    if !command_exists(&env, "bash") {
        return Ok(Json(NativeInstallOutput {
            installed: false,
            error: Some("bash is required to install SDKMAN but was not found on PATH.".into()),
            ..NativeInstallOutput::default()
        }));
    }

    // Resolve the real install directory path
    let install_dir = real_path!(buf, input.install_dir.any_path());

    host_log!(
        stdout,
        "Installing SDKMAN {} to {}...",
        input.context.version,
        install_dir.display()
    );

    // Set SDKMAN_DIR so the installer places files in the proto-managed directory
    let sdkman_dir = install_dir.to_string_lossy().to_string();
    host_env!("SDKMAN_DIR", sdkman_dir.as_str());

    // Download and execute the install script
    let result = exec_command!(
        inherit,
        "bash",
        ["-c", &format!("curl -s \"{}\" | bash", SDKMAN_INSTALL_URL)]
    );

    if result.exit_code != 0 {
        return Ok(Json(NativeInstallOutput {
            installed: false,
            error: Some(format!(
                "SDKMAN install script failed (exit code {}): {}",
                result.exit_code,
                result.get_output()
            )),
            ..NativeInstallOutput::default()
        }));
    }

    // Verify installation by checking for sdkman-init.sh
    let init_script = install_dir.join("bin").join("sdkman-init.sh");
    let installed = init_script.exists();

    if installed {
        host_log!(stdout, "SDKMAN installed successfully.");
    } else {
        host_log!(
            warn,
            "SDKMAN install script completed but sdkman-init.sh was not found."
        );
    }

    Ok(Json(NativeInstallOutput {
        installed,
        error: if !installed {
            Some("Installation completed but sdkman-init.sh not found.".into())
        } else {
            None
        },
        ..NativeInstallOutput::default()
    }))
}

/// Native uninstall: remove the SDKMAN installation directory.
#[plugin_fn]
pub fn native_uninstall(
    Json(_): Json<NativeUninstallInput>,
) -> FnResult<Json<NativeUninstallOutput>> {
    // Let proto handle the directory removal via the default mechanism
    Ok(Json(NativeUninstallOutput {
        uninstalled: true,
        skip_uninstall: false,
        ..NativeUninstallOutput::default()
    }))
}

/// Locate the SDKMAN executables.
/// The primary executable is `sdk` which is actually a shell function
/// sourced from sdkman-init.sh. We point to the init script as the
/// primary executable using bash as the parent.
#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let primary = ExecutableConfig {
        exe_path: Some("bin/sdkman-init.sh".into()),
        parent_exe_name: Some("bash".into()),
        parent_exe_args: vec!["-c".into()],
        shim_before_args: Some(StringOrVec::String("source \"$EXE\" && sdk".into())),
        primary: true,
        no_bin: true,
        ..ExecutableConfig::default()
    };

    let mut exes = std::collections::HashMap::new();
    exes.insert("sdk".into(), primary);

    Ok(Json(LocateExecutablesOutput {
        exes: exes.into_iter().collect(),
        exes_dirs: vec!["bin".into()],
        ..LocateExecutablesOutput::default()
    }))
}

/// Sync the shell profile to source SDKMAN's init script.
/// This adds the necessary environment variable and source command
/// to the user's shell profile.
#[plugin_fn]
pub fn sync_shell_profile(
    Json(input): Json<SyncShellProfileInput>,
) -> FnResult<Json<SyncShellProfileOutput>> {
    let install_dir = real_path!(buf, input.context.tool_dir.any_path());
    let sdkman_dir = install_dir.to_string_lossy().to_string();

    let mut export_vars = std::collections::HashMap::new();
    export_vars.insert("SDKMAN_DIR".into(), sdkman_dir);

    Ok(Json(SyncShellProfileOutput {
        check_var: "SDKMAN_DIR".into(),
        export_vars: Some(export_vars.into_iter().collect()),
        extend_path: None,
        skip_sync: false,
    }))
}
