# cp-switch

CLI 工具，切换 Claude Code 的 API 连接配置。

在不同项目间快速切换 `ANTHROPIC_BASE_URL`、`ANTHROPIC_API_KEY` 等环境变量，无需手动编辑 `.claude/settings.local.json`。

## 为什么需要它？

Claude Code 通过 `.claude/settings.local.json` 的 `env` 字段读取 API 连接配置。如果你：

- 在多个 API 代理（官方、Azure、AWS Bedrock、自建代理）间切换
- 在不同团队/项目间使用不同 API Key
- 需要临时切换模型测试行为

每次都要手动编辑 JSON 文件，容易出错、遗漏 key、忘记清除旧值。`cp-switch` 把这些配置存为 profile，一键切换，自动处理互斥变量。

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

### 场景 1：官方 API ↔ 自建代理切换

你日常工作用自建代理（更便宜/合规），偶尔需要用官方 API 测试新模型行为。

```bash
# 创建两个 profile
cp-switch add proxy    # 自建代理：BASE_URL=https://my-proxy.example.com, API_KEY=sk-proxy-xxx
cp-switch add official # 官方 API：BASE_URL=https://api.anthropic.com, API_KEY=sk-ant-xxx

# 日常用代理
cp-switch use proxy

# 临时切到官方测试新模型
cp-switch use official

# 测试完切回来
cp-switch use proxy
```

### 场景 2：多项目多团队

不同项目用不同团队的 API Key，项目配置互不干扰（每个项目有自己的 `current` 标记）。

```bash
# 项目 A 用团队 alpha 的 key
cd ~/projects/project-a
cp-switch add alpha   # 团队 alpha 的 API 配置
cp-switch use alpha

# 项目 B 用团队 beta 的 key
cd ~/projects/project-b
cp-switch use beta    # 团队 beta 的 API 配置

# 回到项目 A，仍然是 alpha
cd ~/projects/project-a
cp-switch current     # → alpha
```

### 场景 3：切换模型配置

同一个代理，但需要不同模型配置（如 Opus 做深度分析，Haiku 做批量处理）。

```bash
cp-switch add opus-mode   # MODEL=claude-opus-4-7, 小模型也用 opus
cp-switch add haiku-mode  # MODEL=claude-haiku-4-5, 小模型用 haiku

cp-switch use opus-mode   # 深度分析时
cp-switch use haiku-mode  # 批量处理时
```

### 场景 4：从 OAuth 登录切换到 API Key

你通过 `claude login` 用了官方 OAuth 登录（生成 `AUTH_TOKEN`），后来想切换到自建代理（需要 `API_KEY`）。

```bash
cp-switch use proxy
# 自动清除 AUTH_TOKEN，写入 API_KEY——两种认证方式互斥，不会冲突
```

### 场景 5：预览变更再切换

不确定某个 profile 的配置和当前环境有什么差异，先看 diff。

```bash
cp-switch diff staging
# 输出彩色 diff，展示哪些变量会改变、哪些会新增、哪些会清除

# 确认后再切换
cp-switch use staging
```

## 命令详解

### add — 创建配置

```bash
cp-switch add <name>        # 交互式输入
cp-switch add <name> --force # 覆盖已有配置
```

交互式输入必填项：
- `ANTHROPIC_BASE_URL` — API 代理地址
- `ANTHROPIC_API_KEY` — API Key
- `ANTHROPIC_MODEL` — 主模型 ID

可选自定义（默认从 MODEL 自动推导）：
- `ANTHROPIC_SMALL_FAST_MODEL`
- `ANTHROPIC_DEFAULT_HAIKU_MODEL`
- `ANTHROPIC_DEFAULT_SONNET_MODEL`
- `ANTHROPIC_DEFAULT_OPUS_MODEL`

### use — 切换配置

```bash
cp-switch use <name>
```

行为：
1. 清除 `env` 中所有 `ANTHROPIC_*` key
2. 写入 profile 的 key
3. 处理互斥认证变量（见下方「认证冲突处理」）
4. 不影响 `permissions` 和其他非 `ANTHROPIC_*` 变量
5. 项目无 `settings.local.json` 时自动创建
6. 写入前备份到 `.claude/settings.local.json.bak`

切换后输出变更摘要：

```
✓ Switched to profile 'proxy'
  ANTHROPIC_BASE_URL = https://my-proxy.example.com
  ANTHROPIC_API_KEY = sk-proxy-xxx
  - removed ANTHROPIC_AUTH_TOKEN
```

### current — 查看当前配置

```bash
cp-switch current   # 别名: show
```

### list — 列出所有配置

```bash
cp-switch list      # 别名: ls
```

标记当前活跃 profile。如果活跃 profile 文件已被删除，显示 `missing`。

### diff — 查看差异

```bash
cp-switch diff <name>
```

对比当前环境变量与指定 profile 的彩色文本 diff。

### delete — 删除配置

```bash
cp-switch delete <name>          # 活跃时提示确认
cp-switch delete <name> --force  # 跳过确认
```

别名：`rm`

## 认证冲突处理

Claude Code 支持两种互斥认证方式：

| 变量 | 用途 |
|---|---|
| `ANTHROPIC_API_KEY` | 直接用 API Key 认证 |
| `ANTHROPIC_AUTH_TOKEN` | OAuth Token 认证（如 `claude login` 生成的） |

`use` 命令不会删除所有 `ANTHROPIC_*` 变量，只删除冲突组内的互斥 key：
- profile 含 `API_KEY` → 清除 settings 中已有的 `AUTH_TOKEN`
- profile 含 `AUTH_TOKEN` → 清除 settings 中已有的 `API_KEY`
- 其他 `ANTHROPIC_*` 变量（`MODEL`、`BASE_URL` 等）不受影响

## 存储

| 数据 | 位置 |
|---|---|
| Profile | `~/.cp-switch/profiles/<name>.json` |
| 当前标记 | `~/.cp-switch/projects/<fnv1a-hash>/current` |
| 项目配置 | `<project>/.claude/settings.local.json` 的 `env` 字段 |

- `CP_SWITCH_DIR` 环境变量可覆盖根目录
- 每个 profile 仅存储 `ANTHROPIC_*` 变量，不含其他配置
- 当前标记按项目目录哈希隔离，不同项目可同时使用不同 profile

## 命令别名

| 命令 | 别名 |
|---|---|
| `list` | `ls` |
| `current` | `show` |
| `delete` | `rm` |

## License

MIT