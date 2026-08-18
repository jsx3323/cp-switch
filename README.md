# cp-switch

CLI 工具，切换 Claude Code 的 API 连接配置。

在不同项目间快速切换 `ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN`、模型等环境变量，无需手动编辑 `.claude/settings.local.json`。

## 为什么需要它？

Claude Code 通过 `.claude/settings.local.json` 的 `env` 字段读取 API 连接配置。如果你：

- 在多个 API 代理（官方直连、自建代理、第三方中转）间切换
- 在不同团队/项目间使用不同凭据
- 需要临时切换模型或思考强度测试行为

每次都要手动编辑 JSON 文件，容易出错、遗漏 key、忘记清除旧值。`cp-switch` 把这些配置存为 profile，一键切换，切换时先清干净再写入。

## 安装

```bash
cargo install cp-switch
```

或从源码构建：

```bash
git clone https://github.com/jsx3323/cp-switch.git
cd cp-switch
cargo build --release
```

## 使用场景

### 场景 1：官方直连 ↔ 自建代理切换

日常用自建代理（更便宜/合规），偶尔切回官方直连。官方直连是内置 profile `claude`，不需要创建。

```bash
cp-switch add proxy   # 自建代理：BASE_URL + AUTH_TOKEN + 模型

cp-switch use proxy   # 日常用代理
cp-switch use claude  # 切回官方直连：清空全部受管 env，让 CC 用自己的登录态
cp-switch use proxy   # 再切回来
```

### 场景 2：多项目多团队

不同项目用不同团队的凭据，项目之间互不干扰——活跃标记按项目目录记录。

```bash
cd ~/projects/project-a
cp-switch add alpha
cp-switch use alpha

cd ~/projects/project-b
cp-switch use beta

cd ~/projects/project-a
cp-switch current     # → alpha
```

活跃标记以**项目根目录**（运行命令时的工作目录）为 key，在子目录里运行属于另一个作用域。

### 场景 3：切换模型配置

同一个代理，不同模型组合（如 Opus 做深度分析，Haiku 做批量处理）。

```bash
cp-switch add opus-mode   # MODEL=claude-opus-5，小模型也用 opus
cp-switch add haiku-mode  # MODEL=claude-haiku-4-5

cp-switch use opus-mode
cp-switch use haiku-mode
```

### 场景 4：全局默认 + 项目覆盖

不带 `--user` 写项目 `.claude/settings.local.json`；带 `--user` 写 `~/.claude/settings.json`，作为所有项目的默认。

```bash
cp-switch use proxy --user   # 全局默认走代理
cd ~/projects/work
cp-switch use alpha          # 这个项目单独用 alpha（项目级优先）
cp-switch current --user     # → proxy
```

### 场景 5：预览变更再切换

```bash
cp-switch diff staging   # 彩色 diff：哪些变量会改变、新增、清除
cp-switch use staging
```

## 命令详解

### add / edit — 创建与编辑配置

```bash
cp-switch add <name>          # 交互式输入
cp-switch add <name> --force  # 覆盖已有配置
cp-switch edit <name>         # 以当前值为默认重新过一遍字段
```

profile 与作用域无关（同一份配置既可用于项目级也可用于用户级），因此这两个命令没有 `--user`。

必填（`edit` 时空输入沿用旧值）：

- `ANTHROPIC_BASE_URL` — API 地址
- `ANTHROPIC_AUTH_TOKEN` — 鉴权凭据
- `ANTHROPIC_MODEL` — 主模型 id

可选（回车接受括号里的默认值）：

| 字段 | 默认值 |
|---|---|
| `ANTHROPIC_SMALL_FAST_MODEL` | 同 `ANTHROPIC_MODEL` |
| `ANTHROPIC_DEFAULT_HAIKU_MODEL` | 同 `ANTHROPIC_MODEL` |
| `ANTHROPIC_DEFAULT_SONNET_MODEL` | 同 `ANTHROPIC_MODEL` |
| `ANTHROPIC_DEFAULT_OPUS_MODEL` | 同 `ANTHROPIC_MODEL` |
| `CLAUDE_CODE_SUBAGENT_MODEL` | 继承 `ANTHROPIC_SMALL_FAST_MODEL` |
| `CLAUDE_CODE_EFFORT_LEVEL` | `high` |
| `CLAUDE_CODE_AUTO_COMPACT_WINDOW` | 无——跳过就不写入这个 key |

`edit` 会保留你手动加在 profile 里的非受管 key；受管 key 一律按这轮输入重建。

保留名：`claude` 是内置 profile，`add` / `edit` / `delete` 拒绝这个名字。

### use — 切换配置

```bash
cp-switch use <name>
cp-switch use <name> --user   # 写 ~/.claude/settings.json
cp-switch use claude          # 内置：只清不写，回到 CC 自己的登录态
```

行为：

1. 清除 `env` 中全部[受管 key](#受管的-env-key)（不止认证那两个）
2. 写入 profile 的全部 key
3. 非受管 env 变量、`permissions` 等其他字段不受影响
4. 清完后 `env` 变空则删掉整个 `env` 字段，不留 `"env": {}`
5. settings 文件不存在时自动创建；项目没有 `.claude` 目录时先征求确认
6. 写入前备份到 `.claude/settings.local.json.bak`（不自动清理）

profile 不存在时直接报错，不会为一个不存在的 profile 去问要不要新建 `.claude`。

切换后输出变更摘要：

```
✓ Switched to profile 'proxy'
  ANTHROPIC_BASE_URL = https://my-proxy.example.com
  ANTHROPIC_AUTH_TOKEN = sk-proxy-xxx
  ANTHROPIC_MODEL = claude-opus-5
  - removed ANTHROPIC_API_KEY
```

### current — 查看当前配置

```bash
cp-switch current          # 别名: show
cp-switch current --user
```

### list — 列出所有配置

```bash
cp-switch list             # 别名: ls
cp-switch list --user
```

每个 profile 标注状态：

| 状态 | 含义 |
|---|---|
| `active` | 活跃，且 settings 里的值与 profile 一致 |
| `outdated` | 标记为活跃，但 settings 已被改动，与 profile 不一致 |
| `missing` | 活跃标记指向一个已被删除的 profile |
| （无标注） | 未活跃 |

### diff — 查看差异

```bash
cp-switch diff <name>
cp-switch diff <name> --user
```

对比当前 settings 里的受管 env 与指定 profile。纯只读：settings 或 `.claude` 缺失时按空 env 比较，不会创建任何文件、也不会询问。

### delete — 删除配置

```bash
cp-switch delete <name>          # 别名: rm；活跃时提示确认
cp-switch delete <name> --force  # 跳过确认
```

同时清理项目级与用户级的活跃标记，因此没有 `--user`。已写入 settings 的值不会被回滚。

### model — 设置 CC 原生模型选择器

```bash
cp-switch model                      # 显示当前值
cp-switch model claude-opus-5        # 设置
cp-switch model --clear              # 清除，回退 CC 默认
cp-switch model <value> --user
```

操作 settings **顶层** `model` 字段（Claude Code 自己的模型选择器），与 `env.ANTHROPIC_MODEL` 是两回事。

门控：仅当前 profile 为内置 `claude` 时可用——顶层 `model` 只对官方 API 有意义，无活跃 profile 或使用第三方 profile 时报错且不改文件。与 profile 系统正交：`use` 不动 `model`，`model` 不动 `env`。value 只校验非空，具体 id 交给 CC 判断。

## 受管的 env key

`cp-switch` 只认这 11 个 key：写入时只写它们，`use` 清除时也只清它们。

```
ANTHROPIC_BASE_URL                ANTHROPIC_DEFAULT_HAIKU_MODEL
ANTHROPIC_API_KEY                 ANTHROPIC_DEFAULT_SONNET_MODEL
ANTHROPIC_AUTH_TOKEN              ANTHROPIC_DEFAULT_OPUS_MODEL
ANTHROPIC_MODEL                   CLAUDE_CODE_SUBAGENT_MODEL
ANTHROPIC_SMALL_FAST_MODEL        CLAUDE_CODE_EFFORT_LEVEL
                                  CLAUDE_CODE_AUTO_COMPACT_WINDOW
```

`ANTHROPIC_API_KEY` 和 `ANTHROPIC_AUTH_TOKEN` 在实际使用中是同一个凭据字段，所以交互式输入只问 `AUTH_TOKEN`；settings 里原有的 `API_KEY` 会在 `use` 时一并清除，两者不会残留共存。

不在这张表里的 env 变量（以及 `permissions` 等其他 settings 字段）`cp-switch` 完全不碰。

## 存储

| 数据 | 位置 |
|---|---|
| 全部状态 | `~/.cp-switch/state.json` |
| 项目配置 | `<项目>/.claude/settings.local.json` 的 `env` 字段 |
| 用户配置 | `~/.claude/settings.json` 的 `env` 字段 |

`state.json` 结构：

```json
{
  "profiles": { "proxy": { "ANTHROPIC_BASE_URL": "...", "...": "..." } },
  "project_currents": { "/Users/me/projects/app": "proxy" },
  "user_current": "claude"
}
```

- `CP_SWITCH_DIR` 环境变量可覆盖根目录（`~/.cp-switch`）
- profile 只存受管 env key，不含 settings 的其他内容
- `project_currents` 用项目绝对路径作 key，不做哈希
- 写盘一律先写临时文件再 `rename`，中断不会留下半写的文件
- 旧版的文件夹格式（`profiles/<name>.json` + `projects/<hash>/current`）在首次运行时自动迁移并清理

## 退出码

| 码 | 含义 |
|---|---|
| 1 | profile 不存在 |
| 2 | profile 已存在（需要 `--force`） |
| 4 | 无活跃 profile |
| 5 | profile 名非法或是保留名 |
| 6 / 7 / 8 / 9 | I/O 错误 / JSON 解析失败 / settings 结构不合法 / 序列化失败 |
| 10 | 没有 `.claude` 目录且用户拒绝创建 |
| 11 | `model` 命令要求当前 profile 为 `claude` |
| 12 | model 值为空 |
| 13 | 非交互运行时 stdin 提前结束，必填字段没喂全 |

## 命令别名

| 命令 | 别名 |
|---|---|
| `list` | `ls` |
| `current` | `show` |
| `delete` | `rm` |

## License

MIT
