# 桌面端界面重构 + 首启引导 + 安全内核 设计文档

- 日期：2026-06-25
- 项目：Microsoft Email Manager Desktop（Tauri 2 + Rust + Svelte 5）
- 状态：已与用户确认关键决策，待按本文档出实现计划

## 决策基线（用户已确认）

1. **助记词 = 真正的恢复密钥**：改造为密钥包装（key-wrapping）模型，忘记主密码可用助记词恢复。
2. **2FA 在解锁时强制校验**：密码解出数据密钥 → 解出 TOTP 密钥 → 校验 6 位动态码才放行；提供「独立2FA」模式（无密码、设备密钥兜底）。
3. **分两阶段实施**：阶段一界面/品牌/功能，阶段二首启引导向导 + 安全内核。
4. **允许新增依赖**：纯 Rust 实现 TOTP/QR/助记词，Tauri 官方通知插件。
5. **「4主题」= 规格第 4 条「主题」**：单一浅色设计系统（DESIGN.md / Vercel 风），非四套配色。深色为后续可选项。

## 总览

将现有「深色顶栏 + 三视图（解锁/账号/邮件）」改造为 DESIGN.md 浅色风格的**左侧栏应用壳**，补齐品牌、仪表盘首页、每账号后台通知；随后单独实现首启引导向导与配套的密钥包装安全内核。阶段一不触碰加密内核（仅重皮），把加密改造的风险隔离到阶段二。

现状关键文件：
- 前端：`src/App.svelte`（壳 + 顶栏导航）、`src/components/{Unlock,Accounts,Emails}.svelte`、`src/lib/{api.ts,types.ts,toast.svelte.ts}`、`src/app.css`。
- 后端：`src-tauri/src/{commands,db,state,crypto,models,accounts,accounts_auth,graph,imap_client,export,error,lib}.rs`。
- 配置：`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`scripts/gen-icon.mjs`。
- 设计系统：`DESIGN.md`。

---

## 阶段一：界面 / 品牌 / 功能

### 1. 品牌与 Logo

- 母版：`C:\Users\Administrator\Desktop\Microsoft-Email-Manager-bai.png`（白色「层叠文档」图标）。
- 产出：
  - `src/assets/logo-black.svg` 与 `src/assets/logo-white.svg`（用 `currentColor`/`fill` 便于自适应）。重绘为「层叠卡片 + 折角」干净矢量，保留原 PNG 做图标管线。
  - 应用图标：用 `tauri icon`（或更新 `scripts/gen-icon.mjs` 绘制该字形）生成 `src-tauri/icons/*`。
- 用途：侧栏顶部、引导页、安装器头图/侧边图、任务栏图标。浅色 UI 主用黑版，深色条/动画用白版。
- 验收：成品 logo 给用户确认，不满意可调整字形。

### 2. 主题（DESIGN.md）

- 用 DESIGN.md token 重写 `src/app.css`，定义 CSS 变量：
  - 颜色：`--canvas #ffffff`、`--canvas-soft #fafafa`、`--canvas-soft-2 #f5f5f5`、`--ink #171717`、`--body #4d4d4d`、`--mute #888888`、`--hairline #ebebeb`、`--hairline-strong #a1a1a1`、`--link #0070f3`、`--error #ee0000`、`--warning #f5a623`、`--success #0070f3`。
  - 圆角：`--r-sm 6px`、`--r-md 8px`、`--r-lg 12px`、`--r-pill 100px`、`--r-full 9999px`。
  - 间距阶梯 `--s-xxs..--s-6xl`（4/8/12/16/24/32/40/48/64/96/128）。
  - 阴影：Level 1~5 堆叠阴影 + inset hairline（见 DESIGN.md「Elevation & Depth」）。
  - 字体：`Geist, Inter, system-ui, "Microsoft YaHei", sans-serif`；等宽 `Geist Mono, ui-monospace, monospace`。
- 全局元素（button/input/select/textarea/card/badge/table/toast）改为 DESIGN.md 组件规格。

### 3. 布局：顶栏 → 左侧栏

- 新增 `src/components/Sidebar.svelte`：
  - 顶部：logo + 产品名。
  - 导航项（图标 + 文案）：`dashboard 首页`、`accounts 邮箱账户管理`、`add 添加邮箱`、`categories 分类编辑`、`settings 系统设置`。
  - 底部：`🔒 锁定`（调用 `api.lock()`）。
  - 激活态：左缘 `--ink` 指示条，背景 `--canvas-soft`，圆角 `--r-sm`。
- `src/App.svelte` 改为应用壳：左侧 `Sidebar` + 右侧内容区，按 `view` 状态切换；`view` 类型扩展为 `"dashboard" | "accounts" | "add" | "categories" | "settings" | "emails"`。`emails` 不在侧栏，由账户行点击进入，带 `selectedEmail` 参数。
- 内容区宽度自适应（去掉旧的 `max-width:1000px` 居中限制，改为内容区铺满 + 自身内边距）。

### 4. 首页仪表盘（新增 `src/components/Dashboard.svelte`）

- 三张统计卡（DESIGN.md `card-marketing` 规格）：
  - **当前邮箱数量**：`listAccounts().length`。
  - **邮箱健康度**：各账号 `health_score` 聚合。展示：平均分 + 健康/总数；用环形或水平条。无检查记录时显示「未检查」。
  - **当日接收邮件数**：来自本地 `mail_activity` 表中 `received_at` 为今天的记录数（跨所有账号）。
- 数据来源决策：当日接收数**不**每次进首页全量拉邮件（慢/限流），而是读本地 `mail_activity`（后台刷新写入）。提供「刷新统计」按钮触发 `sync_mail_now`：对每账号收件箱第 1 页做有限抓取并更新 `mail_activity`。
- 次要区块：最近活动列表（最近 N 条 `mail_activity`）、快捷入口（添加邮箱 / 去账户）。

### 5. 邮箱账户管理：横条 + 点击查看邮件

- `src/components/Accounts.svelte` 重做为**整行卡片列表**（取代表格）：
  - 每行：邮箱、接入徽章（imap/graph）、状态点、健康药丸（分数配色：≥100 绿 / ≥60 黄 / 其它红）、上次检查时间、行内操作：`测试`、`健康`、`通知开关`、`删除`。
  - 点击行体（非按钮区域）→ 设置 `selectedEmail` 并切到 `emails` 视图。
- 顶部工具条保留：`刷新`、`导出`（导出也可放系统设置，本设计放账户工具条 + 设置各一入口，复用同一弹窗组件）。`添加账号 / 批量导入`移到「添加邮箱」视图。
- `src/components/Emails.svelte`：沿用三栏（账号列 / 列表 / 详情），按新主题重皮；进入时若带 `selectedEmail` 则默认选中。

### 6. 添加邮箱 / 分类编辑 / 系统设置（独立视图）

- **`AddEmail.svelte`（新）**：迁移现有「单个添加（先测试后入库）」+「批量导入（imap/graph 两种格式）」。
- **`Categories.svelte`（新）**：分类与标签的增删（复用 `add_category/add_tag/delete_category/delete_tag/get_catalog`）；列出现有项，表单新增（key/name_zh/name_en/remark）。
- **`Settings.svelte`（新）**：
  - 语言（中/英，见 i18n 说明）。
  - 主题（浅色；预留深色开关）。
  - 后台刷新：全局开关 + 默认间隔（秒）。
  - 数据导出（复用导出弹窗）。
  - 关于：版本、开发商 **Maishan Inc.**、许可、数据目录路径。

### 7. 每账号通知 + 后台刷新（新功能）

- 数据库：
  - `accounts` 加列：`notify_enabled INTEGER NOT NULL DEFAULT 0`、`poll_interval_secs INTEGER`（可空，回退全局）、`last_sync_at TEXT`。
  - 新表 `mail_activity(email TEXT, message_id TEXT, subject TEXT, from_email TEXT, received_at TEXT, seen_at TEXT, PRIMARY KEY(email, message_id))`，保留近 7 天/上限 N 条。
- 后端后台任务：
  - 解锁成功后启动一个 tokio 任务（锁定时停止/置标志）。循环：对每个 `notify_enabled=1` 的账号，按其 `poll_interval_secs`（或全局默认）抓取收件箱最新一页，与 `mail_activity` 已记录的 diff。
  - 新邮件：① 通过 `tauri-plugin-notification` 弹系统通知；② 写入 `mail_activity`；③ `app.emit("mail:new", payload)` 通知前端刷新仪表盘/账户。
  - 并发与锁：读凭据只在持 `vault` 锁的瞬间完成，网络抓取不跨锁（沿用现有 `with_vault` + `spawn_blocking`/async 模式）。
- 命令：`set_account_notify(email, enabled, interval_secs?)`、`sync_mail_now()`、`dashboard_stats()`。
- 全局开关/间隔存 `app_meta`：`bg_refresh_enabled`、`bg_refresh_interval_secs`（默认 300）。
- 权限：首启或首次开启通知时申请系统通知权限。

### 8. 安装包中英文 + 开发商

- `tauri.conf.json`：
  - 顶层/bundle：`"publisher": "Maishan Inc."`、`"copyright": "© 2026 Maishan Inc."`。
  - `bundle.windows.nsis`：`"languages": ["SimpChinese", "English"]`、`"displayLanguageSelector": true`、`installerIcon`、`headerImage`/`sidebarImage`（用 logo 生成的 BMP）。
  - 图标数组指向新生成的 `icons/*`。
- macOS/Linux：通过 `publisher`/`longDescription`/maintainer 字段体现开发商；中英文以 NSIS 为主（Windows 主目标）。

---

## 阶段二：首启引导向导 + 安全内核

### 9. 引导向导（`src/components/onboarding/OnboardingWizard.svelte` 状态机 + 子步骤组件）

启动判定：`onboarding_status()` 返回未完成（未初始化或 `onboarding_completed` 未置位）→ 渲染向导，替代 `Unlock`。步骤：

1. **弹出动画 Splash**：logo 揭示动画（GSAP 或 CSS），短暂后「开始」/自动进入。
2. **用户协议 Agreement**：长文可滚动；监听滚动，**滚到底部**才启用「同意」按钮。同意 → `accept_agreement()` 记录 `agreement_accepted_at`。
3. **配置密码 SetPassword**：密码 + 确认 + 强度提示（≥8 位）。提交 → 生成 DEK，用密码 KEK 包装（见安全模型）。
4. **是否开启 2FA Enable2FA**：`跳过` / `开启` 两按钮。
5. **TOTP 设置 TotpSetup**（仅开启时）：
   - `enroll_totp()` 在后端生成密钥与 otpauth URI、QR 的 SVG。
   - 展示二维码 + **密钥中间用星号遮罩、可一键复制完整密钥**。
   - 按钮：`上一步` / `输入令牌`（6 位输入框）→ `verify_totp(code)` 通过 → 存密钥（DEK 加密），标记 2FA 启用。
6. **如何完成认证 AuthMode**（仅 2FA 已启用时出现）：`使用密码+2FA` / `独立2FA` → 设 `auth_mode`；独立2FA 额外建立设备密钥包装。
7. **你是否记得密码 MnemonicIntro**：`记得`（仍建议配置恢复，可选）/ `不记得，生成助记词`。
8. **助记词展示 MnemonicShow**：`generate_mnemonic()` 返回 12 词（BIP39）。网格展示，提示保存，`下载 txt`（前端用 Blob 或 Tauri save 对话框）。按钮 `上一步` / `下一步` → `enroll_mnemonic(words)` 用助记词 KEK 再包装一份 DEK。
9. **校验助记词 MnemonicVerify**：随机隐藏 3~4 词，用户补全；`确认` 校验。失败可重试。
10. **完成 Done**：「你已完成恢复助记词配置」→ `开始使用 Microsoft Email Manager` 按钮 → `complete_onboarding()`。
11. **首次/老用户 FirstRun**：`我是第一次使用` → 新手教程（覆盖式 coach-marks 指向侧栏各项）；`我之前使用过` → 直接进应用。记 `tutorial_seen`。

向导可前进/后退（除已落库的不可逆步骤需提示）。任意安全步骤失败给出明确错误，不静默。

### 10. 安全模型（密钥包装；`crypto.rs` + `db.rs` 改造）

- **DEK（Data Encryption Key）**：32 字节随机，真正用于加密账号字段（沿用 AES-256-GCM）。DEK 不直接落盘，被多个 KEK 各包装一份。
- **KEK 与包装**：
  - `KEK_password = Argon2id(password, salt_pw)` → `dek_wrapped_pw = AES-GCM(KEK_password, DEK)`。
  - `KEK_mnemonic = HKDF-SHA256(BIP39_seed)` → `dek_wrapped_mn`（恢复路径）。
  - 独立2FA 模式：`KEK_device = HKDF(本地随机设备密钥)` → `dek_wrapped_dev`（无密码日常解锁；at-rest 较弱，需在 UI 明确告知）。设备密钥存于应用数据目录受限文件。
- **2FA 作为门禁**：TOTP 密钥用 DEK 加密存 `totp_secret_enc`。解锁流程：密码（或设备密钥）解出 DEK → 解出 TOTP 密钥 → 前端输入 6 位 → `verify_unlock` 校验通过才置 `vault` 为已解锁。
- **校验串 verifier**：改为校验 DEK（解出 DEK 后比对），替代旧的「密码派生密钥直接校验」。
- **恢复**：`recover_with_mnemonic(words)` 用助记词解出 DEK → 允许用户重设密码（重写 `dek_wrapped_pw` 与 `salt_pw`）。
- **迁移**：本仓库尚无提交、属预发布；按全新安装设计。`SCHEMA_VERSION` 升到 2；检测到旧式（有 `verifier`/`master_salt` 无 `dek_wrapped_*`）时，提示用户为预发布数据、需重新初始化（或在解锁瞬间以旧密钥为 DEK 完成一次性包装升级——实现计划阶段二选定其一）。
- **阶段隔离**：阶段一不改 crypto；阶段二一次性引入 DEK 包装、TOTP、助记词。

### 11. crypto.rs / db.rs 接口（阶段二）

- `crypto`: `random_dek()`、`derive_kek_password(pw, salt)`、`derive_kek_mnemonic(seed)`、`derive_kek_device(secret)`、`wrap_dek(kek, dek) -> blob`、`unwrap_dek(kek, blob) -> dek`、`totp_*`（用 `totp-rs`）、`mnemonic_generate()/seed()`（用 `bip39`）、`qr_svg(otpauth_uri)`（用 `qrcode`）。
- `db`/`app_meta` 新键：`dek_wrapped_pw`、`salt_pw`、`dek_wrapped_mn`、`salt_mn`、`dek_wrapped_dev`、`totp_secret_enc`、`auth_mode`(`password_2fa`|`standalone_2fa`|`password_only`)、`agreement_accepted_at`、`onboarding_completed`、`tutorial_seen`。

---

## 依赖清单

- Rust（`src-tauri/Cargo.toml`）：`totp-rs`、`qrcode`、`bip39`、`hkdf`、`sha2`、`tauri-plugin-notification`（+ 可选 `tauri-plugin-autostart`）。
- 前端（`package.json`）：`gsap`、`@tauri-apps/plugin-notification`。
- 图标：`tauri icon` 流程（或现有 `scripts/gen-icon.mjs` 改造）。

## 数据库变更汇总

- `accounts`：+`notify_enabled`、`poll_interval_secs`、`last_sync_at`。
- 新表 `mail_activity`。
- `app_meta`：+ 安全相关键（见 §11）+ `bg_refresh_enabled`、`bg_refresh_interval_secs`。
- `SCHEMA_VERSION`：1 → 2。

## 新增/变更 Tauri 命令

- 阶段一：`set_account_notify`、`sync_mail_now`、`dashboard_stats`、`get_settings`/`set_settings`（语言/主题/后台刷新）。
- 阶段二：`onboarding_status`、`accept_agreement`、`complete_onboarding`、`enroll_totp`、`verify_totp`、`generate_mnemonic`、`enroll_mnemonic`、`verify_unlock`、`recover_with_mnemonic`；`setup_master_password`/`unlock` 改造为 DEK 包装语义。

## i18n（中/英）

- 安装器：NSIS 双语 + 语言选择器。
- 应用内：引入轻量 i18n（一个 `src/lib/i18n.ts` + 中英文案表 + `$state` 当前语言），系统设置切换；阶段一先把侧栏/仪表盘/设置等新文案纳入，逐步替换硬编码中文。默认中文。

## 验收口径

- **阶段一**：`pnpm check` 与 `pnpm build` 通过；`pnpm tauri dev` 可启动；侧栏五项 + 锁定可用；仪表盘三卡有数；账户横条点击进邮件；添加/分类/设置可用；开通知的账号后台能弹系统通知并刷新当日计数；`pnpm tauri build` 产出带中英文选择的 NSIS，开发商为 Maishan Inc.。
- **阶段二**：全新库走完引导（协议滚动门禁、密码、2FA 二维码+遮罩复制+令牌校验、认证模式、助记词生成/下载/隐藏校验、完成、首次教程）后能正常解锁；用助记词可恢复并重设密码。

## 风险与开放项

- Logo 字形为「按观感重绘」，需用户确认。
- 「独立2FA」at-rest 安全弱于「密码+2FA」，UI 必须明确告知。
- 「当日接收数」依赖后台刷新写入，冷启动无数据时为 0，需「刷新统计」兜底。
- 旧 dev 库迁移策略在实现计划阶段二最终敲定（重置 vs 一次性升级包装）。
- i18n 为渐进式：阶段一覆盖新视图，存量中文文案后续批次替换。
