# Microsoft Email Manager Desktop · 桌面端

本地化的 Microsoft 邮箱账户与邮件管理桌面应用。由 [Microsoft-Email-Manager](https://github.com/Maishan-Inc/Microsoft-Email-Manager)（FastAPI Web 版）重构为 **Tauri + Rust + Svelte** 单机桌面端。

## 为什么是这套技术栈

针对 5 个核心诉求做的选型：

| 诉求 | 实现 |
|------|------|
| **数据保存在本地** | 所有数据存于系统应用数据目录下的加密 SQLite（`mem-desktop.db`），不连任何后端服务器 |
| **账号凭据加密** | `refresh_token` / `client_id` 用 **AES-256-GCM** 加密；密钥由 **Argon2id** 从主密码派生，主密码不落盘、无法找回 |
| **支持导出** | 账号可导出为 JSON / CSV，可选「含凭据」并强制整体加密导出 |
| **内存占用低** | Tauri 用系统 WebView（非打包 Chromium），实测主进程约 **47MB**（Electron 通常 120MB+）；Rust 后端零 GC |
| **三端开发简单** | Tauri 原生支持 Windows / macOS / Linux 打包；HTTP 用 rustls（免 OpenSSL），仅 Linux 的 IMAP 用到 native-tls |

## 功能

- 🧭 **左侧栏应用壳**（首页 / 邮箱账户管理 / 添加邮箱 / 分类编辑 / 系统设置 + 锁定），DESIGN.md 浅/深双主题、中英文切换
- 🚀 **首启引导向导**：弹出动画 → 用户协议（滚动到底才可同意）→ 配置主密码 → 可选 2FA → 认证模式 → 恢复助记词（生成/下载/隐藏校验）→ 第一次使用教程
- 🔐 主密码解锁（密钥包装模型，见「安全模型」）；可选 **TOTP 两步验证**；忘记密码可用**助记词恢复**并重设
- 📊 **仪表盘**：邮箱数量、邮箱健康度、当日接收邮件、最近活动
- 👤 账号管理：横条列表，点击进入查看邮件；单个添加（先连接测试再入库）、批量导入、删除
- 🔔 **每账号通知**：可单独开启「新邮件系统通知」，开启后后台自动轮询刷新
- 📥 批量导入兼容多格式：
  - IMAP：`邮箱----刷新令牌----客户端ID`（兼容旧 `邮箱----占位密码----刷新令牌----客户端ID`）
  - Graph：`邮箱----密码----client_id----令牌`
- 📧 两种接入方式取邮件：**IMAP**（OAuth2 + XOAUTH2）/ **Microsoft Graph API**
- 📨 收件箱 / 垃圾箱 / 全部，邮件列表 + 详情（纯文本/HTML 安全沙箱渲染）
- 🩺 账号健康检查（OAuth 刷新 + 协议探测）
- 📤 导出 JSON / CSV（可选含凭据 + 加密）
- 🏷️ 分类（category）/ 标签（tag）编辑

## 架构

```
src/                      前端（Svelte 5 + TS + Vite）
├─ App.svelte             应用壳：侧栏 + 视图路由 + 引导门控
├─ components/
│  ├─ Sidebar.svelte      左侧导航 + 锁定
│  ├─ Dashboard.svelte    控制面板（邮箱数/健康度/当日邮件/最近）
│  ├─ Accounts.svelte     账号横条 + 通知开关 + 点击进邮件
│  ├─ AddEmail.svelte     单个添加 / 批量导入
│  ├─ Categories.svelte   分类 / 标签编辑
│  ├─ Settings.svelte     语言 / 主题 / 后台刷新 / 导出 / 关于
│  ├─ Emails.svelte       邮件列表 + 详情
│  ├─ Unlock.svelte       解锁 / 2FA / 助记词恢复
│  └─ onboarding/OnboardingWizard.svelte  首启引导向导
├─ lib/{api,types,toast,i18n,theme}  invoke 封装 / 类型 / 提示 / 中英文 / 主题
src-tauri/src/            后端（Rust）
├─ crypto.rs              Argon2id 派生 + AES-256-GCM + DEK 密钥包装
├─ security.rs            TOTP(RFC6238) / Base32 / 二维码 / BIP39 助记词
├─ db.rs                  rusqlite（bundled）模型与迁移（含 mail_activity）
├─ state.rs              解锁态 Vault + pending(待 2FA)，锁定时 zeroize 清零
├─ background.rs          后台轮询新邮件 + 系统通知
├─ accounts_auth.rs       Microsoft OAuth2 token 刷新
├─ accounts.rs            批量导入解析 / 连接测试
├─ imap_client.rs         IMAP XOAUTH2 取邮件（spawn_blocking）
├─ graph.rs               Graph API 取邮件
├─ export.rs              JSON / CSV 导出
└─ commands.rs            #[tauri::command] 暴露给前端
```

## 安全模型

- **密钥包装（key-wrapping）**：随机生成 32 字节数据密钥（DEK），真正用于加密账号字段（AES-256-GCM，`base64(nonce(12) || ct+tag)`）。
- DEK 被多把 KEK 各包装一份落盘：
  - 主密码 → Argon2id（64MiB / 3 轮）→ `KEK_pw` → `dek_wrapped_pw`
  - 可选恢复助记词（BIP39）→ Argon2id → `KEK_mn` → `dek_wrapped_mn`
  - 解包用 GCM 认证标签校验，失败即密码 / 助记词错误。
- **两步验证（可选 TOTP）**：密钥用 DEK 加密存储；启用后解锁需「主密码 → 解出 DEK → 校验 6 位动态码」才放行。
- 解锁期间 DEK 常驻内存，锁定 / 退出时 `zeroize` 清零。
- **恢复**：忘记主密码时用助记词解出 DEK 并重设密码；未配置助记词则无法找回（本地加密的代价，也是安全所在）。
- 加密导出文件以 `MEMENC1` 开头，可在本应用内解密恢复。

## 开发

前置：Node ≥ 18、pnpm、Rust ≥ 1.77。

```bash
pnpm install
pnpm tauri dev      # 开发模式（热重载）
```

类型检查 / 前端构建：

```bash
pnpm check
pnpm build
```

## 三端打包

```bash
pnpm tauri build
```

产物在 `src-tauri/target/release/bundle/`。

各平台前置依赖：

| 平台 | 前置 |
|------|------|
| **Windows** | WebView2 运行时（Win10/11 多已内置）；产出 `.msi` / `.exe(NSIS)` |
| **macOS** | Xcode Command Line Tools；产出 `.app` / `.dmg`（原生 TLS 用 SecureTransport，无需 OpenSSL） |
| **Linux** | `webkit2gtk`、`libssl-dev`（IMAP 的 native-tls 需要）；产出 `.deb` / `.AppImage` / `.rpm` |

> HTTP（token 刷新 / Graph）统一用 rustls，免 OpenSSL；仅 IMAP 的 native-tls 在 Linux 需 `libssl-dev`。

### GitHub Actions 三端自动构建（可选）

可用官方 `tauri-apps/tauri-action` 在 `windows-latest` / `macos-latest` / `ubuntu-latest` 矩阵中一键出三端安装包。

## 数据位置

- Windows：`%APPDATA%\com.maishan.mem.desktop\mem-desktop.db`
- macOS：`~/Library/Application Support/com.maishan.mem.desktop/mem-desktop.db`
- Linux：`~/.local/share/com.maishan.mem.desktop/mem-desktop.db`

## 开源说明

源项目为 CC BY-NC 4.0。本桌面端沿用该许可，适合学习、研究与自用。

[linux.do](https://linux.do)
