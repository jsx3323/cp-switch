# cp-switch

Rust CLI 工具，切换 Claude Code 的 API 连接配置。

## 项目结构

```
src/
  cli.rs          — clap 命令定义 + validate_name/ensure_not_reserved/validate_profile_arg
  main.rs         — 命令分发（Scope::from_flag 集中构造作用域）+ 错误处理
  lib.rs          — 模块导出
  error.rs        — CsError 枚举 + io_err/json_err/serialization_err
  input.rs        — 交互式输入（prompt_required/prompt_with_default/prompt_optional→Option/prompt_confirm，共用 read_trimmed）
  output.rs       — 终端彩色输出（render_diff 渲染差异 + ListStatus 枚举）
  store/
    mod.rs        — 显式 re-export（不含 validate_name）
    keys.rs       — KEY_* 常量（11 个 env key）+ MODEL_FIELD/ENV_FIELD（settings 顶层字段）+ is_claude_env_key + derive_default_models
    state.rs      — State 结构（profiles / project_currents / user_current，单文件 state.json 的内存表示）
    path.rs       — 路径构造（state_path/settings_local_path/user_settings_path）+ find_project_dir
    scope.rs      — Scope 枚举（Project/User）：settings 文件与活跃标记的层级差异只在此表达一次
    io.rs         — state.json CRUD（读写走 read_state/update_state）+ 旧文件夹格式自动迁移（try_migrate）+ settings 读写（read/write_settings_file 按路径复用）+ 原子写入（write→tmp→rename）+ settings 备份
    merge.rs      — env/model 纯函数（不读写文件）：merge_env（clear_env 是其空参特例）/managed_env/is_env_applied/get_model/set_model/clear_model
  command/
    mod.rs        — 模块导出 + 跨命令共享的 scope_label / ensure_claude_dir
    prompt.rs     — 共享 prompt_profile_env（add/edit 共用字段提示；鉴权字段收集 ANTHROPIC_AUTH_TOKEN）
    add.rs        — 交互式创建 profile
    use_profile.rs — 切换配置（IO 编排：read→merge→write，含 .claude 目录检查）
    list.rs       — 列出 profiles + 活跃标记（ListStatus::Active/Outdated/Missing/Inactive）
    current.rs    — 显示当前 profile
    delete.rs     — 删除 profile（活跃时需确认，同时清理两级活跃标记）
    diff.rs       — 当前 env 与 profile 的文本 diff（含 .claude 目录检查）
    edit.rs       — 编辑已有 profile（保留非标准 key）
    model.rs      — 设置/查看顶层 model 字段（门控：仅当前 profile 为 claude 时可用）
tests/
  integration.rs  — 单元/纯函数测试 + CLI 子进程测试 + 错误路径 + 端到端行为测试
```

## 编码约定

- 环境变量 key 用 `KEY_*` 常量（store/keys.rs 中 11 个），不硬编码字符串
- 文件操作 TOCTOU-free：直接操作 + `match` NotFound，不先 `exists()` 再操作
- 写入操作原子性：先写临时文件 → `fs::rename`，防止半写损坏
- `write_settings_local` 备份已有文件到 `.claude/settings.local.json.bak`，不自动清理
- 注释只写非显而易见的 WHY，不写 WHAT
- 错误用 CsError 枚举 + exit_code/hint，不 println 后 exit
- 命令模块签名：`pub fn run(..., scope: &Scope) -> Result<(), CsError>`；不写成对的 `run`/`run_user`，
  项目级与用户级的差异一律经 `Scope` 表达（add/edit 与作用域无关，delete 同时作用于两级故仍取 `&Path`）
- `colored` 仅在 output.rs 中导入，其他模块通过 output 函数使用
- 交互式输入通过 input.rs，不直接调用 stdin
- 序列化内存 Value 用 `serialization_err`，解析文件 JSON 用 `json_err`
- validate_name 在 cli.rs（命令层关注点），不在 store
- merge.rs 全是纯函数（不读写文件），命令层负责 IO 编排；env 语义的判断（受管 key 过滤、
  是否已生效）也放这里，不散落到命令层
- store 公共 API 通过 mod.rs 显式 re-export，内部函数 pub(crate)

## 存储

- 单文件 `~/.cp-switch/state.json`，结构见 store/state.rs：
  - `profiles`: `{ <name>: { env kv } }`（仅含受管 env vars）
  - `project_currents`: `{ <项目绝对路径>: <profile 名> }`（用完整路径作 key，无 hash）
  - `user_current`: `Option<String>`（`--user` 级活跃 profile）
- 旧的文件夹格式（`profiles/<name>.json` + `projects/<hash>/current` + `current`）在首次读取 state.json 缺失时由 `try_migrate` 自动迁入并清理
- Settings: 项目 `.claude/settings.local.json` 的 `env` 字段；用户级为 `~/.claude/settings.json`
- `CP_SWITCH_DIR` 环境变量可覆盖根目录
- 跨平台 home 目录通过 `dirs` crate

## use 行为

先清除 `env` 中所有 `ANTHROPIC_*` key，再写入 profile 的 key。非 ANTHROPIC_* env 和 permissions 不受影响。项目无 `settings.local.json` 时自动创建。

`use claude`（内置名）是「受管 env 为空」的退化 profile，走同一条 `read→merge_env→write`：只清不写，
清空受管 key 后若 `env` 变空则连字段一起删除，原本没有 `env` 时也不会造出 `"env": {}`。
`.claude` 目录缺失时与普通 profile 一样先确认再创建。

profile 不存在时先报 `ProfileNotFound`，不会为一个不存在的 profile 去问「是否新建 .claude」。

## model 行为

`cp-switch model [value] [--clear] [--user]` 操作 settings 顶层 `model` 字段（CC 原生模型选择器，区别于 `env.ANTHROPIC_MODEL`）。

门控：仅当前 profile 为内置 `claude`（官方直连）时可用——无活跃 profile 或第三方 profile 时报错 `ModelRequiresClaude` 并不改文件。理由：顶层 model 只对官方 API 有意义。无参显示当前值，`--clear` 删除该字段。与 profile 系统正交（`use` 不动 model，`model` 不动 env），value 只校验非空，具体 id 交给 CC。

## 测试

```bash
cargo test -- --test-threads=1
```

`--test-threads=1` 必须因为 `std::env::set_var` 需要单线程。测试通过 `CP_SWITCH_DIR` + tempfile 隔离，不操作真实环境。

## 分支与提交

分支模型是 **GitHub Flow**，合并统一用 squash。文档订正一类的小改动可以直接提交到 main。

- **分支名**：`<类型>/<英文短描述>`，小写连字符，如 `feat/model-command`
- **提交信息**：`<类型>: <中文描述>`，类型取 `feat` / `fix` / `refactor` / `docs` / `chore` / `test`
- **不带范围**：本仓库既有历史一律无 `(<范围>)`，沿用即可，别单独引入
