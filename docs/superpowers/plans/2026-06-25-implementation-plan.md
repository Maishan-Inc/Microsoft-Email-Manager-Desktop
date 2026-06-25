# 实现计划：界面重构 + 首启引导 + 安全内核

- 关联设计：`docs/superpowers/specs/2026-06-25-desktop-ui-and-onboarding-design.md`
- 分支：`feature/ui-redo-and-onboarding`
- 基线：`pnpm check` 绿（0 错 0 警）。前端验证用 `pnpm check` + `pnpm build`；后端用 `cargo check`（在 `src-tauri/`）；打包用 `pnpm tauri build`。

## 执行原则

- 前端优先，按里程碑提交，每个里程碑跑 `pnpm check`。
- 后端改动后跑 `cargo check`。
- 阶段一不动 `crypto.rs`（风险隔离）。阶段二再引入 DEK 包装 / TOTP / 助记词。

---

## 阶段一

### A. 设计基座（主题 + Logo + i18n 脚手架）
- [ ] A1 `src/app.css`：按 DESIGN.md 重写为浅色 token（颜色/圆角/间距/阴影/字体）+ 全局元素样式。
- [ ] A2 `src/assets/logo-black.svg`、`logo-white.svg`：重绘「层叠卡片+折角」字形。
- [ ] A3 `src/lib/i18n.ts`：`$state` 当前语言 + 中英文案表 + `t()`；默认中文，持久化到 localStorage。
- [ ] 检查点：`pnpm check`、`pnpm build`。

### B. 应用壳 + 侧栏 + 路由
- [ ] B1 `src/components/Sidebar.svelte`：logo+名称、五导航项、底部锁定、激活态左缘条。
- [ ] B2 `src/App.svelte`：壳布局（侧栏+内容区），`view` 扩展为 dashboard/accounts/add/categories/settings/emails；`selectedEmail` 传参。
- [ ] 检查点：`pnpm check`。

### C. 各视图
- [ ] C1 `Dashboard.svelte`：三统计卡（邮箱数 / 健康度 / 当日接收）+ 最近活动 + 快捷入口；调用 `dashboard_stats`、`sync_mail_now`。
- [ ] C2 `Accounts.svelte`：整行卡片列表 + 行内操作（测试/健康/通知开关/删除）+ 点行进 emails；工具条留刷新/导出。
- [ ] C3 `AddEmail.svelte`：迁移单个添加 + 批量导入。
- [ ] C4 `Categories.svelte`：分类/标签增删（复用现有命令）。
- [ ] C5 `Settings.svelte`：语言/主题/后台刷新开关与间隔/导出/关于(Maishan Inc.)。
- [ ] C6 `Emails.svelte`：新主题重皮 + 接受 `selectedEmail`。
- [ ] C7 `src/lib/{api.ts,types.ts}`：新增命令封装与类型。
- [ ] 检查点：`pnpm check`、`pnpm build`。

### D. 后端：通知 + 后台刷新 + 设置
- [ ] D1 `db.rs`：`accounts` 加 `notify_enabled/poll_interval_secs/last_sync_at`；新表 `mail_activity`；`app_meta` 设置键；`SCHEMA_VERSION→2` + 迁移（`ALTER TABLE ... ADD COLUMN` 幂等）。
- [ ] D2 `commands.rs`：`set_account_notify`、`dashboard_stats`、`sync_mail_now`、`get_settings`、`set_settings`。
- [ ] D3 后台任务：解锁后启动 tokio 轮询（锁定停止），新邮件→系统通知 + 写 `mail_activity` + `emit("mail:new")`。
- [ ] D4 `Cargo.toml` + `lib.rs`：加 `tauri-plugin-notification` 并注册新命令；`capabilities/default.json` 放行通知权限。
- [ ] 检查点：`cargo check`（在 `src-tauri/`）。

### E. 安装包中英文 + 品牌 + 图标
- [ ] E1 `scripts/gen-icon.mjs` 或 `tauri icon`：用新 logo 生成 `src-tauri/icons/*`。
- [ ] E2 `tauri.conf.json`：`publisher: Maishan Inc.`、版权、NSIS `languages:[SimpChinese,English]` + `displayLanguageSelector` + 头图/侧边图。
- [ ] 检查点：`pnpm tauri build`（产出 NSIS，验证语言选择）。

### F. 阶段一验收
- [ ] `pnpm check` + `pnpm build` 绿；`pnpm tauri dev` 起得来，五视图可用；开通知账号后台弹通知；NSIS 中英文 + 开发商正确。

---

## 阶段二（界面与功能稳定后）

### G. 安全内核（密钥包装）
- [ ] G1 `crypto.rs`：`random_dek`、`derive_kek_password/mnemonic/device`、`wrap_dek/unwrap_dek`、TOTP（totp-rs）、助记词（bip39）、`qr_svg`（qrcode）。
- [ ] G2 `db.rs`/`app_meta`：`dek_wrapped_pw/mn/dev`、`salt_*`、`totp_secret_enc`、`auth_mode`、`agreement_accepted_at`、`onboarding_completed`、`tutorial_seen`。
- [ ] G3 `commands.rs`：`setup_master_password`/`unlock` 改 DEK 语义；`enroll_totp`/`verify_totp`/`generate_mnemonic`/`enroll_mnemonic`/`verify_unlock`/`recover_with_mnemonic`/`onboarding_status`/`accept_agreement`/`complete_onboarding`。
- [ ] G4 旧库策略：检测旧式 → 提示重置或一次性升级包装（实现时定）。

### H. 引导向导（前端）
- [ ] H1 `onboarding/OnboardingWizard.svelte` 状态机 + 子步骤：Splash(GSAP)→Agreement(滚动门禁)→SetPassword→Enable2FA→TotpSetup(QR+星号遮罩+复制+令牌)→AuthMode→MnemonicIntro→MnemonicShow(下载txt)→MnemonicVerify(隐藏校验)→Done→FirstRun(教程/直接进)。
- [ ] H2 `App.svelte`：`onboarding_status` 未完成 → 渲染向导替代 `Unlock`。
- [ ] H3 教程：覆盖式 coach-marks 指向侧栏。
- [ ] 检查点：`pnpm check`；全新库走完整流程；助记词恢复重置密码。

---

## 提交节奏

每完成一个里程碑（A/B/C/D/E、G/H 各步）提交一次，信息含改动要点与验证结果。
