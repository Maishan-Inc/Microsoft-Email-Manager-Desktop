/**
 * 快捷查看：可固定「一组」邮箱，在「快捷查看」页只在这组邮箱间切换。
 * 持久化到 localStorage；兼容旧版单邮箱键（mem.quickview）。
 */

const KEY = "mem.quickview.v2";
const LEGACY = "mem.quickview";

interface QuickState {
  emails: string[];
  current: string;
}

function read(): QuickState {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      const p = JSON.parse(raw);
      if (p && Array.isArray(p.emails)) {
        const emails: string[] = p.emails.filter((e: unknown) => typeof e === "string");
        const current = typeof p.current === "string" && emails.includes(p.current)
          ? p.current
          : emails[0] ?? "";
        return { emails, current };
      }
    }
    // 迁移旧版单邮箱
    const legacy = localStorage.getItem(LEGACY);
    if (legacy) return { emails: [legacy], current: legacy };
  } catch {
    /* ignore */
  }
  return { emails: [], current: "" };
}

export const quickview = $state<QuickState>(read());

function persist() {
  try {
    localStorage.setItem(KEY, JSON.stringify({ emails: quickview.emails, current: quickview.current }));
  } catch {
    /* ignore */
  }
}

/** 是否已固定在快捷查看 */
export function isQuick(email: string): boolean {
  return quickview.emails.includes(email);
}

/** 加入快捷查看（已存在则仅切到它） */
export function addQuick(email: string) {
  if (!email) return;
  if (!quickview.emails.includes(email)) quickview.emails = [...quickview.emails, email];
  quickview.current = email;
  persist();
}

/** 从快捷查看移除 */
export function removeQuick(email: string) {
  quickview.emails = quickview.emails.filter((e) => e !== email);
  if (quickview.current === email) quickview.current = quickview.emails[0] ?? "";
  persist();
}

/** 切换：在/不在快捷查看之间翻转 */
export function toggleQuick(email: string) {
  if (quickview.emails.includes(email)) removeQuick(email);
  else addQuick(email);
}

/** 设为当前展示的快捷查看邮箱 */
export function setCurrentQuick(email: string) {
  if (quickview.emails.includes(email)) {
    quickview.current = email;
    persist();
  }
}

/** 兼容旧调用：固定一个邮箱并设为当前 */
export function setQuick(email: string) {
  addQuick(email);
}

export function clearQuick() {
  quickview.emails = [];
  quickview.current = "";
  persist();
}
