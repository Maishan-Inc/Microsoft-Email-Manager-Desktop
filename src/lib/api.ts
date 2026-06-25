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
} from "./types";

export const api = {
  // 解锁 / 主密码
  getStatus: () => invoke<AppStatus>("get_status"),
  setupMasterPassword: (password: string) =>
    invoke<void>("setup_master_password", { password }),
  unlock: (password: string) => invoke<void>("unlock", { password }),
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
