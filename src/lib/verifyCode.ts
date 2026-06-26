/**
 * 从邮件主题/正文中提取验证码（OTP）。命中返回纯码，否则 null。
 * 策略：先在「验证码/code/OTP…」关键词附近找；否则退而求一个独立的 6 位数字。
 */

const KEYWORDS =
  /(验证码|校验码|动态码|动态密码|安全码|安全代码|登录码|确认码|verification\s*code|verification|one[\s-]?time|passcode|pass\s*code|security\s*code|\botp\b|\bcode\b)/i;

function pick(text: string): string | null {
  if (!text) return null;

  // 1) 关键词附近优先（窗口内找 4-8 位数字，或含数字的 5-8 位字母数字）
  const kw = text.match(KEYWORDS);
  if (kw && kw.index != null) {
    const start = Math.max(0, kw.index - 28);
    const around = text.slice(start, kw.index + 60);
    const digits = around.match(/(?<![0-9])([0-9]{4,8})(?![0-9])/);
    if (digits) return digits[1];
    const alnum = around.match(/\b([0-9A-Za-z]{5,8})\b/g);
    if (alnum) {
      const code = alnum.find((s) => /[0-9]/.test(s) && /^[0-9A-Za-z]+$/.test(s) && !/^[A-Za-z]+$/.test(s));
      if (code) return code.toUpperCase();
    }
  }

  // 2) 无关键词：找一个独立的 6 位数字（最常见的一次性验证码）
  const six = text.match(/(?<![0-9])([0-9]{6})(?![0-9])/);
  if (six) return six[1];

  return null;
}

/** 合并多段文本后提取验证码。 */
export function extractCode(...parts: (string | null | undefined)[]): string | null {
  const text = parts.filter(Boolean).join("  ");
  return pick(text);
}
