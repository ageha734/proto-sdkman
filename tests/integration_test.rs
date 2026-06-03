use proto_pdk_test_utils::*;

mod register_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("sdkman").await;

        let output = plugin
            .register_tool(RegisterToolInput {
                id: "sdkman".into(),
            })
            .await;

        assert_eq!(output.name, "SDKMAN");
        assert_eq!(output.type_of, PluginType::VersionManager);
        assert!(output.self_upgrade_commands.contains(&"selfupdate".into()));
    }
}

mod resolve_version {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn resolves_latest() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("sdkman").await;

        let output = plugin
            .resolve_version(ResolveVersionInput {
                initial: UnresolvedVersionSpec::parse("latest").unwrap(),
                ..Default::default()
            })
            .await;

        assert!(output.version.is_some(), "Should resolve a version");

        let version = output.version.unwrap();
        let version_str = version.to_string();

        // SDKMAN versions follow semver (e.g., 5.18.2)
        assert!(
            version_str.contains('.'),
            "Version should be semver-like: {}",
            version_str
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resolves_stable_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("sdkman").await;

        let output = plugin
            .resolve_version(ResolveVersionInput {
                initial: UnresolvedVersionSpec::Alias("stable".into()),
                ..Default::default()
            })
            .await;

        assert!(output.version.is_some(), "Should resolve stable alias");
    }
}

mod load_versions {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_api() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("sdkman").await;

        let output = plugin
            .load_versions(LoadVersionsInput {
                initial: UnresolvedVersionSpec::parse("latest").unwrap(),
                ..Default::default()
            })
            .await;

        assert!(
            !output.versions.is_empty(),
            "Should have at least one version"
        );
        assert!(output.latest.is_some(), "Should have a latest version");
    }
}

mod locate_executables {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_sdk_executable() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("sdkman").await;

        let output = plugin
            .locate_executables(LocateExecutablesInput {
                ..Default::default()
            })
            .await;

        assert!(
            output.exes.contains_key("sdk"),
            "Should have sdk executable"
        );

        let sdk_config = &output.exes["sdk"];
        assert!(sdk_config.primary, "sdk should be the primary executable");
        assert_eq!(
            sdk_config.parent_exe_name.as_deref(),
            Some("bash"),
            "Should use bash as parent"
        );
    }
}

mod sync_shell_profile {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn syncs_sdkman_dir() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("sdkman").await;

        let output = plugin
            .sync_shell_profile(SyncShellProfileInput {
                ..Default::default()
            })
            .await;

        assert_eq!(output.check_var, "SDKMAN_DIR");
        assert!(!output.skip_sync, "Should not skip sync");
        assert!(
            output.export_vars.is_some(),
            "Should export SDKMAN_DIR variable"
        );

        let vars = output.export_vars.unwrap();
        assert!(vars.contains_key("SDKMAN_DIR"), "Should contain SDKMAN_DIR");
    }
}
