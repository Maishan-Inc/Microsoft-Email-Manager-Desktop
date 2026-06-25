/** 快捷查看：指定一个邮箱，在「快捷查看」页直接显示其邮件。持久化到 localStorage。 */

const KEY = "mem.quickview";

function read(): string {
  try {
    return localStorage.getItem(KEY) || "";
  } catch {
    return "";
  }
}

export const quickview = $state<{ email: string }>({ email: read() });

export function setQuick(email: string) {
  quickview.email = email;
  try {
    localStorage.setItem(KEY, email);
  } catch {
    /* ignore */
  }
}

export function clearQuick() {
  setQuick("");
}
