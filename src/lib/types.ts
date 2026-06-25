export interface AppStatus {
  initialized: boolean;
  unlocked: boolean;
}

export interface AccountCredentials {
  email: string;
  refresh_token: string;
  client_id: string;
  auth_method: string;
}

export interface AccountInfo {
  email: string;
  auth_method: string;
  client_id: string;
  status: string;
  category_key: string | null;
  tag_keys: string[];
  health_score: number;
  health_summary: string;
  health_checked_at: string | null;
  created_at: string;
  updated_at: string;
  // 通知与后台刷新（阶段一D 起后端返回）
  notify_enabled: boolean;
  poll_interval_secs: number | null;
  last_sync_at: string | null;
}

export interface ClassificationOption {
  key: string;
  name_zh: string;
  name_en: string;
  remark: string | null;
  created_at: string | null;
}

export interface Catalog {
  categories: ClassificationOption[];
  tags: ClassificationOption[];
}

export interface ImportResult {
  added: number;
  total: number;
  errors: string[];
}

export interface EmailItem {
  message_id: string;
  folder: string;
  subject: string;
  from_email: string;
  date: string;
  is_read: boolean;
  has_attachments: boolean;
}

export interface EmailListResponse {
  email_id: string;
  folder_view: string;
  page: number;
  page_size: number;
  total_emails: number;
  emails: EmailItem[];
}

export interface EmailDetails {
  message_id: string;
  subject: string;
  from_email: string;
  to_email: string;
  date: string;
  body_plain: string | null;
  body_html: string | null;
}

export interface HealthResult {
  email: string;
  score: number;
  summary: string;
}

/** 最近邮件活动（后台刷新写入，供仪表盘统计） */
export interface MailActivityItem {
  email: string;
  message_id: string;
  subject: string;
  from_email: string;
  received_at: string;
}

/** 仪表盘统计 */
export interface DashboardStats {
  account_count: number;
  health_avg: number;
  healthy_count: number;
  unchecked_count: number;
  today_mail: number;
  recent: MailActivityItem[];
}

/** 应用级设置（后台刷新等；语言/主题在前端本地持久化） */
export interface AppSettings {
  bg_refresh_enabled: boolean;
  bg_refresh_interval_secs: number;
}
