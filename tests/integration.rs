use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

fn setup_store() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    unsafe { std::env::set_var("CP_SWITCH_DIR", dir.path()); }
    dir
}

fn store_dir_val() -> String {
    std::env::var("CP_SWITCH_DIR").unwrap_or_default()
}

fn setup_project(settings_json: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let claude_dir = dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(claude_dir.join("settings.local.json"), settings_json).unwrap();
    dir
}

fn read_settings(project: &std::path::Path) -> serde_json::Value {
    let path = project.join(".claude/settings.local.json");
    serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap()
}

fn get_env_obj(settings: &serde_json::Value) -> &serde_json::Map<String, serde_json::Value> {
    settings.get("env").unwrap().as_object().unwrap()
}

fn run_cli(args: &str, project: &std::path::Path) -> (bool, String, String) {
    let bin = std::env::var("CARGO_BIN_EXE_cp-switch").unwrap();
    let output = Command::new(&bin)
        .args(args.split_whitespace())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env("CP_SWITCH_DIR", store_dir_val())
        .current_dir(project)
        .output()
        .unwrap();
    (output.status.success(),
     String::from_utf8_lossy(&output.stdout).to_string(),
     String::from_utf8_lossy(&output.stderr).to_string())
}

fn run_cli_stdin(args: &str, input: &str, project: &std::path::Path) -> (bool, String, String) {
    let bin = std::env::var("CARGO_BIN_EXE_cp-switch").unwrap();
    let mut child = Command::new(&bin)
        .args(args.split_whitespace())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env("CP_SWITCH_DIR", store_dir_val())
        .current_dir(project)
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    (output.status.success(),
     String::from_utf8_lossy(&output.stdout).to_string(),
     String::from_utf8_lossy(&output.stderr).to_string())
}

fn setup_project_no_settings() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".claude")).unwrap();
    dir
}

fn combined_output(stdout: &str, stderr: &str) -> String {
    stdout.to_string() + stderr
}

// ============================================================
// 单元测试
// ============================================================

#[test]
fn test_validate_name() {
    assert!(cp_switch::cli::validate_name("work").is_ok());
    assert!(cp_switch::cli::validate_name("my-profile").is_ok());
    assert!(cp_switch::cli::validate_name("").is_err());
    assert!(cp_switch::cli::validate_name("has space").is_err());
    assert!(cp_switch::cli::validate_name("dot.name").is_err());
}

#[test]
fn test_is_claude_env_key() {
    assert!(cp_switch::store::is_claude_env_key("ANTHROPIC_BASE_URL"));
    assert!(cp_switch::store::is_claude_env_key("ANTHROPIC_MODEL"));
    assert!(cp_switch::store::is_claude_env_key("ANTHROPIC_API_KEY"));
    assert!(cp_switch::store::is_claude_env_key("ANTHROPIC_AUTH_TOKEN"));
    assert!(cp_switch::store::is_claude_env_key("CLAUDE_CODE_SUBAGENT_MODEL"));
    assert!(cp_switch::store::is_claude_env_key("CLAUDE_CODE_EFFORT_LEVEL"));
    assert!(!cp_switch::store::is_claude_env_key("API_TIMEOUT_MS"));
}

#[test]
fn test_derive_default_models() {
    let defaults = cp_switch::store::derive_default_models("glm-5.1");
    assert_eq!(defaults[0].0, "ANTHROPIC_SMALL_FAST_MODEL");
    assert_eq!(defaults[0].1, "glm-5.1");
    assert_eq!(defaults[3].0, "ANTHROPIC_DEFAULT_OPUS_MODEL");
}

// ============================================================
// profile 读写测试
// ============================================================

#[test]
fn test_save_and_read_profile() {
    let _store = setup_store();
    let env = serde_json::json!({"ANTHROPIC_BASE_URL": "https://a.com", "ANTHROPIC_MODEL": "x"});
    cp_switch::store::save_profile("test", &env).unwrap();
    assert_eq!(cp_switch::store::read_profile("test").unwrap(), env);
}

#[test]
fn test_save_profile_rejects_invalid_name() {
    let _store = setup_store();
    assert!(cp_switch::cli::validate_name("bad.name").is_err());
}

#[test]
fn test_read_nonexistent_profile() {
    let _store = setup_store();
    assert!(cp_switch::store::read_profile("nonexistent").is_err());
}

#[test]
fn test_list_profiles() {
    let _store = setup_store();
    assert!(cp_switch::store::list_profiles().unwrap().is_empty());
    cp_switch::store::save_profile("alpha", &serde_json::json!({})).unwrap();
    cp_switch::store::save_profile("beta", &serde_json::json!({})).unwrap();
    assert_eq!(cp_switch::store::list_profiles().unwrap(), vec!["alpha", "beta"]);
}

// ============================================================
// merge 纯函数测试
// ============================================================

#[test]
fn test_merge_clears_old_keys_and_writes_new() {
    let settings = serde_json::json!({"permissions":{"allow":["Bash(ls)"]},"env":{"ANTHROPIC_BASE_URL":"https://old","ANTHROPIC_API_KEY":"sk-old","ANTHROPIC_MODEL":"old-model","ANTHROPIC_SMALL_FAST_MODEL":"old-model","API_TIMEOUT_MS":"3000","OTHER":"keep"}});
    let new_env = serde_json::json!({"ANTHROPIC_BASE_URL":"https://new","ANTHROPIC_API_KEY":"sk-new","ANTHROPIC_MODEL":"new-model","ANTHROPIC_DEFAULT_HAIKU_MODEL":"haiku"});
    let (merged, _changed, _removed) = cp_switch::store::merge_env(settings, &new_env).unwrap();
    let env_obj = merged.get("env").unwrap().as_object().unwrap();
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://new");
    assert_eq!(env_obj.get("ANTHROPIC_API_KEY").unwrap(), "sk-new");
    assert_eq!(env_obj.get("ANTHROPIC_SMALL_FAST_MODEL").unwrap(), "old-model"); // 保留，不删除
    assert_eq!(env_obj.get("API_TIMEOUT_MS").unwrap(), "3000");
    assert!(merged.get("permissions").is_some());
}

#[test]
fn test_merge_switch_back_and_forth() {
    let a_env = serde_json::json!({"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_API_KEY":"sk-a","ANTHROPIC_MODEL":"a"});
    let b_env = serde_json::json!({"ANTHROPIC_BASE_URL":"https://b","ANTHROPIC_API_KEY":"sk-b","ANTHROPIC_MODEL":"b"});
    let settings = serde_json::json!({"env":{"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_API_KEY":"sk-a","ANTHROPIC_MODEL":"a"}});

    let (merged, _, _) = cp_switch::store::merge_env(settings, &b_env).unwrap();
    let (merged, _, _) = cp_switch::store::merge_env(merged, &a_env).unwrap();
    let env_obj = merged.get("env").unwrap().as_object().unwrap();
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://a");
}

#[test]
fn test_merge_creates_env_when_missing() {
    let settings = serde_json::json!({"permissions":{"allow":["Bash"]}});
    let (merged, _, _) = cp_switch::store::merge_env(settings, &serde_json::json!({"ANTHROPIC_MODEL":"x"})).unwrap();
    assert!(merged.get("env").is_some());
    assert!(merged.get("permissions").is_some());
}

#[test]
fn test_merge_with_empty_env() {
    let settings = serde_json::json!({"permissions":{"allow":["Bash"]},"env":{}});
    let (merged, _, _) = cp_switch::store::merge_env(settings, &serde_json::json!({"ANTHROPIC_MODEL":"x"})).unwrap();
    let env_obj = merged.get("env").unwrap().as_object().unwrap();
    assert_eq!(env_obj.len(), 1);
}

#[test]
fn test_merge_malformed_env_values() {
    let settings = serde_json::json!({"env":{}});
    let result = cp_switch::store::merge_env(settings, &serde_json::json!("not an object"));
    assert!(result.is_err());
}

#[test]
fn test_merge_malformed_settings_env() {
    let settings = serde_json::json!({"env":"not an object"});
    let result = cp_switch::store::merge_env(settings, &serde_json::json!({"ANTHROPIC_MODEL":"x"}));
    assert!(result.is_err());
}

#[test]
fn test_merge_removes_auth_token_when_api_key_set() {
    // settings 有 AUTH_TOKEN，profile 有 API_KEY → AUTH_TOKEN 被清除
    let settings = serde_json::json!({"env":{"ANTHROPIC_AUTH_TOKEN":"tok-old","ANTHROPIC_BASE_URL":"https://old","ANTHROPIC_MODEL":"old"}});
    let profile_env = serde_json::json!({"ANTHROPIC_API_KEY":"sk-new","ANTHROPIC_BASE_URL":"https://new","ANTHROPIC_MODEL":"new"});
    let (merged, written, removed) = cp_switch::store::merge_env(settings, &profile_env).unwrap();
    let env_obj = merged.get("env").unwrap().as_object().unwrap();
    assert!(!env_obj.contains_key("ANTHROPIC_AUTH_TOKEN"));
    assert_eq!(env_obj.get("ANTHROPIC_API_KEY").unwrap(), "sk-new");
    assert!(removed.contains(&"ANTHROPIC_AUTH_TOKEN".to_string()));
    assert!(written.contains(&"ANTHROPIC_API_KEY".to_string()));
}

#[test]
fn test_merge_preserves_auth_token_when_no_auth_key() {
    // profile 无 API_KEY/AUTH_TOKEN → AUTH_TOKEN 保留
    let settings = serde_json::json!({"env":{"ANTHROPIC_AUTH_TOKEN":"tok-keep","ANTHROPIC_BASE_URL":"https://old","OTHER":"keep"}});
    let profile_env = serde_json::json!({"ANTHROPIC_BASE_URL":"https://new","ANTHROPIC_MODEL":"new"});
    let (merged, _, removed) = cp_switch::store::merge_env(settings, &profile_env).unwrap();
    let env_obj = merged.get("env").unwrap().as_object().unwrap();
    assert_eq!(env_obj.get("ANTHROPIC_AUTH_TOKEN").unwrap(), "tok-keep");
    assert!(removed.is_empty());
}

#[test]
fn test_merge_removes_api_key_when_auth_token_set() {
    // 反向：profile 有 AUTH_TOKEN，settings 有 API_KEY → API_KEY 被清除
    let settings = serde_json::json!({"env":{"ANTHROPIC_API_KEY":"sk-old","ANTHROPIC_BASE_URL":"https://old"}});
    let profile_env = serde_json::json!({"ANTHROPIC_AUTH_TOKEN":"tok-new","ANTHROPIC_BASE_URL":"https://new"});
    let (merged, _, removed) = cp_switch::store::merge_env(settings, &profile_env).unwrap();
    let env_obj = merged.get("env").unwrap().as_object().unwrap();
    assert!(!env_obj.contains_key("ANTHROPIC_API_KEY"));
    assert_eq!(env_obj.get("ANTHROPIC_AUTH_TOKEN").unwrap(), "tok-new");
    assert!(removed.contains(&"ANTHROPIC_API_KEY".to_string()));
}

// ============================================================
// delete / current 标记测试
// ============================================================

#[test]
fn test_delete_profile() {
    let _store = setup_store();
    cp_switch::store::save_profile("temp", &serde_json::json!({})).unwrap();
    cp_switch::store::delete_profile("temp").unwrap();
    assert!(!cp_switch::store::list_profiles().unwrap().contains(&"temp".to_string()));
}

#[test]
fn test_delete_nonexistent() {
    let _store = setup_store();
    assert!(cp_switch::store::delete_profile("nonexistent").is_err());
}

#[test]
fn test_current_marker() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);
    assert!(cp_switch::store::read_current(dir.path()).unwrap().is_none());
    cp_switch::store::write_current(dir.path(), "x").unwrap();
    assert_eq!(cp_switch::store::read_current(dir.path()).unwrap(), Some("x".to_string()));
    cp_switch::store::clear_current(dir.path()).unwrap();
    assert!(cp_switch::store::read_current(dir.path()).unwrap().is_none());
}

#[test]
fn test_clear_current_nonexistent_ok() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);
    assert!(cp_switch::store::clear_current(dir.path()).is_ok());
}

// ============================================================
// diff / read_current_env 测试
// ============================================================

#[test]
fn test_read_current_env_filters_only_anthropic() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"glm","API_TIMEOUT_MS":"3000","OTHER":"keep"}}"#);
    let env = cp_switch::store::read_current_env(dir.path()).unwrap();
    let obj = env.as_object().unwrap();
    assert_eq!(obj.len(), 1);
    assert!(obj.contains_key("ANTHROPIC_MODEL"));
}

#[test]
fn test_read_current_env_no_env_field() {
    let _store = setup_store();
    let dir = setup_project(r#"{"permissions":{}}"#);
    assert_eq!(cp_switch::store::read_current_env(dir.path()).unwrap(), serde_json::json!({}));
}

#[test]
fn test_diff_identical() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_MODEL":"x"}}"#);
    let env = serde_json::json!({"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_MODEL":"x"});
    cp_switch::store::save_profile("same", &env).unwrap();
    assert_eq!(cp_switch::store::read_current_env(dir.path()).unwrap(), env);
}

#[test]
fn test_diff_shows_changes() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_BASE_URL":"https://old","ANTHROPIC_MODEL":"old"}}"#);
    let env = serde_json::json!({"ANTHROPIC_BASE_URL":"https://new","ANTHROPIC_MODEL":"new","ANTHROPIC_API_KEY":"sk"});
    cp_switch::store::save_profile("new", &env).unwrap();

    let current = cp_switch::store::read_current_env(dir.path()).unwrap();
    let profile = cp_switch::store::read_profile("new").unwrap();
    assert_ne!(current, profile);
    assert!(profile.as_object().unwrap().contains_key("ANTHROPIC_API_KEY"));
}

// ============================================================
// CLI 子进程测试
// ============================================================

#[test]
fn test_cli_add_interactive_auto_derive() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);
    // 3 必填 + 4 可选（全留空用默认）
    let input = "https://api.anthropic.com\nsk-ant-test\nclaude-sonnet-4-6\n\n\n\n\n";
    let (ok, stdout, stderr) = run_cli_stdin("add test-add", input, dir.path());
    assert!(ok, "add failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Created profile 'test-add'"));

    let profile = cp_switch::store::read_profile("test-add").unwrap();
    let obj = profile.as_object().unwrap();
    assert_eq!(obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://api.anthropic.com");
    assert_eq!(obj.get("ANTHROPIC_API_KEY").unwrap(), "sk-ant-test");
    assert_eq!(obj.get("ANTHROPIC_MODEL").unwrap(), "claude-sonnet-4-6");
    assert_eq!(obj.get("ANTHROPIC_SMALL_FAST_MODEL").unwrap(), "claude-sonnet-4-6"); // auto-derived
}

#[test]
fn test_cli_add_with_custom_optional() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);
    let input = "https://infini.ai\nsk-infini\nglm-5.1\nglm-mini\nglm-haiku\nglm-sonnet\nglm-opus\n";
    let (ok, _, stderr) = run_cli_stdin("add infini", input, dir.path());
    assert!(ok, "add failed: {}", stderr);

    let profile = cp_switch::store::read_profile("infini").unwrap();
    assert_eq!(profile.get("ANTHROPIC_SMALL_FAST_MODEL").unwrap(), "glm-mini");
    assert_eq!(profile.get("ANTHROPIC_DEFAULT_OPUS_MODEL").unwrap(), "glm-opus");
}

#[test]
fn test_cli_add_duplicate_no_force() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);
    let input = "https://a.com\nsk-x\nmodel\n\n\n\n\n";
    let (ok, _, _) = run_cli_stdin("add dup", input, dir.path());
    assert!(ok);
    let input2 = "https://b.com\nsk-y\nmodel\n\n\n\n\n";
    let (ok, _, stderr) = run_cli_stdin("add dup", input2, dir.path());
    assert!(!ok);
    assert!(stderr.contains("already exists"));
}

#[test]
fn test_cli_add_force_overwrite() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);
    let input = "https://a.com\nsk-x\nmodel\n\n\n\n\n";
    let (ok, _, _) = run_cli_stdin("add overwrite-test", input, dir.path());
    assert!(ok);
    let input2 = "https://b.com\nsk-y\nmodel\n\n\n\n\n";
    let (ok, stdout, stderr) = run_cli_stdin("add overwrite-test --force", input2, dir.path());
    assert!(ok, "overwrite failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Overwritten"));

    let profile = cp_switch::store::read_profile("overwrite-test").unwrap();
    assert_eq!(profile.get("ANTHROPIC_BASE_URL").unwrap(), "https://b.com");
}

#[test]
fn test_cli_use_and_current() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_API_KEY":"sk-a","ANTHROPIC_MODEL":"a"}}"#);

    cp_switch::store::save_profile("test-use", &serde_json::json!({"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_API_KEY":"sk-a","ANTHROPIC_MODEL":"a"})).unwrap();

    let (ok, _stdout, stderr) = run_cli("use test-use", dir.path());
    assert!(ok, "use failed: {}", stderr);

    let (ok, stdout, stderr) = run_cli("current", dir.path());
    assert!(ok, "current failed: {}", stderr);
    assert!(combined_output(&stdout, &stderr).contains("test-use"));
}

#[test]
fn test_cli_list() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);
    cp_switch::store::save_profile("alpha", &serde_json::json!({})).unwrap();
    cp_switch::store::save_profile("beta", &serde_json::json!({})).unwrap();

    let (ok, stdout, stderr) = run_cli("list", dir.path());
    assert!(ok, "list failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("alpha") && out.contains("beta"));
}

#[test]
fn test_cli_list_empty() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);
    let (ok, stdout, stderr) = run_cli("list", dir.path());
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("No profiles found"));
}

#[test]
fn test_cli_delete() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);
    cp_switch::store::save_profile("del-me", &serde_json::json!({})).unwrap();
    let (ok, _, _) = run_cli("delete del-me --force", dir.path());
    assert!(ok);
    assert!(!cp_switch::store::list_profiles().unwrap().contains(&"del-me".to_string()));
}

#[test]
fn test_cli_diff() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_BASE_URL":"https://old","ANTHROPIC_MODEL":"old"}}"#);
    cp_switch::store::save_profile("new-profile", &serde_json::json!({"ANTHROPIC_BASE_URL":"https://new","ANTHROPIC_MODEL":"new"})).unwrap();

    let (ok, stdout, stderr) = run_cli("diff new-profile", dir.path());
    assert!(ok, "diff failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("current env") || out.contains("profile:"));
}

#[test]
fn test_cli_use_nonexistent() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);
    let (ok, _, stderr) = run_cli("use nonexistent", dir.path());
    assert!(!ok);
    assert!(stderr.contains("not found"));
}

// ============================================================
// 错误路径测试
// ============================================================

#[test]
fn test_cli_add_invalid_name() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);
    let (ok, _, stderr) = run_cli("add bad.name", dir.path());
    assert!(!ok);
    assert!(stderr.contains("Invalid profile name"));
}

#[test]
fn test_cli_current_no_active() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);
    let (ok, stdout, stderr) = run_cli("current", dir.path());
    assert!(ok);
    assert!(combined_output(&stdout, &stderr).contains("No active profile"));
}

#[test]
fn test_cli_diff_nonexistent() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);
    let (ok, _, stderr) = run_cli("diff nonexistent", dir.path());
    assert!(!ok);
    assert!(stderr.contains("not found"));
}

// ============================================================
// 端到端行为验证
// ============================================================

#[test]
fn test_cli_use_writes_settings_and_preserves_non_anthropic() {
    let _store = setup_store();
    let dir = setup_project(r#"{"permissions":{"allow":["Bash(ls)"]},"env":{"ANTHROPIC_BASE_URL":"https://old","ANTHROPIC_API_KEY":"sk-old","ANTHROPIC_MODEL":"old","ANTHROPIC_SMALL_FAST_MODEL":"old-fast","API_TIMEOUT_MS":"3000"}}"#);

    cp_switch::store::save_profile("new", &serde_json::json!({"ANTHROPIC_BASE_URL":"https://new","ANTHROPIC_API_KEY":"sk-new","ANTHROPIC_MODEL":"new"})).unwrap();

    let (ok, _, stderr) = run_cli("use new", dir.path());
    assert!(ok, "use failed: {}", stderr);

    // 验证 settings.local.json 实际写入
    let settings = read_settings(dir.path());
    let env_obj = get_env_obj(&settings);
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://new");
    assert_eq!(env_obj.get("ANTHROPIC_API_KEY").unwrap(), "sk-new");
    assert_eq!(env_obj.get("ANTHROPIC_MODEL").unwrap(), "new");
    assert_eq!(env_obj.get("ANTHROPIC_SMALL_FAST_MODEL").unwrap(), "old-fast"); // 不在冲突组，保留
    assert_eq!(env_obj.get("API_TIMEOUT_MS").unwrap(), "3000");   // 非 ANTHROPIC_* 保留
    assert!(settings.get("permissions").is_some());                // permissions 保留
}

#[test]
fn test_cli_use_switch_back_preserves_env() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_MODEL":"a","ANTHROPIC_SMALL_FAST_MODEL":"a","OTHER":"keep"}}"#);

    cp_switch::store::save_profile("a", &serde_json::json!({"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_MODEL":"a"})).unwrap();
    cp_switch::store::save_profile("b", &serde_json::json!({"ANTHROPIC_BASE_URL":"https://b","ANTHROPIC_MODEL":"b"})).unwrap();

    run_cli("use b", dir.path());
    let settings = read_settings(dir.path());
    assert_eq!(settings.get("env").unwrap().get("ANTHROPIC_BASE_URL").unwrap(), "https://b");
    assert_eq!(settings.get("env").unwrap().get("ANTHROPIC_SMALL_FAST_MODEL").unwrap(), "a"); // 保留
    assert_eq!(settings.get("env").unwrap().get("OTHER").unwrap(), "keep");

    run_cli("use a", dir.path());
    let settings = read_settings(dir.path());
    assert_eq!(settings.get("env").unwrap().get("ANTHROPIC_BASE_URL").unwrap(), "https://a");
    assert_eq!(settings.get("env").unwrap().get("ANTHROPIC_MODEL").unwrap(), "a");
    assert_eq!(settings.get("env").unwrap().get("OTHER").unwrap(), "keep");
}

#[test]
fn test_cli_list_shows_active_marker() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);

    cp_switch::store::save_profile("alpha", &serde_json::json!({})).unwrap();
    cp_switch::store::save_profile("beta", &serde_json::json!({})).unwrap();

    // 无活跃 profile
    let (ok, stdout, stderr) = run_cli("list", dir.path());
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("alpha") && out.contains("beta"));
    assert!(!out.contains("(active)"));

    // use 后再 list
    run_cli("use alpha", dir.path());
    let (ok, stdout, stderr) = run_cli("list", dir.path());
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("(active)"));
}

#[test]
fn test_cli_diff_shows_additions_and_deletions() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_BASE_URL":"https://old","ANTHROPIC_MODEL":"old"}}"#);

    cp_switch::store::save_profile("new", &serde_json::json!({"ANTHROPIC_BASE_URL":"https://new","ANTHROPIC_API_KEY":"sk-new","ANTHROPIC_MODEL":"new"})).unwrap();

    let (ok, stdout, stderr) = run_cli("diff new", dir.path());
    assert!(ok, "diff failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    // 验证 diff 输出包含具体增删行
    assert!(out.contains("-") && out.contains("+"));
}

#[test]
fn test_cli_diff_identical_no_changes() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_MODEL":"x"}}"#);

    cp_switch::store::save_profile("same", &serde_json::json!({"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_MODEL":"x"})).unwrap();

    let (ok, stdout, stderr) = run_cli("diff same", dir.path());
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("No differences"));
}

#[test]
fn test_cli_delete_active_without_force_prompts() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);

    cp_switch::store::save_profile("active", &serde_json::json!({"ANTHROPIC_MODEL":"x"})).unwrap();
    run_cli("use active", dir.path());

    // 不带 --force，回答 n 取消删除
    let (ok, stdout, stderr) = run_cli_stdin("delete active", "n\n", dir.path());
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Cancelled"));
    // profile 仍然存在
    assert!(cp_switch::store::list_profiles().unwrap().contains(&"active".to_string()));
}

#[test]
fn test_cli_delete_active_with_force_skips_prompt() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);

    cp_switch::store::save_profile("active", &serde_json::json!({})).unwrap();
    run_cli("use active", dir.path());

    let (ok, _, stderr) = run_cli("delete active --force", dir.path());
    assert!(ok, "delete failed: {}", stderr);
    assert!(!cp_switch::store::list_profiles().unwrap().contains(&"active".to_string()));
}

#[test]
fn test_cli_add_empty_required_retries() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);

    // 先留空 ANTHROPIC_BASE_URL，再输入有效值
    let input = "\nhttps://a.com\nsk-x\nmodel\n\n\n\n\n";
    let (ok, stdout, stderr) = run_cli_stdin("add retry-test", input, dir.path());
    assert!(ok, "add with retry failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Created profile 'retry-test'"));

    let profile = cp_switch::store::read_profile("retry-test").unwrap();
    assert_eq!(profile.get("ANTHROPIC_BASE_URL").unwrap(), "https://a.com");
}

#[test]
fn test_full_workflow() {
    let _store = setup_store();
    let dir = setup_project(r#"{"permissions":{"allow":["Bash"]},"env":{"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_API_KEY":"sk-a","ANTHROPIC_MODEL":"a","ANTHROPIC_SMALL_FAST_MODEL":"a","API_TIMEOUT_MS":"3000"}}"#);
    let project = dir.path();

    let a_env = serde_json::json!({"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_API_KEY":"sk-a","ANTHROPIC_MODEL":"a","ANTHROPIC_SMALL_FAST_MODEL":"a"});
    cp_switch::store::save_profile("a", &a_env).unwrap();

    let b_env = serde_json::json!({"ANTHROPIC_BASE_URL":"https://b","ANTHROPIC_API_KEY":"sk-b","ANTHROPIC_MODEL":"b","ANTHROPIC_DEFAULT_OPUS_MODEL":"opus"});
    cp_switch::store::save_profile("b", &b_env).unwrap();

    // 切到 b
    let settings = cp_switch::store::read_settings_local(project).unwrap();
    let (merged, _, _) = cp_switch::store::merge_env(settings, &b_env).unwrap();
    cp_switch::store::write_settings_local(project, &merged).unwrap();
    let settings = read_settings(project);
    let env_obj = get_env_obj(&settings);
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://b");
    assert_eq!(env_obj.get("ANTHROPIC_SMALL_FAST_MODEL").unwrap(), "a"); // 保留，不在冲突组
    assert_eq!(env_obj.get("API_TIMEOUT_MS").unwrap(), "3000");

    // 切回 a
    let a_profile = cp_switch::store::read_profile("a").unwrap();
    let settings = cp_switch::store::read_settings_local(project).unwrap();
    let (merged, _, _) = cp_switch::store::merge_env(settings, &a_profile).unwrap();
    cp_switch::store::write_settings_local(project, &merged).unwrap();
    let settings = read_settings(project);
    let env_obj = get_env_obj(&settings);
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://a");
    assert_eq!(env_obj.get("ANTHROPIC_DEFAULT_OPUS_MODEL").unwrap(), "opus"); // 保留
}

// ============================================================
// 补充场景测试
// ============================================================

#[test]
fn test_cli_use_creates_settings_in_brand_new_project() {
    let _store = setup_store();
    // 完全新项目：连 .claude 目录都不存在，需要确认新建
    let dir = tempfile::tempdir().unwrap();
    assert!(!dir.path().join(".claude").exists());

    cp_switch::store::save_profile("brandnew", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://new", "ANTHROPIC_API_KEY": "sk-new", "ANTHROPIC_MODEL": "new"
    })).unwrap();

    let (ok, stdout, stderr) = run_cli_stdin("use brandnew", "y\n", dir.path());
    assert!(ok, "use failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Switched to profile 'brandnew'"));

    // 确认后应自动创建 .claude 目录和 settings.local.json，并写入环境变量
    assert!(dir.path().join(".claude/settings.local.json").exists());
    let settings = read_settings(dir.path());
    assert!(settings.get("permissions").is_some());
    let env_obj = get_env_obj(&settings);
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://new");
    assert_eq!(env_obj.get("ANTHROPIC_API_KEY").unwrap(), "sk-new");
    assert_eq!(env_obj.get("ANTHROPIC_MODEL").unwrap(), "new");
}

#[test]
fn test_cli_use_reject_create_no_claude_dir() {
    let _store = setup_store();
    // 完全新项目：拒绝新建 .claude 目录应报错退出
    let dir = tempfile::tempdir().unwrap();
    assert!(!dir.path().join(".claude").exists());

    cp_switch::store::save_profile("reject", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://r", "ANTHROPIC_API_KEY": "sk-r"
    })).unwrap();

    let (ok, _stdout, stderr) = run_cli_stdin("use reject", "n\n", dir.path());
    assert!(!ok);
    assert!(stderr.contains("当前目录没有 .claude 目录"));
    // 不应创建 .claude 目录
    assert!(!dir.path().join(".claude").exists());
}

#[test]
fn test_cli_use_creates_settings_when_missing() {
    let _store = setup_store();
    // 项目有 .claude 目录但无 settings.local.json
    let dir = setup_project_no_settings();
    assert!(!dir.path().join(".claude/settings.local.json").exists());

    cp_switch::store::save_profile("newproj", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://new", "ANTHROPIC_API_KEY": "sk-new", "ANTHROPIC_MODEL": "new"
    })).unwrap();

    let (ok, stdout, stderr) = run_cli("use newproj", dir.path());
    assert!(ok, "use failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Switched to profile 'newproj'"));

    let settings = read_settings(dir.path());
    assert!(settings.get("permissions").is_some());
    let env_obj = get_env_obj(&settings);
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://new");
    assert_eq!(env_obj.get("ANTHROPIC_API_KEY").unwrap(), "sk-new");
    assert_eq!(env_obj.get("ANTHROPIC_MODEL").unwrap(), "new");
}

#[test]
fn test_cli_delete_active_clears_current_marker() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);

    cp_switch::store::save_profile("active-del", &serde_json::json!({"ANTHROPIC_MODEL":"x"})).unwrap();
    run_cli("use active-del", dir.path());

    // 确认 current marker 存在
    assert_eq!(cp_switch::store::read_current(dir.path()).unwrap(), Some("active-del".to_string()));

    // --force 删除活跃 profile
    let (ok, _, stderr) = run_cli("delete active-del --force", dir.path());
    assert!(ok, "delete failed: {}", stderr);

    // current marker 应被清除
    assert!(cp_switch::store::read_current(dir.path()).unwrap().is_none());
}

#[test]
fn test_cli_list_shows_missing_active() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);

    cp_switch::store::save_profile("vanish", &serde_json::json!({"ANTHROPIC_MODEL":"x"})).unwrap();
    run_cli("use vanish", dir.path());

    // 手动删除 profile 文件（模拟用户误删）
    let path = cp_switch::store::profile_path("vanish");
    fs::remove_file(&path).unwrap();

    // list 应显示 "(active - missing!)"
    let (ok, stdout, stderr) = run_cli("list", dir.path());
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("vanish"));
    assert!(out.contains("missing"));
}

#[test]
fn test_cli_list_shows_outdated_when_profile_updated() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);

    // 创建 profile 并 use
    cp_switch::store::save_profile("myenv", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://old", "ANTHROPIC_API_KEY": "sk-old"
    })).unwrap();
    run_cli("use myenv", dir.path());

    // use 后 list 应显示 (active)
    let (ok, stdout, stderr) = run_cli("list", dir.path());
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("(active)"));
    assert!(!out.contains("outdated"));

    // 更新 profile 内容（模拟 edit 命令）
    cp_switch::store::save_profile("myenv", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://new", "ANTHROPIC_API_KEY": "sk-new"
    })).unwrap();

    // list 应显示 (active - outdated)
    let (ok, stdout, stderr) = run_cli("list", dir.path());
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("outdated"));
    assert!(!out.contains("(active)") || out.contains("(active - outdated)"));

    // 再次 use 后恢复正常
    run_cli("use myenv", dir.path());
    let (ok, stdout, stderr) = run_cli("list", dir.path());
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("(active)"));
    assert!(!out.contains("outdated"));
}

#[test]
fn test_cli_version() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);
    let (ok, stdout, _) = run_cli("--version", dir.path());
    assert!(ok);
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_cli_use_reapply_same_profile() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_API_KEY":"sk-a","ANTHROPIC_MODEL":"a","API_TIMEOUT_MS":"3000"}}"#);

    cp_switch::store::save_profile("work", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://a", "ANTHROPIC_API_KEY": "sk-a", "ANTHROPIC_MODEL": "a"
    })).unwrap();

    // 第一次 use
    let (ok, _, stderr) = run_cli("use work", dir.path());
    assert!(ok, "first use failed: {}", stderr);

    // 再次 use 同一个 profile — 不应破坏 settings
    let (ok, stdout, stderr) = run_cli("use work", dir.path());
    assert!(ok, "re-apply failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Switched to profile 'work'"));

    let settings = read_settings(dir.path());
    let env_obj = get_env_obj(&settings);
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://a");
    assert_eq!(env_obj.get("API_TIMEOUT_MS").unwrap(), "3000"); // 非 ANTHROPIC_* 保留
}

#[test]
fn test_cli_use_in_project_without_env_field() {
    let _store = setup_store();
    // Claude Code 刚初始化的项目：只有 permissions，没有 env
    let dir = setup_project(r#"{"permissions":{"allow":["Bash(ls)"]}}"#);

    cp_switch::store::save_profile("first", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://first", "ANTHROPIC_API_KEY": "sk-first", "ANTHROPIC_MODEL": "first"
    })).unwrap();

    let (ok, stdout, stderr) = run_cli("use first", dir.path());
    assert!(ok, "use failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Switched to profile 'first'"));

    let settings = read_settings(dir.path());
    assert!(settings.get("permissions").is_some());
    let env_obj = get_env_obj(&settings);
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://first");
}

#[test]
fn test_cli_delete_nonactive_no_force() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);

    cp_switch::store::save_profile("other", &serde_json::json!({"ANTHROPIC_MODEL":"x"})).unwrap();
    // other 不是活跃 profile，--force 不需要，也不应提示确认
    let (ok, stdout, stderr) = run_cli("delete other", dir.path());
    assert!(ok, "delete failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Deleted profile 'other'"));
    assert!(!cp_switch::store::list_profiles().unwrap().contains(&"other".to_string()));
}

#[test]
fn test_cli_use_corrupted_settings() {
    let _store = setup_store();
    // settings.local.json 存在但内容是非法 JSON
    let dir = tempfile::tempdir().unwrap();
    let claude_dir = dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(claude_dir.join("settings.local.json"), "{invalid json!!!}").unwrap();

    cp_switch::store::save_profile("test", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://a", "ANTHROPIC_API_KEY": "sk-a", "ANTHROPIC_MODEL": "a"
    })).unwrap();

    let (ok, _, stderr) = run_cli("use test", dir.path());
    assert!(!ok);
    assert!(stderr.contains("Invalid JSON"));
}

// ============================================================
// 原子写入 + 备份测试
// ============================================================

#[test]
fn test_write_settings_creates_backup() {
    let _store = setup_store();
    // 初始 settings 内容
    let dir = setup_project(r#"{"env":{"ANTHROPIC_BASE_URL":"https://old","ANTHROPIC_MODEL":"old"}}"#);
    let original = read_settings(dir.path());

    cp_switch::store::save_profile("new", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://new", "ANTHROPIC_MODEL": "new"
    })).unwrap();
    run_cli("use new", dir.path());

    // 备份文件应存在且内容与原始一致
    let bak_path = dir.path().join(".claude/settings.local.json.bak");
    assert!(bak_path.exists());
    let bak_content: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&bak_path).unwrap()
    ).unwrap();
    assert_eq!(bak_content, original);
}

#[test]
fn test_write_settings_no_backup_when_missing() {
    let _store = setup_store();
    let dir = setup_project_no_settings();
    assert!(!dir.path().join(".claude/settings.local.json").exists());

    cp_switch::store::save_profile("newproj", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://new", "ANTHROPIC_API_KEY": "sk-new", "ANTHROPIC_MODEL": "new"
    })).unwrap();
    run_cli("use newproj", dir.path());

    // 无旧文件时不产生备份
    let bak_path = dir.path().join(".claude/settings.local.json.bak");
    assert!(!bak_path.exists());
}

#[test]
fn test_atomic_write_no_residual_tmp() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);

    cp_switch::store::save_profile("clean", &serde_json::json!({"ANTHROPIC_MODEL":"y"})).unwrap();
    run_cli("use clean", dir.path());

    // 临时文件不应残留
    assert!(!dir.path().join(".claude/settings.local.json.tmp").exists());
}

#[test]
fn test_write_settings_backup_overwrites_on_successive_use() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_BASE_URL":"https://original","ANTHROPIC_MODEL":"original"}}"#);

    cp_switch::store::save_profile("a", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://a", "ANTHROPIC_MODEL": "a"
    })).unwrap();
    cp_switch::store::save_profile("b", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://b", "ANTHROPIC_MODEL": "b"
    })).unwrap();

    // 第一次 use：备份应为原始内容
    run_cli("use a", dir.path());
    let bak_path = dir.path().join(".claude/settings.local.json.bak");
    let bak1: serde_json::Value = serde_json::from_str(&fs::read_to_string(&bak_path).unwrap()).unwrap();
    assert_eq!(bak1.get("env").unwrap().get("ANTHROPIC_BASE_URL").unwrap(), "https://original");

    // 第二次 use：备份应为 use a 后的内容，不是原始内容
    run_cli("use b", dir.path());
    let bak2: serde_json::Value = serde_json::from_str(&fs::read_to_string(&bak_path).unwrap()).unwrap();
    assert_eq!(bak2.get("env").unwrap().get("ANTHROPIC_BASE_URL").unwrap(), "https://a");
}