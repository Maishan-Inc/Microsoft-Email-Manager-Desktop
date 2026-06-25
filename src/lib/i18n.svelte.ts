/** 轻量 i18n：响应式语言状态 + 中英文案表 + t() 插值。默认中文，持久化到 localStorage。 */

export type Lang = "zh" | "en";

const STORAGE_KEY = "mem.lang";

function detectInitial(): Lang {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "zh" || saved === "en") return saved;
  } catch {
    /* ignore */
  }
  return "zh";
}

export const i18n = $state<{ lang: Lang }>({ lang: detectInitial() });

export function setLang(lang: Lang) {
  i18n.lang = lang;
  try {
    localStorage.setItem(STORAGE_KEY, lang);
  } catch {
    /* ignore */
  }
}

/** 文案表：每个 key 给 zh / en 两版。新增 UI 时往这里加键。 */
const M = {
  "app.name": { zh: "Microsoft 邮箱管理", en: "Microsoft Email Manager" },
  "app.starting": { zh: "启动中…", en: "Starting…" },

  // 导航
  "nav.dashboard": { zh: "首页", en: "Home" },
  "nav.accounts": { zh: "邮箱账户管理", en: "Accounts" },
  "nav.add": { zh: "添加邮箱", en: "Add Email" },
  "nav.categories": { zh: "分类编辑", en: "Categories" },
  "nav.settings": { zh: "系统设置", en: "Settings" },
  "nav.lock": { zh: "锁定", en: "Lock" },

  // 通用
  "common.refresh": { zh: "刷新", en: "Refresh" },
  "common.cancel": { zh: "取消", en: "Cancel" },
  "common.confirm": { zh: "确认", en: "Confirm" },
  "common.save": { zh: "保存", en: "Save" },
  "common.delete": { zh: "删除", en: "Delete" },
  "common.add": { zh: "添加", en: "Add" },
  "common.back": { zh: "上一步", en: "Back" },
  "common.next": { zh: "下一步", en: "Next" },
  "common.loading": { zh: "加载中…", en: "Loading…" },
  "common.none": { zh: "暂无", en: "None" },
  "common.copy": { zh: "复制", en: "Copy" },
  "common.copied": { zh: "已复制", en: "Copied" },
  "common.download": { zh: "下载", en: "Download" },
  "common.enabled": { zh: "已开启", en: "On" },
  "common.disabled": { zh: "已关闭", en: "Off" },

  // 仪表盘
  "dash.title": { zh: "概览", en: "Overview" },
  "dash.accounts": { zh: "邮箱数量", en: "Mailboxes" },
  "dash.health": { zh: "邮箱健康度", en: "Health" },
  "dash.todayMail": { zh: "当日接收邮件", en: "Mail today" },
  "dash.recent": { zh: "最近邮件", en: "Recent mail" },
  "dash.refreshStats": { zh: "刷新统计", en: "Refresh stats" },
  "dash.noActivity": { zh: "暂无邮件活动，开启账户通知或点「刷新统计」", en: "No activity yet — enable notifications or refresh stats" },
  "dash.healthy": { zh: "健康", en: "Healthy" },
  "dash.unchecked": { zh: "未检查", en: "Unchecked" },
  "dash.goAdd": { zh: "添加邮箱", en: "Add a mailbox" },
  "dash.goAccounts": { zh: "管理账户", en: "Manage accounts" },

  // 账户
  "acc.title": { zh: "邮箱账户管理", en: "Accounts" },
  "acc.count": { zh: "共 {n} 个", en: "{n} total" },
  "acc.empty": { zh: "还没有账号，去「添加邮箱」开始。", en: "No accounts yet. Go to “Add Email”." },
  "acc.test": { zh: "测试", en: "Test" },
  "acc.testing": { zh: "测试中…", en: "Testing…" },
  "acc.health": { zh: "健康", en: "Health" },
  "acc.checking": { zh: "检查中…", en: "Checking…" },
  "acc.notify": { zh: "通知", en: "Notify" },
  "acc.openMail": { zh: "查看邮件", en: "View mail" },
  "acc.export": { zh: "导出", en: "Export" },
  "acc.lastCheck": { zh: "上次检查", en: "Last check" },
  "acc.never": { zh: "从未", en: "Never" },

  // 添加邮箱
  "add.title": { zh: "添加邮箱", en: "Add Email" },
  "add.single": { zh: "单个添加", en: "Add one" },
  "add.bulk": { zh: "批量导入", en: "Bulk import" },
  "add.email": { zh: "邮箱", en: "Email" },
  "add.clientId": { zh: "客户端 ID", en: "Client ID" },
  "add.refreshToken": { zh: "刷新令牌", en: "Refresh token" },
  "add.testAdd": { zh: "测试并添加", en: "Test & add" },
  "add.verifying": { zh: "验证中…", en: "Verifying…" },
  "add.bulkHint": { zh: "每行一个账号，使用 ---- 分隔", en: "One account per line, separated by ----" },
  "add.startImport": { zh: "开始导入", en: "Start import" },
  "add.importing": { zh: "导入中…", en: "Importing…" },

  // 分类
  "cat.title": { zh: "分类编辑", en: "Categories" },
  "cat.categories": { zh: "分类", en: "Categories" },
  "cat.tags": { zh: "标签", en: "Tags" },
  "cat.key": { zh: "标识 key", en: "Key" },
  "cat.nameZh": { zh: "中文名", en: "Name (ZH)" },
  "cat.nameEn": { zh: "英文名", en: "Name (EN)" },
  "cat.remark": { zh: "备注", en: "Remark" },
  "cat.empty": { zh: "还没有项目", en: "Nothing here yet" },

  // 设置
  "set.title": { zh: "系统设置", en: "Settings" },
  "set.language": { zh: "语言", en: "Language" },
  "set.theme": { zh: "主题", en: "Theme" },
  "set.themeLight": { zh: "浅色", en: "Light" },
  "set.themeDark": { zh: "深色", en: "Dark" },
  "set.themeSystem": { zh: "跟随系统", en: "System" },
  "set.bgRefresh": { zh: "后台刷新新邮件", en: "Background mail refresh" },
  "set.bgInterval": { zh: "刷新间隔（秒）", en: "Interval (seconds)" },
  "set.export": { zh: "数据导出", en: "Export data" },
  "set.about": { zh: "关于", en: "About" },
  "set.publisher": { zh: "开发商", en: "Publisher" },
  "set.version": { zh: "版本", en: "Version" },
  "set.dataDir": { zh: "数据目录", en: "Data directory" },
  "set.license": { zh: "许可", en: "License" },

  // 邮件
  "mail.inbox": { zh: "收件箱", en: "Inbox" },
  "mail.junk": { zh: "垃圾箱", en: "Junk" },
  "mail.all": { zh: "全部", en: "All" },
  "mail.none": { zh: "暂无邮件", en: "No mail" },
  "mail.selectOne": { zh: "选择一封邮件查看详情", en: "Select an email to view" },
  "mail.from": { zh: "发件人", en: "From" },
  "mail.to": { zh: "收件人", en: "To" },
  "mail.time": { zh: "时间", en: "Time" },
  "mail.prev": { zh: "上一页", en: "Prev" },
  "mail.nextPage": { zh: "下一页", en: "Next" },
  "mail.pageInfo": { zh: "第 {page} 页 / 共 {total} 封", en: "Page {page} / {total} total" },
  "mail.pickAccount": { zh: "请先在「添加邮箱」添加账号", en: "Add an account first" },

  // 解锁 / 首次设置
  "unlock.subtitleSetup": { zh: "首次使用，请设置一个主密码", en: "First run — set a master password" },
  "unlock.subtitleUnlock": { zh: "请输入主密码解锁本地数据库", en: "Enter your master password to unlock" },
  "unlock.hint": { zh: "数据全部加密保存在本机，主密码不会上传也无法找回，请牢记。", en: "All data is encrypted locally. The master password is never uploaded and cannot be recovered — keep it safe." },
  "unlock.password": { zh: "主密码（至少 8 位）", en: "Master password (min 8)" },
  "unlock.confirm": { zh: "再次输入主密码", en: "Confirm master password" },
  "unlock.minLen": { zh: "主密码至少 8 位", en: "Password must be at least 8 characters" },
  "unlock.mismatch": { zh: "两次输入不一致", en: "Passwords do not match" },
  "unlock.unlock": { zh: "解锁", en: "Unlock" },
  "unlock.setup": { zh: "设置并进入", en: "Set & enter" },
  "unlock.processing": { zh: "处理中…", en: "Working…" },
  "unlock.need2fa": { zh: "请输入两步验证码", en: "Enter your 2FA code" },
  "unlock.verify2fa": { zh: "验证", en: "Verify" },

  // ===== 首启引导向导 =====
  "ob.splash.tagline": { zh: "本地加密 · 多账号 · 隐私优先", en: "Local-encrypted · Multi-account · Privacy-first" },
  "ob.splash.start": { zh: "开始", en: "Get started" },

  "ob.agreement.title": { zh: "用户协议", en: "User Agreement" },
  "ob.agreement.scrollHint": { zh: "请滚动阅读至底部以继续", en: "Scroll to the bottom to continue" },
  "ob.agreement.agree": { zh: "我已阅读并同意", en: "I have read and agree" },

  "ob.password.title": { zh: "配置主密码", en: "Set master password" },
  "ob.password.desc": { zh: "主密码用于加密本地数据，不会上传，请牢记。", en: "Used to encrypt local data. Never uploaded — keep it safe." },
  "ob.password.pwd": { zh: "主密码（至少 8 位）", en: "Master password (min 8)" },
  "ob.password.confirm": { zh: "再次输入", en: "Confirm" },
  "ob.password.weak": { zh: "强度：弱", en: "Strength: weak" },
  "ob.password.medium": { zh: "强度：中", en: "Strength: medium" },
  "ob.password.strong": { zh: "强度：强", en: "Strength: strong" },

  "ob.2fa.title": { zh: "是否开启两步验证（2FA）", en: "Enable two-factor (2FA)?" },
  "ob.2fa.desc": { zh: "开启后，解锁需额外输入动态验证码，更安全。", en: "Adds a one-time code at unlock for extra security." },
  "ob.2fa.skip": { zh: "跳过", en: "Skip" },
  "ob.2fa.enable": { zh: "开启 2FA", en: "Enable 2FA" },

  "ob.totp.title": { zh: "扫码绑定验证器", en: "Scan with your authenticator" },
  "ob.totp.scanHint": { zh: "用 Authenticator 等应用扫描二维码，或手动输入密钥：", en: "Scan with an authenticator app, or enter the key manually:" },
  "ob.totp.copy": { zh: "复制密钥", en: "Copy key" },
  "ob.totp.tokenPlaceholder": { zh: "输入 6 位验证码", en: "Enter 6-digit code" },
  "ob.totp.verify": { zh: "验证并完成", en: "Verify & finish" },
  "ob.totp.invalid": { zh: "验证码不正确", en: "Invalid code" },

  "ob.authmode.title": { zh: "如何完成认证", en: "How to authenticate" },
  "ob.authmode.pw2fa": { zh: "使用 密码 + 2FA", en: "Password + 2FA" },
  "ob.authmode.pw2faDesc": { zh: "解锁需主密码与动态码，安全性最高。", en: "Requires master password and a code. Most secure." },
  "ob.authmode.standalone": { zh: "独立 2FA", en: "Standalone 2FA" },
  "ob.authmode.standaloneDesc": { zh: "仅用动态码解锁，无需主密码。", en: "Unlock with the code only, no password." },
  "ob.authmode.standaloneWarn": { zh: "⚠️ 独立 2FA 下数据由设备密钥兜底加密，安全性弱于「密码 + 2FA」。", en: "⚠️ Standalone 2FA encrypts data with a device key — weaker at rest than Password + 2FA." },

  "ob.mnem.introTitle": { zh: "恢复助记词", en: "Recovery phrase" },
  "ob.mnem.introQ": { zh: "你能记住主密码吗？若不记得，可生成一组助记词用于恢复。", en: "Can you remember the password? If not, generate a recovery phrase." },
  "ob.mnem.remember": { zh: "我能记住，跳过", en: "I'll remember — skip" },
  "ob.mnem.generate": { zh: "生成助记词", en: "Generate phrase" },
  "ob.mnem.showTitle": { zh: "请抄写并妥善保存", en: "Write these down and keep them safe" },
  "ob.mnem.showDesc": { zh: "任何人拿到这组助记词都能恢复你的数据，切勿截图上传或泄露。", en: "Anyone with this phrase can recover your data. Never screenshot, upload, or share it." },
  "ob.mnem.download": { zh: "下载为 txt", en: "Download as txt" },
  "ob.mnem.verifyTitle": { zh: "校验助记词", en: "Verify your phrase" },
  "ob.mnem.verifyDesc": { zh: "请填回下列缺失的词，确认你已正确保存。", en: "Fill in the missing words to confirm you saved them." },
  "ob.mnem.verifyWrong": { zh: "与生成的助记词不一致，请检查。", en: "Doesn't match the generated phrase." },
  "ob.mnem.word": { zh: "第 {n} 个", en: "Word {n}" },

  "ob.done.title": { zh: "你已完成恢复助记词配置", en: "Recovery setup complete" },
  "ob.done.desc": { zh: "一切就绪，开始使用吧。", en: "All set — let's get started." },
  "ob.done.start": { zh: "开始使用 Microsoft Email Manager", en: "Start using Microsoft Email Manager" },

  "ob.firstrun.title": { zh: "你之前用过本软件吗？", en: "Have you used this app before?" },
  "ob.firstrun.first": { zh: "我是第一次使用", en: "First time here" },
  "ob.firstrun.firstDesc": { zh: "带你快速了解界面", en: "A quick tour of the interface" },
  "ob.firstrun.returning": { zh: "我之前使用过", en: "I've used it before" },
  "ob.firstrun.returningDesc": { zh: "直接进入应用", en: "Go straight to the app" },

  "ob.tutorial.skip": { zh: "跳过教程", en: "Skip tour" },
  "ob.tutorial.next": { zh: "下一步", en: "Next" },
  "ob.tutorial.done": { zh: "完成", en: "Done" },
  "ob.tutorial.t1": { zh: "左侧是导航：首页、账户、添加邮箱、分类、设置。", en: "The sidebar: Home, Accounts, Add, Categories, Settings." },
  "ob.tutorial.t2": { zh: "首页是控制面板，看邮箱数量、健康度与当日邮件。", en: "Home is your dashboard: mailbox count, health, mail today." },
  "ob.tutorial.t3": { zh: "在「添加邮箱」导入账号，账户列表点一行即可看邮件。", en: "Add accounts under “Add Email”; click an account row to read mail." },
  "ob.tutorial.t4": { zh: "每个账户可单独开启新邮件通知，后台自动刷新。", en: "Enable per-account new-mail notifications with background refresh." },
} satisfies Record<string, { zh: string; en: string }>;

export type MsgKey = keyof typeof M;

/** 取当前语言文案，支持 {name} 占位插值。 */
export function t(key: MsgKey, params?: Record<string, string | number>): string {
  const entry = M[key];
  let s = entry ? entry[i18n.lang] : (key as string);
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      s = s.replace(new RegExp(`\\{${k}\\}`, "g"), String(v));
    }
  }
  return s;
}
