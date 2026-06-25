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
