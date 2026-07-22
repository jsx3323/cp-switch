# cp-switch

Rust CLI 工具，切换 Claude Code 的 API 连接配置。

## 项目结构

```
src/
  cli.rs          — clap 命令定义 + validate_name
  main.rs         — 命令分发（find_project_dir 集中调用）+ 错误处理
  lib.rs          — 模块导出
  error.rs        — CsError 枚举（8 变体）+ io_err/json_err/serialization_err
  input.rs        — 交互式输入（prompt_required/prompt_with_default/prompt_optional→Option/prompt_confirm）
  output.rs       — 终端彩色输出（含 diff 渲染和 ListStatus 枚举）
  store/
    mod.rs        — 显式 re-export（不含 validate_name）
    keys.rs       — KEY_* 常量（11 个）+ is_claude_env_key + derive_default_models + CONFLICT_GROUPS
    path.rs       — 路径构造 + find_project_dir + simple_hash（pub(crate）内部函数）
    io.rs         — 文件 CRUD（profile/current/settings 读写）+ read_current_env（settings 不存在时返回默认空值）+ 原子写入（write→tmp→rename）+ settings 备份
    merge.rs      — merge_env 纯函数（不读写文件）
  command/
    prompt.rs     — 共享 prompt_profile_env（add/edit 共用字段提示）
    add.rs        — 交互式创建 profile
    use_profile.rs — 切换配置（IO 编排：read→merge→write，含 .claude 目录检查）
    list.rs       — 列出 profiles + 活跃标记（ListStatus::Active/Outdated/Missing/Inactive）
    current.rs    — 显示当前 profile
    delete.rs     — 删除 profile（活跃时需确认）
    diff.rs       — 当前 env 与 profile 的文本 diff（含 .claude 目录检查）
    edit.rs       — 编辑已有 profile（保留非标准 key）
tests/
  integration.rs  — 单元/纯函数测试 + CLI 子进程测试 + 错误路径 + 端到端行为测试
```

## 编码约定

- 环境变量 key 用 `KEY_*` 常量（store/keys.rs 中 7 个），不硬编码字符串
- 文件操作 TOCTOU-free：直接操作 + `match` NotFound，不先 `exists()` 再操作
- 写入操作原子性：先写临时文件 → `fs::rename`，防止半写损坏
- `write_settings_local` 备份已有文件到 `.claude/settings.local.json.bak`，不自动清理
- 注释只写非显而易见的 WHY，不写 WHAT
- 错误用 CsError 枚举 + exit_code/hint，不 println 后 exit
- 命令模块签名：`pub fn run(..., project: &Path) -> Result<(), CsError>`（add 除外）
- `colored` 仅在 output.rs 中导入，其他模块通过 output 函数使用
- 交互式输入通过 input.rs，不直接调用 stdin
- 序列化内存 Value 用 `serialization_err`，解析文件 JSON 用 `json_err`
- validate_name 在 cli.rs（命令层关注点），不在 store
- merge_env 是纯函数（不读写文件），命令层负责 IO 编排
- store 公共 API 通过 mod.rs 显式 re-export，内部函数 pub(crate)

## 存储

- Profile: `~/.cp-switch/profiles/<name>.json`（仅含 ANTHROPIC_* env vars）
- Current marker: `~/.cp-switch/projects/<fnv1a-hash>/current`
- Settings: 项目 `.claude/settings.local.json` 的 `env` 字段
- `CP_SWITCH_DIR` 环境变量可覆盖根目录
- 跨平台 home 目录通过 `dirs` crate

## use 行为

先清除 `env` 中所有 `ANTHROPIC_*` key，再写入 profile 的 key。非 ANTHROPIC_* env 和 permissions 不受影响。项目无 `settings.local.json` 时自动创建。

## 测试

```bash
cargo test -- --test-threads=1
```

`--test-threads=1` 必须因为 `std::env::set_var` 需要单线程。测试通过 `CP_SWITCH_DIR` + tempfile 隔离，不操作真实环境。