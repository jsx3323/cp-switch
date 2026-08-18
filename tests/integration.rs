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

/// 连 .claude 目录都不存在的裸项目
fn setup_bare_project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    assert!(!dir.path().join(".claude").exists());
    dir
}

fn setup_project(settings_json: &str) -> TempDir {
    let dir = setup_project_no_settings();
    fs::write(dir.path().join(".claude/settings.local.json"), settings_json).unwrap();
    dir
}

fn read_json(path: impl AsRef<std::path::Path>) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn read_settings(project: &std::path::Path) -> serde_json::Value {
    read_json(project.join(".claude/settings.local.json"))
}

fn get_env_obj(settings: &serde_json::Value) -> &serde_json::Map<String, serde_json::Value> {
    settings.get("env").unwrap().as_object().unwrap()
}

/// 唯一的子进程入口：cwd 用于项目级命令，home 用于 --user 命令
fn spawn_cli(
    args: &str,
    input: &str,
    cwd: Option<&std::path::Path>,
    home: Option<&std::path::Path>,
) -> (bool, String, String) {
    let bin = std::env::var("CARGO_BIN_EXE_cp-switch").unwrap();
    let mut cmd = Command::new(&bin);
    cmd.args(args.split_whitespace())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env("CP_SWITCH_DIR", store_dir_val());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    if let Some(dir) = home {
        cmd.env("HOME", dir);
    }
    let mut child = cmd.spawn().unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    (output.status.success(),
     String::from_utf8_lossy(&output.stdout).to_string(),
     String::from_utf8_lossy(&output.stderr).to_string())
}

/// 子进程的原始退出码，用于断言 CsError::exit_code 的对外契约
fn exit_code(args: &str, project: &std::path::Path) -> i32 {
    let bin = std::env::var("CARGO_BIN_EXE_cp-switch").unwrap();
    Command::new(&bin)
        .args(args.split_whitespace())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .env("CP_SWITCH_DIR", store_dir_val())
        .current_dir(project)
        .status()
        .unwrap()
        .code()
        .unwrap()
}

fn run_cli(args: &str, project: &std::path::Path) -> (bool, String, String) {
    spawn_cli(args, "", Some(project), None)
}

fn run_cli_stdin(args: &str, input: &str, project: &std::path::Path) -> (bool, String, String) {
    spawn_cli(args, input, Some(project), None)
}

/// 有 .claude 目录但没有 settings.local.json
fn setup_project_no_settings() -> TempDir {
    let dir = setup_bare_project();
    fs::create_dir_all(dir.path().join(".claude")).unwrap();
    dir
}

fn combined_output(stdout: &str, stderr: &str) -> String {
    stdout.to_string() + stderr
}

// 子进程经 env::current_dir() 规范化了项目路径（macOS 上 /var → /private/var），
// 进程内读取子进程写入的 current marker 时须用同样规范化的路径，否则 key 不匹配。
fn read_current_canonical(project: &std::path::Path) -> Option<String> {
    cp_switch::store::read_current(&project.canonicalize().unwrap()).unwrap()
}

// ── 用户模式测试辅助 ──

fn setup_home() -> TempDir {
    tempfile::tempdir().unwrap()
}

fn read_user_settings(home: &std::path::Path) -> serde_json::Value {
    read_json(home.join(".claude/settings.json"))
}

/// 预置用户级 settings.json
fn setup_user_settings(home: &std::path::Path, settings_json: &str) {
    let claude_dir = home.join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(claude_dir.join("settings.json"), settings_json).unwrap();
}

fn run_cli_user(args: &str, input: &str, home: &std::path::Path) -> (bool, String, String) {
    spawn_cli(args, input, None, Some(home))
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
    // 精确管理的 11 个 key
    assert!(cp_switch::store::is_claude_env_key("ANTHROPIC_BASE_URL"));
    assert!(cp_switch::store::is_claude_env_key("ANTHROPIC_API_KEY"));
    assert!(cp_switch::store::is_claude_env_key("ANTHROPIC_AUTH_TOKEN"));
    assert!(cp_switch::store::is_claude_env_key("ANTHROPIC_MODEL"));
    assert!(cp_switch::store::is_claude_env_key("ANTHROPIC_SMALL_FAST_MODEL"));
    assert!(cp_switch::store::is_claude_env_key("ANTHROPIC_DEFAULT_HAIKU_MODEL"));
    assert!(cp_switch::store::is_claude_env_key("ANTHROPIC_DEFAULT_SONNET_MODEL"));
    assert!(cp_switch::store::is_claude_env_key("ANTHROPIC_DEFAULT_OPUS_MODEL"));
    assert!(cp_switch::store::is_claude_env_key("CLAUDE_CODE_SUBAGENT_MODEL"));
    assert!(cp_switch::store::is_claude_env_key("CLAUDE_CODE_EFFORT_LEVEL"));
    assert!(cp_switch::store::is_claude_env_key("CLAUDE_CODE_AUTO_COMPACT_WINDOW"));
    // 前缀相同但不在白名单内的
    assert!(!cp_switch::store::is_claude_env_key("ANTHROPIC_OTHER"));
    assert!(!cp_switch::store::is_claude_env_key("CLAUDE_CODE_FOO"));
    assert!(!cp_switch::store::is_claude_env_key("ANTHROPIC_"));
    // 完全不相关的
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
    let (merged, _removed) = cp_switch::store::merge_env(settings, &new_env).unwrap();
    let env_obj = merged.get("env").unwrap().as_object().unwrap();
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://new");
    assert_eq!(env_obj.get("ANTHROPIC_API_KEY").unwrap(), "sk-new");
    assert!(env_obj.get("ANTHROPIC_SMALL_FAST_MODEL").is_none()); // 受管理 key，已清除
    assert_eq!(env_obj.get("API_TIMEOUT_MS").unwrap(), "3000");
    assert!(merged.get("permissions").is_some());
}

#[test]
fn test_merge_removed_excludes_overwritten_keys() {
    // removed 只报告旧 env 中受管理、且 profile 未重新写入的 key；
    // 被 profile 覆盖的 key（BASE_URL/API_KEY）不应出现在 removed 中
    let settings = serde_json::json!({"env":{
        "ANTHROPIC_BASE_URL":"https://old",
        "ANTHROPIC_API_KEY":"sk-old",
        "ANTHROPIC_SMALL_FAST_MODEL":"old",
        "OTHER":"keep"}});
    let new_env = serde_json::json!({"ANTHROPIC_BASE_URL":"https://new","ANTHROPIC_API_KEY":"sk-new","ANTHROPIC_DEFAULT_HAIKU_MODEL":"haiku"});
    let (_merged, mut removed) = cp_switch::store::merge_env(settings, &new_env).unwrap();
    removed.sort();
    assert_eq!(removed, vec!["ANTHROPIC_SMALL_FAST_MODEL".to_string()]);
}

#[test]
fn test_merge_switch_back_and_forth() {
    let a_env = serde_json::json!({"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_API_KEY":"sk-a","ANTHROPIC_MODEL":"a"});
    let b_env = serde_json::json!({"ANTHROPIC_BASE_URL":"https://b","ANTHROPIC_API_KEY":"sk-b","ANTHROPIC_MODEL":"b"});
    let settings = serde_json::json!({"env":{"ANTHROPIC_BASE_URL":"https://a","ANTHROPIC_API_KEY":"sk-a","ANTHROPIC_MODEL":"a"}});

    let (merged, _) = cp_switch::store::merge_env(settings, &b_env).unwrap();
    let (merged, _) = cp_switch::store::merge_env(merged, &a_env).unwrap();
    let env_obj = merged.get("env").unwrap().as_object().unwrap();
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://a");
}

#[test]
fn test_merge_creates_env_when_missing() {
    let settings = serde_json::json!({"permissions":{"allow":["Bash"]}});
    let (merged, _) = cp_switch::store::merge_env(settings, &serde_json::json!({"ANTHROPIC_MODEL":"x"})).unwrap();
    assert!(merged.get("env").is_some());
    assert!(merged.get("permissions").is_some());
}

#[test]
fn test_merge_with_empty_env() {
    let settings = serde_json::json!({"permissions":{"allow":["Bash"]},"env":{}});
    let (merged, _) = cp_switch::store::merge_env(settings, &serde_json::json!({"ANTHROPIC_MODEL":"x"})).unwrap();
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

// ============================================================
// clear_env 纯函数测试
// ============================================================

#[test]
fn test_clear_env_removes_all_managed_keys() {
    let settings = serde_json::json!({
        "permissions": {"allow": ["Bash"]},
        "env": {
            "ANTHROPIC_BASE_URL": "https://x",
            "ANTHROPIC_API_KEY": "sk-x",
            "ANTHROPIC_AUTH_TOKEN": "tok-x",
            "ANTHROPIC_MODEL": "x",
            "ANTHROPIC_SMALL_FAST_MODEL": "x",
            "ANTHROPIC_DEFAULT_HAIKU_MODEL": "x",
            "ANTHROPIC_DEFAULT_SONNET_MODEL": "x",
            "ANTHROPIC_DEFAULT_OPUS_MODEL": "x",
            "CLAUDE_CODE_SUBAGENT_MODEL": "x",
            "CLAUDE_CODE_EFFORT_LEVEL": "high",
            "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "50",
            "API_TIMEOUT_MS": "3000",
            "OTHER": "keep"
        }
    });
    let (merged, removed) = cp_switch::store::clear_env(settings).unwrap();
    let env_obj = merged.get("env").unwrap().as_object().unwrap();
    // 11 个 managed key 都被清除
    assert!(env_obj.get("ANTHROPIC_BASE_URL").is_none());
    assert!(env_obj.get("ANTHROPIC_API_KEY").is_none());
    assert!(env_obj.get("ANTHROPIC_AUTH_TOKEN").is_none());
    assert!(env_obj.get("ANTHROPIC_MODEL").is_none());
    assert!(env_obj.get("ANTHROPIC_SMALL_FAST_MODEL").is_none());
    assert!(env_obj.get("ANTHROPIC_DEFAULT_HAIKU_MODEL").is_none());
    assert!(env_obj.get("ANTHROPIC_DEFAULT_SONNET_MODEL").is_none());
    assert!(env_obj.get("ANTHROPIC_DEFAULT_OPUS_MODEL").is_none());
    assert!(env_obj.get("CLAUDE_CODE_SUBAGENT_MODEL").is_none());
    assert!(env_obj.get("CLAUDE_CODE_EFFORT_LEVEL").is_none());
    assert!(env_obj.get("CLAUDE_CODE_AUTO_COMPACT_WINDOW").is_none());
    // 非 managed key 保留
    assert_eq!(env_obj.get("API_TIMEOUT_MS").unwrap(), "3000");
    assert_eq!(env_obj.get("OTHER").unwrap(), "keep");
    assert_eq!(env_obj.len(), 2);
    // permissions 保留
    assert!(merged.get("permissions").is_some());
    // removed 列表包含所有清掉的 key
    assert_eq!(removed.len(), 11);
}

#[test]
fn test_clear_env_keeps_missing_env_missing() {
    let settings = serde_json::json!({"permissions": {"allow": ["Bash"]}});
    let (merged, removed) = cp_switch::store::clear_env(settings).unwrap();
    // 原本没有 env，清除后不应凭空多出 "env": {}
    assert!(merged.get("env").is_none());
    assert!(removed.is_empty());
    assert!(merged.get("permissions").is_some());
}

#[test]
fn test_clear_env_drops_env_when_emptied() {
    let settings = serde_json::json!({
        "model": "claude-fable-5",
        "env": {"ANTHROPIC_BASE_URL": "https://x", "ANTHROPIC_API_KEY": "sk-x"}
    });
    let (merged, removed) = cp_switch::store::clear_env(settings).unwrap();
    // env 里只剩受管 key，清空后整个字段一并删除
    assert!(merged.get("env").is_none());
    assert_eq!(removed.len(), 2);
    // 顶层 model 不受影响
    assert_eq!(merged.get("model").unwrap(), "claude-fable-5");
}

// ============================================================
// set_model / clear_model 纯函数测试
// ============================================================

#[test]
fn test_set_model_adds_top_level_field() {
    let settings = serde_json::json!({"permissions":{"allow":[]},"env":{"ANTHROPIC_MODEL":"x"}});
    let new = cp_switch::store::set_model(settings, "claude-fable-5[1m]").unwrap();
    // 顶层 model 字段被写入，且不影响 env / permissions
    assert_eq!(new.get("model").unwrap(), "claude-fable-5[1m]");
    assert_eq!(new.get("env").unwrap().get("ANTHROPIC_MODEL").unwrap(), "x");
    assert!(new.get("permissions").is_some());
}

#[test]
fn test_set_model_overwrites_existing() {
    let settings = serde_json::json!({"model":"old","env":{}});
    let new = cp_switch::store::set_model(settings, "claude-opus-4-8").unwrap();
    assert_eq!(new.get("model").unwrap(), "claude-opus-4-8");
}

#[test]
fn test_clear_model_removes_field() {
    let settings = serde_json::json!({"model":"claude-fable-5","env":{}});
    let (new, existed) = cp_switch::store::clear_model(settings).unwrap();
    assert!(existed);
    assert!(new.get("model").is_none());
    assert!(new.get("env").is_some());
}

#[test]
fn test_clear_model_when_absent() {
    let settings = serde_json::json!({"env":{}});
    let (new, existed) = cp_switch::store::clear_model(settings).unwrap();
    assert!(!existed);
    assert!(new.get("model").is_none());
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
    assert_eq!(obj.get("ANTHROPIC_AUTH_TOKEN").unwrap(), "sk-ant-test");
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
    assert!(env_obj.get("ANTHROPIC_SMALL_FAST_MODEL").is_none()); // 受管理 key，已清除
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
    assert!(settings.get("env").unwrap().get("ANTHROPIC_SMALL_FAST_MODEL").is_none()); // 受管理 key，已清除
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
fn test_cli_diff_in_bare_project_no_prompt() {
    let _store = setup_store();
    let dir = setup_bare_project();
    cp_switch::store::save_profile("bare", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://bare", "ANTHROPIC_MODEL": "m-bare"
    })).unwrap();

    // 只读命令：没有 .claude 也直接按空 env 比较，不提示、不创建
    let (ok, stdout, stderr) = run_cli("diff bare", dir.path());
    assert!(ok, "diff failed: {}", stderr);
    assert!(!combined_output(&stdout, &stderr).contains("没有 .claude 目录"));
    assert!(stdout.contains("+++ profile: bare"));
    assert!(stdout.contains("ANTHROPIC_BASE_URL"));
    assert!(!dir.path().join(".claude").exists());
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

// ============================================================
// 补充场景测试
// ============================================================

#[test]
fn test_cli_use_creates_settings_in_brand_new_project() {
    let _store = setup_store();
    // 完全新项目：连 .claude 目录都不存在，需要确认新建
    let dir = setup_bare_project();

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
    let dir = setup_bare_project();

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
    assert_eq!(read_current_canonical(dir.path()), Some("active-del".to_string()));

    // --force 删除活跃 profile
    let (ok, _, stderr) = run_cli("delete active-del --force", dir.path());
    assert!(ok, "delete failed: {}", stderr);

    // current marker 应被清除
    assert!(read_current_canonical(dir.path()).is_none());
}

#[test]
fn test_cli_list_shows_missing_active() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{"ANTHROPIC_MODEL":"x"}}"#);

    cp_switch::store::save_profile("vanish", &serde_json::json!({"ANTHROPIC_MODEL":"x"})).unwrap();
    run_cli("use vanish", dir.path());

    // 手动删除 profile（模拟用户误删）
    cp_switch::store::delete_profile("vanish").unwrap();

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
    let (ok, stdout, _stderr) = run_cli("list", dir.path());
    assert!(ok);
    assert!(stdout.contains("myenv"));
    assert_eq!(stdout.matches("(active - outdated)").count(), 1);
    assert_eq!(stdout.matches("(active)").count(), 0);

    // 再次 use 后恢复正常
    run_cli("use myenv", dir.path());
    let (ok, stdout, stderr) = run_cli("list", dir.path());
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("(active)"));
    assert!(!out.contains("outdated"));
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
    let dir = setup_project("{invalid json!!!}");

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
    assert_eq!(read_json(&bak_path), original);
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
    let bak1 = read_json(&bak_path);
    assert_eq!(bak1.get("env").unwrap().get("ANTHROPIC_BASE_URL").unwrap(), "https://original");

    // 第二次 use：备份应为 use a 后的内容，不是原始内容
    run_cli("use b", dir.path());
    let bak2 = read_json(&bak_path);
    assert_eq!(bak2.get("env").unwrap().get("ANTHROPIC_BASE_URL").unwrap(), "https://a");
}

// ============================================================
// state.json 存储测试
// ============================================================

#[test]
fn test_state_migration_from_old_format() {
    let _store = setup_store();
    let store_dir = std::path::PathBuf::from(store_dir_val());

    // 创建旧版 profiles/ 目录和文件
    let profiles_dir = store_dir.join("profiles");
    fs::create_dir_all(&profiles_dir).unwrap();

    let work_env = serde_json::json!({"ANTHROPIC_BASE_URL":"https://work.com","ANTHROPIC_API_KEY":"sk-work"});
    fs::write(profiles_dir.join("work.json"), serde_json::to_string(&work_env).unwrap()).unwrap();

    let home_env = serde_json::json!({"ANTHROPIC_BASE_URL":"https://home.com","ANTHROPIC_API_KEY":"sk-home"});
    fs::write(profiles_dir.join("home.json"), serde_json::to_string(&home_env).unwrap()).unwrap();

    // 创建旧版用户级 current
    fs::write(store_dir.join("current"), "work").unwrap();

    // 创建旧版 project current（用虚拟 hash 目录）
    fs::create_dir_all(store_dir.join("projects").join("abcd1234")).unwrap();
    fs::write(store_dir.join("projects/abcd1234/current"), "home").unwrap();

    // 触发迁移：list_profiles 会调用 read_state → try_migrate
    let profiles = cp_switch::store::list_profiles().unwrap();
    assert_eq!(profiles, vec!["home", "work"]);

    // state.json 已创建
    let state_path = store_dir.join("state.json");
    assert!(state_path.exists());

    // 旧目录已被清理
    assert!(!profiles_dir.exists());
    assert!(!store_dir.join("projects").exists());
    assert!(!store_dir.join("current").exists());

    // state.json 内容正确
    let state_content = read_json(&state_path);
    let profiles_obj = state_content.get("profiles").unwrap().as_object().unwrap();
    assert_eq!(profiles_obj.len(), 2);
    assert_eq!(
        profiles_obj.get("work").unwrap().get("ANTHROPIC_BASE_URL").unwrap(),
        "https://work.com"
    );
    assert_eq!(state_content.get("user_current").unwrap(), "work");
    // project_currents 不迁移（旧版用不可逆的 hash 路径），字段为空则被跳过
    assert!(state_content.get("project_currents").is_none());
}

#[test]
fn test_state_json_corrupted() {
    let _store = setup_store();
    let store_path = store_dir_val();
    let state_path = std::path::Path::new(&store_path).join("state.json");

    // 直接写入非法 JSON
    fs::create_dir_all(state_path.parent().unwrap()).unwrap();
    fs::write(&state_path, "{invalid json!!!}").unwrap();

    // 读取应报错，不 panic
    let result = cp_switch::store::list_profiles();
    assert!(result.is_err());
}

#[test]
fn test_state_json_fresh_creates_default() {
    let _store = setup_store();

    // 干净的目录，无旧格式也无 state.json
    // list_profiles 应返回空列表而不是报错
    let profiles = cp_switch::store::list_profiles().unwrap();
    assert!(profiles.is_empty());

    // state.json 被自动创建
    let store_path = store_dir_val();
    let state_path = std::path::Path::new(&store_path).join("state.json");
    assert!(state_path.exists());

    // 内容是合法的空 state（空 map 被 skip_serializing_if 跳过）
    let state_content = read_json(&state_path);
    assert!(state_content.get("profiles").is_none());
    assert!(state_content.get("project_currents").is_none());
    assert!(state_content.get("user_current").is_none());
}

// ============================================================
// 用户模式（--user）测试
// ============================================================

#[test]
fn test_cli_use_user_creates_settings_and_current() {
    let _store = setup_store();
    let home = setup_home();
    let home_path = home.path().to_path_buf();

    // 先添加 profile
    let (ok, _, stderr) = run_cli_user("add test-user",
        "https://api.test.com\nsk-test\nclaude-sonnet-4\n\n\n\n\n",
        &home_path);
    assert!(ok, "add failed: {}", stderr);

    // use --user
    let input = "y\n"; // 项目模式才需要确认，用户模式不需要
    let (ok, _, stderr) = run_cli_user("use --user test-user", input, &home_path);
    assert!(ok, "use --user failed: {}", stderr);

    // 验证 ~/home/.claude/settings.json 被创建
    assert!(home_path.join(".claude/settings.json").exists());
    let settings = read_user_settings(&home_path);
    let env_obj = get_env_obj(&settings);
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://api.test.com");
    assert_eq!(env_obj.get("ANTHROPIC_AUTH_TOKEN").unwrap(), "sk-test");
    assert_eq!(env_obj.get("ANTHROPIC_MODEL").unwrap(), "claude-sonnet-4");

    // 验证用户级 current 标记
    let current = cp_switch::store::read_user_current().unwrap();
    assert_eq!(current, Some("test-user".to_string()));
}

#[test]
fn test_cli_use_user_reapply() {
    let _store = setup_store();
    let home = setup_home();
    let home_path = home.path().to_path_buf();

    let (ok, _, stderr) = run_cli_user("add user-reapply",
        "https://a.com\nsk-a\nmodel-a\n\n\n\n\n",
        &home_path);
    assert!(ok, "add failed: {}", stderr);

    // 第一次 use --user
    let (ok, _, stderr) = run_cli_user("use --user user-reapply", "", &home_path);
    assert!(ok, "first use failed: {}", stderr);

    // 再次 use --user
    let (ok, stdout, stderr) = run_cli_user("use --user user-reapply", "", &home_path);
    assert!(ok, "reapply failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Switched to profile 'user-reapply' (user)"));

    let settings = read_user_settings(&home_path);
    let env_obj = get_env_obj(&settings);
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://a.com");
}

#[test]
fn test_cli_current_user() {
    let _store = setup_store();
    let home = setup_home();
    let home_path = home.path().to_path_buf();

    // 无活跃时
    let (ok, stdout, stderr) = run_cli_user("current --user", "", &home_path);
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("No active user-level profile"));

    // 创建并 use
    run_cli_user("add user-curr", "https://c.com\nsk-c\nmodel-c\n\n\n\n\n", &home_path);
    run_cli_user("use --user user-curr", "", &home_path);

    // current --user 应显示
    let (ok, stdout, stderr) = run_cli_user("current --user", "", &home_path);
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("user-curr"));
}

#[test]
fn test_cli_list_user_shows_active() {
    let _store = setup_store();
    let home = setup_home();
    let home_path = home.path().to_path_buf();

    cp_switch::store::save_profile("alpha", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://a", "ANTHROPIC_API_KEY": "sk-a", "ANTHROPIC_MODEL": "a"
    })).unwrap();
    cp_switch::store::save_profile("beta", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://b", "ANTHROPIC_API_KEY": "sk-b", "ANTHROPIC_MODEL": "b"
    })).unwrap();

    // use --user alpha
    run_cli_user("use --user alpha", "", &home_path);

    // list --user 应显示 alpha 为 active
    let (ok, stdout, _stderr) = run_cli_user("list --user", "", &home_path);
    assert!(ok);
    // 只有 alpha 标 active，beta 不标
    assert!(stdout.contains("alpha") && stdout.contains("beta"));
    assert_eq!(stdout.matches("(active)").count(), 1);
}

#[test]
fn test_cli_diff_user() {
    let _store = setup_store();
    let home = setup_home();
    let home_path = home.path().to_path_buf();

    cp_switch::store::save_profile("diff-me", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://new", "ANTHROPIC_MODEL": "new"
    })).unwrap();

    // use --user 先设置
    run_cli_user("use --user diff-me", "", &home_path);

    // 更新 profile 使其 outdated
    cp_switch::store::save_profile("diff-me", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://updated", "ANTHROPIC_MODEL": "updated"
    })).unwrap();

    let (ok, stdout, stderr) = run_cli_user("diff --user diff-me", "", &home_path);
    assert!(ok, "diff failed: {}", stderr);
    assert!(stdout.contains("--- user settings"));
    assert!(stdout.contains("+++ profile: diff-me"));
    assert!(stdout.contains("https://updated"));
}

#[test]
fn test_cli_delete_user_active() {
    let _store = setup_store();
    let home = setup_home();
    let home_path = home.path().to_path_buf();

    cp_switch::store::save_profile("user-del", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://d", "ANTHROPIC_MODEL": "d"
    })).unwrap();

    // use --user
    run_cli_user("use --user user-del", "", &home_path);

    // 确认用户级 current 标记存在
    assert_eq!(cp_switch::store::read_user_current().unwrap(), Some("user-del".to_string()));

    // --force 删除
    let (ok, _, stderr) = run_cli_user("delete user-del --force", "", &home_path);
    assert!(ok, "delete failed: {}", stderr);

    // 用户级 current 标记应被清除
    assert!(cp_switch::store::read_user_current().unwrap().is_none());
}

#[test]
fn test_cli_delete_user_active_prompts() {
    let _store = setup_store();
    let home = setup_home();
    let home_path = home.path().to_path_buf();

    cp_switch::store::save_profile("user-prompt", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://e", "ANTHROPIC_MODEL": "e"
    })).unwrap();

    run_cli_user("use --user user-prompt", "", &home_path);

    // 不带 --force，回答 n 取消
    let (ok, stdout, stderr) = run_cli_user("delete user-prompt", "n\n", &home_path);
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Cancelled"));
    // profile 仍存在
    assert!(cp_switch::store::list_profiles().unwrap().contains(&"user-prompt".to_string()));
    // current 标记仍在
    assert_eq!(cp_switch::store::read_user_current().unwrap(), Some("user-prompt".to_string()));
}

#[test]
fn test_cli_use_user_preserves_permissions() {
    let _store = setup_store();
    let home = setup_home();
    let home_path = home.path().to_path_buf();

    // 先创建 ~/home/.claude/settings.json 带 permissions
    setup_user_settings(&home_path, r#"{"permissions":{"allow":["Bash(ls)"]},"env":{"ANTHROPIC_MODEL":"old"}}"#);

    cp_switch::store::save_profile("perm-test", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://p", "ANTHROPIC_API_KEY": "sk-p", "ANTHROPIC_MODEL": "p"
    })).unwrap();

    let (ok, _, stderr) = run_cli_user("use --user perm-test", "", &home_path);
    assert!(ok, "use failed: {}", stderr);

    let settings = read_user_settings(&home_path);
    assert!(settings.get("permissions").is_some());
    let env_obj = get_env_obj(&settings);
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://p");
}

#[test]
fn test_cli_list_user_no_profiles() {
    let _store = setup_store();
    let home = setup_home();
    let home_path = home.path().to_path_buf();

    let (ok, stdout, stderr) = run_cli_user("list --user", "", &home_path);
    assert!(ok);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("No profiles found"));
}

#[test]
fn test_cli_delete_dual_active_clears_both() {
    let _store = setup_store();
    let home = setup_home();
    let home_path = home.path().to_path_buf();

    // 创建 profile 并用 use 和 use --user 分别设置两个上下文的活跃
    cp_switch::store::save_profile("dual", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://dual", "ANTHROPIC_API_KEY": "sk-dual", "ANTHROPIC_MODEL": "dual"
    })).unwrap();

    // 项目级 use
    let project_dir = setup_project(r#"{"env":{}}"#);
    run_cli("use dual", project_dir.path());

    // 用户级 use --user
    run_cli_user("use --user dual", "", &home_path);

    // 两个 current 标记都应存在
    assert_eq!(read_current_canonical(project_dir.path()), Some("dual".to_string()));
    assert_eq!(cp_switch::store::read_user_current().unwrap(), Some("dual".to_string()));

    // --force 删除
    let (ok, stdout, stderr) = run_cli("delete dual --force", project_dir.path());
    assert!(ok, "delete failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("both project and user"));

    // 两个 current 标记都应被清除
    assert!(read_current_canonical(project_dir.path()).is_none());
    assert!(cp_switch::store::read_user_current().unwrap().is_none());
}

#[test]
fn test_cli_use_user_and_project_independent() {
    let _store = setup_store();
    let home = setup_home();
    let home_path = home.path().to_path_buf();

    // 两个 profile：proj-profile 和 user-profile
    cp_switch::store::save_profile("proj-profile", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://proj", "ANTHROPIC_API_KEY": "sk-proj", "ANTHROPIC_MODEL": "proj"
    })).unwrap();
    cp_switch::store::save_profile("user-profile", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://usr", "ANTHROPIC_API_KEY": "sk-usr", "ANTHROPIC_MODEL": "usr"
    })).unwrap();

    // 项目级 use proj-profile
    let project_dir = setup_project(r#"{"env":{}}"#);
    run_cli("use proj-profile", project_dir.path());

    // 用户级 use --user user-profile
    run_cli_user("use --user user-profile", "", &home_path);

    // current 分别显示各自的
    let (_ok, stdout, _) = run_cli("current", project_dir.path());
    let out = combined_output(&stdout, "");
    assert!(out.contains("proj-profile"));

    let (_ok, stdout, _) = run_cli_user("current --user", "", &home_path);
    let out = combined_output(&stdout, "");
    assert!(out.contains("user-profile"));

    // 项目 settings 不应有用户 profile 的数据
    let settings = read_settings(project_dir.path());
    let env_obj = get_env_obj(&settings);
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://proj");

    // 用户 settings 不应有项目 profile 的数据
    let user_settings = read_user_settings(&home_path);
    let user_env_obj = get_env_obj(&user_settings);
    assert_eq!(user_env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://usr");
}

// ============================================================
// 默认 Claude Provider（claude）测试
// ============================================================

#[test]
fn test_cli_add_claude_rejected() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);
    let (ok, _, stderr) = run_cli("add claude", dir.path());
    assert!(!ok);
    // 拒绝理由是「保留名」，不是「名字里有非法字符」
    assert!(stderr.contains("is reserved"), "stderr: {}", stderr);
    assert!(!stderr.contains("Invalid profile name"));
    assert!(stderr.contains("cp-switch use claude"));
}

#[test]
fn test_reserved_and_invalid_name_share_exit_code() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);

    // 对脚本而言两者是同一类「名字不能用」，退出码保持一致
    let reserved = spawn_cli("add claude", "", Some(dir.path()), None);
    let invalid = spawn_cli("add bad!name", "", Some(dir.path()), None);
    assert!(!reserved.0 && !invalid.0);
    assert_eq!(exit_code("add claude", dir.path()), 5);
    assert_eq!(exit_code("add bad!name", dir.path()), 5);

    // claude 本身是合法名字，只是被 cp-switch 占用
    assert!(cp_switch::cli::validate_name("claude").is_ok());
    assert!(cp_switch::cli::ensure_not_reserved("claude").is_err());
}

#[test]
fn test_cli_add_eof_before_required_field() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);

    // stdin 直接 EOF：必填字段拿不到值应立即报错，而不是无限重问
    let (ok, _stdout, stderr) = run_cli("add eof-probe", dir.path());
    assert!(!ok);
    assert!(stderr.contains("Input ended before"), "stderr: {}", stderr);
    assert!(stderr.contains("ANTHROPIC_BASE_URL"));
    // 报错前只提示一次「is required」，没有重试风暴
    assert!(stderr.matches("is required").count() <= 1, "stderr: {}", stderr);
    assert!(cp_switch::store::list_profiles().unwrap().is_empty());
}

#[test]
fn test_cli_add_eof_after_required_fields_keeps_defaults() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);

    // 只喂 3 个必填字段，其余可选字段靠 EOF 走默认值——这是固件依赖的行为
    let (ok, _stdout, stderr) =
        run_cli_stdin("add eof-tail", "https://e.com\nsk-e\nmodel-e\n", dir.path());
    assert!(ok, "add failed: {}", stderr);

    let profile = cp_switch::store::read_profile("eof-tail").unwrap();
    assert_eq!(profile.get("ANTHROPIC_BASE_URL").unwrap(), "https://e.com");
    // 派生模型与 EFFORT 取默认值
    assert_eq!(profile.get("ANTHROPIC_DEFAULT_OPUS_MODEL").unwrap(), "model-e");
    assert_eq!(profile.get("CLAUDE_CODE_EFFORT_LEVEL").unwrap(), "high");
    // 真正可选的 WINDOW 不注入
    assert!(profile.get("CLAUDE_CODE_AUTO_COMPACT_WINDOW").is_none());
}

#[test]
fn test_cli_delete_claude_rejected() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);
    let (ok, _, stderr) = run_cli("delete claude", dir.path());
    assert!(!ok);
    assert!(stderr.contains("is reserved"), "stderr: {}", stderr);
    assert!(!stderr.contains("Invalid profile name"));
}

#[test]
fn test_cli_use_claude_clears_env() {
    let _store = setup_store();
    let dir = setup_project(r#"{"permissions":{"allow":["Bash(ls)"]},"env":{"ANTHROPIC_BASE_URL":"https://old","ANTHROPIC_API_KEY":"sk-old","ANTHROPIC_MODEL":"old","ANTHROPIC_SMALL_FAST_MODEL":"old","API_TIMEOUT_MS":"3000","OTHER":"keep"}}"#);

    let (ok, stdout, stderr) = run_cli("use claude", dir.path());
    assert!(ok, "use claude failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Switched to default Claude"));

    // 验证 managed keys 被清除，非 managed keys 保留
    let settings = read_settings(dir.path());
    let env_obj = get_env_obj(&settings);
    assert!(!env_obj.contains_key("ANTHROPIC_BASE_URL"));
    assert!(!env_obj.contains_key("ANTHROPIC_API_KEY"));
    assert!(!env_obj.contains_key("ANTHROPIC_MODEL"));
    assert!(!env_obj.contains_key("ANTHROPIC_SMALL_FAST_MODEL"));
    assert!(env_obj.contains_key("API_TIMEOUT_MS"));
    assert!(env_obj.contains_key("OTHER"));
    assert_eq!(env_obj.get("API_TIMEOUT_MS").unwrap(), "3000");
    assert_eq!(env_obj.get("OTHER").unwrap(), "keep");

    // permissions 保留
    assert!(settings.get("permissions").is_some());

    // current 标记为 claude
    assert_eq!(read_current_canonical(dir.path()), Some("claude".to_string()));
}

#[test]
fn test_cli_use_claude_reject_create_no_claude_dir() {
    let _store = setup_store();
    let dir = setup_bare_project();

    // 拒绝后不应建出 .claude，也不应把该目录登记进 project_currents
    let (ok, _stdout, stderr) = run_cli_stdin("use claude", "n\n", dir.path());
    assert!(!ok);
    assert!(stderr.contains("当前目录没有 .claude 目录"));
    assert!(!dir.path().join(".claude").exists());
    assert_eq!(read_current_canonical(dir.path()), None);
}

#[test]
fn test_cli_use_claude_user() {
    let _store = setup_store();
    let home = setup_home();
    let home_path = home.path().to_path_buf();

    // 预设用户 settings
    setup_user_settings(&home_path, r#"{"permissions":{"allow":["Bash"]},"env":{"ANTHROPIC_BASE_URL":"https://old","ANTHROPIC_API_KEY":"sk-old","ANTHROPIC_MODEL":"old","OTHER":"keep"}}"#);

    let (ok, stdout, stderr) = run_cli_user("use --user claude", "", &home_path);
    assert!(ok, "use --user claude failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Switched to default Claude"));

    // 验证 managed keys 被清除
    let user_settings = read_user_settings(&home_path);
    let env_obj = get_env_obj(&user_settings);
    assert!(!env_obj.contains_key("ANTHROPIC_BASE_URL"));
    assert!(!env_obj.contains_key("ANTHROPIC_API_KEY"));
    assert!(!env_obj.contains_key("ANTHROPIC_MODEL"));
    assert!(env_obj.contains_key("OTHER"));
    assert_eq!(env_obj.get("OTHER").unwrap(), "keep");

    // permissions 保留
    assert!(user_settings.get("permissions").is_some());

    // 用户级 current 标记为 claude
    assert_eq!(cp_switch::store::read_user_current().unwrap(), Some("claude".to_string()));
}

#[test]
fn test_cli_use_claude_then_switch_back() {
    let _store = setup_store();
    let dir = setup_project(r#"{"permissions":{"allow":["Bash"]},"env":{"ANTHROPIC_BASE_URL":"https://original","ANTHROPIC_API_KEY":"sk-original","ANTHROPIC_MODEL":"original","API_TIMEOUT_MS":"5000"}}"#);

    // 创建 profile
    cp_switch::store::save_profile("work", &serde_json::json!({
        "ANTHROPIC_BASE_URL": "https://work", "ANTHROPIC_API_KEY": "sk-work", "ANTHROPIC_MODEL": "work"
    })).unwrap();

    // 先 use claude 清空
    run_cli("use claude", dir.path());
    let settings = read_settings(dir.path());
    let env_obj = get_env_obj(&settings);
    assert!(!env_obj.contains_key("ANTHROPIC_BASE_URL"));

    // 再切回 work — 非 managed key 仍保留
    let (ok, _, stderr) = run_cli("use work", dir.path());
    assert!(ok, "use work after claude failed: {}", stderr);
    let settings = read_settings(dir.path());
    let env_obj = get_env_obj(&settings);
    assert_eq!(env_obj.get("ANTHROPIC_BASE_URL").unwrap(), "https://work");
    assert_eq!(env_obj.get("API_TIMEOUT_MS").unwrap(), "5000");
}

#[test]
fn test_cli_use_claude_on_clean_project() {
    let _store = setup_store();
    // 新项目连 .claude 目录都没有
    let dir = setup_bare_project();

    // use claude 同样需要确认创建 .claude
    let (ok, stdout, stderr) = run_cli_stdin("use claude", "y\n", dir.path());
    assert!(ok, "use claude on clean project failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Switched to default Claude"));

    // .claude 被创建
    assert!(dir.path().join(".claude/settings.local.json").exists());
    let settings = read_settings(dir.path());
    // 原本没有 env，切到官方直连不应造出空的 env 字段
    assert!(settings.get("env").is_none());

    // current 标记为 claude
    assert_eq!(read_current_canonical(dir.path()), Some("claude".to_string()));
}

#[test]
fn test_cli_list_shows_claude_active() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);

    // use claude
    run_cli("use claude", dir.path());

    // list 应显示 claude 为 active
    let (ok, stdout, stderr) = run_cli("list", dir.path());
    assert!(ok, "list failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("claude"));
    assert!(out.contains("(active)"));
}

#[test]
fn test_cli_current_claude() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);

    run_cli("use claude", dir.path());

    let (ok, stdout, stderr) = run_cli("current", dir.path());
    assert!(ok, "current failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("claude"));
    assert!(out.contains("default Claude provider"));
}

#[test]
fn test_cli_edit_claude_rejected() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);
    let (ok, _, stderr) = run_cli("edit claude", dir.path());
    assert!(!ok);
    assert!(stderr.contains("is reserved"), "stderr: {}", stderr);
    assert!(!stderr.contains("Invalid profile name"));
}

#[test]
fn test_cli_edit_updates_profile() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);

    // 先 add 一个 profile
    let add_input = "https://old.com\nsk-old\nold-model\n\n\n\n\n";
    let (ok, _, stderr) = run_cli_stdin("add edit-me", add_input, dir.path());
    assert!(ok, "add failed: {}", stderr);

    // verify initial values
    let profile = cp_switch::store::read_profile("edit-me").unwrap();
    assert_eq!(profile.get("ANTHROPIC_BASE_URL").unwrap(), "https://old.com");
    assert_eq!(profile.get("ANTHROPIC_AUTH_TOKEN").unwrap(), "sk-old");

    // 编辑为新的值
    let edit_input = "https://new.com\nsk-new\nnew-model\n\n\n\n\n";
    let (ok, stdout, stderr) = run_cli_stdin("edit edit-me", edit_input, dir.path());
    assert!(ok, "edit failed: {}", stderr);
    let out = combined_output(&stdout, &stderr);
    assert!(out.contains("Updated profile 'edit-me'"));

    // 验证值已更新
    let profile = cp_switch::store::read_profile("edit-me").unwrap();
    assert_eq!(profile.get("ANTHROPIC_BASE_URL").unwrap(), "https://new.com");
    assert_eq!(profile.get("ANTHROPIC_AUTH_TOKEN").unwrap(), "sk-new");
    assert_eq!(profile.get("ANTHROPIC_MODEL").unwrap(), "new-model");
}

#[test]
fn test_cli_edit_migrates_api_key_to_auth_token() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);

    // 旧 profile：只有 API_KEY + 一个用户自定义 non-managed key
    cp_switch::store::save_profile("legacy", &serde_json::json!({
        "ANTHROPIC_BASE_URL":"https://old","ANTHROPIC_API_KEY":"sk-old","ANTHROPIC_MODEL":"old",
        "MY_CUSTOM":"keep"
    })).unwrap();

    // edit：base_url 留空沿用旧值，token 填新值
    let edit_input = "\ntok-new\n\n\n\n\n\n";
    let (ok, _, stderr) = run_cli_stdin("edit legacy", edit_input, dir.path());
    assert!(ok, "edit failed: {}", stderr);

    let profile = cp_switch::store::read_profile("legacy").unwrap();
    // 迁移到 AUTH_TOKEN，旧 API_KEY 被丢弃
    assert_eq!(profile.get("ANTHROPIC_AUTH_TOKEN").unwrap(), "tok-new");
    assert!(profile.get("ANTHROPIC_API_KEY").is_none());
    // 非 managed key 仍保留
    assert_eq!(profile.get("MY_CUSTOM").unwrap(), "keep");
}

// ============================================================
// model 命令测试
// ============================================================

#[test]
fn test_cli_model_refused_when_no_active_profile() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);
    // 从未 use 过，无活跃 profile → 拒绝且不修改文件
    let (ok, _, stderr) = run_cli("model claude-fable-5", dir.path());
    assert!(!ok);
    assert!(stderr.contains("仅在当前 profile 为 claude"));
    assert!(read_settings(dir.path()).get("model").is_none());
}

#[test]
fn test_cli_model_refused_on_third_party_profile() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);
    cp_switch::store::save_profile("proxy", &serde_json::json!({
        "ANTHROPIC_BASE_URL":"https://proxy","ANTHROPIC_API_KEY":"sk-x"
    })).unwrap();
    run_cli("use proxy", dir.path());

    // 活跃 profile 是第三方 → 拒绝
    let (ok, _, stderr) = run_cli("model claude-fable-5", dir.path());
    assert!(!ok);
    assert!(stderr.contains("仅在当前 profile 为 claude"));
    assert!(read_settings(dir.path()).get("model").is_none());
}

#[test]
fn test_cli_model_set_and_show_on_claude() {
    let _store = setup_store();
    // 带一个非受管 key，用于验证 model 命令不碰 env
    let dir = setup_project(r#"{"env":{"OTHER":"keep"}}"#);
    // 切到官方直连
    run_cli("use claude", dir.path());

    // 设置 model
    let (ok, stdout, stderr) = run_cli("model claude-fable-5[1m]", dir.path());
    assert!(ok, "model set failed: {}", stderr);
    assert!(combined_output(&stdout, &stderr).contains("claude-fable-5[1m]"));

    // 顶层 model 字段写入，env 不受影响
    let settings = read_settings(dir.path());
    assert_eq!(settings.get("model").unwrap(), "claude-fable-5[1m]");
    assert_eq!(get_env_obj(&settings).get("OTHER").unwrap(), "keep");

    // 无参展示当前值
    let (ok, stdout, stderr) = run_cli("model", dir.path());
    assert!(ok);
    assert!(combined_output(&stdout, &stderr).contains("claude-fable-5[1m]"));
}

#[test]
fn test_cli_model_clear() {
    let _store = setup_store();
    let dir = setup_project(r#"{"env":{}}"#);
    run_cli("use claude", dir.path());
    run_cli("model claude-opus-4-8", dir.path());
    assert!(read_settings(dir.path()).get("model").is_some());

    let (ok, stdout, stderr) = run_cli("model --clear", dir.path());
    assert!(ok, "model clear failed: {}", stderr);
    assert!(combined_output(&stdout, &stderr).contains("Cleared model"));
    assert!(read_settings(dir.path()).get("model").is_none());
}

#[test]
fn test_cli_model_user() {
    let _store = setup_store();
    let home = setup_home();
    let home_path = home.path().to_path_buf();

    // 用户级切到官方直连
    run_cli_user("use --user claude", "", &home_path);

    let (ok, _, stderr) = run_cli_user("model --user claude-sonnet-5", "", &home_path);
    assert!(ok, "model --user failed: {}", stderr);

    let settings = read_user_settings(&home_path);
    assert_eq!(settings.get("model").unwrap(), "claude-sonnet-5");
}