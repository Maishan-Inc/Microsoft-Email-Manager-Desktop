import { invoke } from "@tauri-apps/api/core";
import type {
  AppStatus,
  AccountCredentials,
  AccountInfo,
  Catalog,
  ClassificationOption,
  ImportResult,
  EmailListResponse,
  EmailDetails,
  HealthResult,
  DashboardStats,
  AppSettings,
  OnboardingStatus,
  TotpSetup,
  MnemonicGen,
  SecuritySetup,
  UnlockResult,
} from "./types";

export const api = {
  // 解锁 / 主密码
  getStatus: () => invoke<AppStatus>("get_status"),
  setupMasterPassword: (password: string) =>
    invoke<void>("setup_master_password", { password }),
  unlock: (password: string) => invoke<UnlockResult>("unlock", { password }),
  verify2fa: (code: string) => invoke<void>("verify_2fa", { code }),
  recoverWithMnemonic: (words: string) =>
    invoke<void>("recover_with_mnemonic", { words }),
  resetPassword: (newPassword: string) =>
    invoke<void>("reset_password", { newPassword }),
  lock: () => invoke<void>("lock"),

  // 账号
  listAccounts: () => invoke<AccountInfo[]>("list_accounts"),
  addAccount: (
    creds: AccountCredentials,
    categoryKey: string | null,
    tagKeys: string[],
  ) =>
    invoke<void>("add_account", {
      creds,
      categoryKey,
      tagKeys,
    }),
  deleteAccount: (email: string) => invoke<void>("delete_account", { email }),
  updateClassification: (
    email: string,
    categoryKey: string | null,
    tagKeys: string[],
  ) =>
    invoke<void>("update_classification", { email, categoryKey, tagKeys }),
  importAccounts: (text: string, authMethod: string) =>
    invoke<ImportResult>("import_accounts", { text, authMethod }),
  testCredentials: (creds: AccountCredentials) =>
    invoke<void>("test_credentials", { creds }),
  testAccount: (email: string) => invoke<void>("test_account", { email }),

  // 每账号通知 + 后台刷新
  setAccountNotify: (
    email: string,
    enabled: boolean,
    intervalSecs: number | null,
  ) =>
    invoke<void>("set_account_notify", { email, enabled, intervalSecs }),

  // 分类 / 标签
  getCatalog: () => invoke<Catalog>("get_catalog"),
  addCategory: (opt: ClassificationOption) =>
    invoke<void>("add_category", { opt }),
  addTag: (opt: ClassificationOption) => invoke<void>("add_tag", { opt }),
  deleteCategory: (key: string) => invoke<void>("delete_category", { key }),
  deleteTag: (key: string) => invoke<void>("delete_tag", { key }),

  // 邮件
  listEmails: (
    email: string,
    folder: string,
    page: number,
    pageSize: number,
  ) =>
    invoke<EmailListResponse>("list_emails", {
      email,
      folder,
      page,
      pageSize,
    }),
  getEmailDetails: (email: string, messageId: string) =>
    invoke<EmailDetails>("get_email_details", { email, messageId }),

  // 健康检查
  checkAccountHealth: (email: string) =>
    invoke<HealthResult>("check_account_health", { email }),

  // 仪表盘 / 设置
  dashboardStats: () => invoke<DashboardStats>("dashboard_stats"),
  syncMailNow: () => invoke<number>("sync_mail_now"),
  getSettings: () => invoke<AppSettings>("get_settings"),
  setSettings: (settings: AppSettings) =>
    invoke<void>("set_settings", { settings }),

  // 首启引导 / 安全配置
  onboardingStatus: () => invoke<OnboardingStatus>("onboarding_status"),
  acceptAgreement: () => invoke<void>("accept_agreement"),
  generateTotp: () => invoke<TotpSetup>("generate_totp"),
  verifyTotpCode: (secret: string, code: string) =>
    invoke<boolean>("verify_totp_code", { secret, code }),
  generateMnemonic: () => invoke<MnemonicGen>("generate_mnemonic"),
  completeSetup: (setup: SecuritySetup) =>
    invoke<void>("complete_setup", { setup }),
  completeOnboarding: () => invoke<void>("complete_onboarding"),
  setTutorialSeen: () => invoke<void>("set_tutorial_seen"),

  // 导出
  exportAccounts: (
    path: string,
    format: "json" | "csv",
    includeCredentials: boolean,
    encrypt: boolean,
  ) =>
    invoke<void>("export_accounts", {
      path,
      format,
      includeCredentials,
      encrypt,
    }),
};

/** 把后端抛出的错误统一转成字符串 */
export function errMsg(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}
