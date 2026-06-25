<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t } from "../lib/i18n.svelte";
  import { setQuick } from "../lib/quickview.svelte";
  import { listen } from "@tauri-apps/api/event";
  import Icon from "./Icon.svelte";
  import type { AccountInfo, EmailItem, EmailDetails } from "../lib/types";

  let { initialEmail }: { initialEmail?: string } = $props();

  let accounts = $state<AccountInfo[]>([]);
  let selectedEmail = $state<string>("");
  let folder = $state<"inbox" | "junk" | "all">("inbox");
  let page = $state(1);
  const pageSize = 50;

  let emails = $state<EmailItem[]>([]);
  let total = $state(0);
  let loading = $state(false);

  // 搜索 / 状态筛选
  let search = $state("");
  let statusFilter = $state<"all" | "unread" | "read">("all");

  // 详情模态
  let detailItem = $state<EmailItem | null>(null);
  let detail = $state<EmailDetails | null>(null);
  let detailLoading = $state(false);
  let detailTab = $state<"html" | "plain" | "raw">("html");

  // 竞态防护：仅最新请求可写入，避免切换账号时窜邮件
  let reqId = 0;

  async function init() {
    try {
      accounts = await api.listAccounts();
      if (!selectedEmail) selectedEmail = initialEmail ?? (accounts.length ? accounts[0].email : "");
      if (selectedEmail) await loadList();
    } catch (e) {
      showToast(errMsg(e), "error");
    }
  }
  init();

  // 后台检测到本账号新邮件时自动刷新（独立邮件页/快捷查看页都生效）
  $effect(() => {
    const un = listen<{ email: string; count: number }>("mail:new", (e) => {
      if (e.payload?.email === selectedEmail) loadList();
    });
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  });

  function pinQuick() {
    if (!selectedEmail) return;
    setQuick(selectedEmail);
    showToast(t("mail.openInQuick"), "ok");
  }

  async function loadList() {
    if (!selectedEmail) return;
    const email = selectedEmail;
    const my = ++reqId;
    loading = true;
    try {
      const res = await api.listEmails(email, folder, page, pageSize);
      if (my !== reqId || email !== selectedEmail) return; // 过期请求，丢弃
      emails = res.emails;
      total = res.total_emails;
    } catch (e) {
      if (my !== reqId) return;
      emails = [];
      total = 0;
      showToast(errMsg(e), "error");
    } finally {
      if (my === reqId) loading = false;
    }
  }

  async function switchAccount(email: string) {
    selectedEmail = email;
    page = 1;
    closeDetail();
    await loadList();
  }
  async function switchFolder(f: "inbox" | "junk" | "all") {
    folder = f;
    page = 1;
    await loadList();
  }

  let filtered = $derived(
    emails.filter((m) => {
      const q = search.trim().toLowerCase();
      if (q && !`${m.subject} ${m.from_email}`.toLowerCase().includes(q)) return false;
      if (statusFilter === "read" && !m.is_read) return false;
      if (statusFilter === "unread" && m.is_read) return false;
      return true;
    }),
  );

  let stats = $derived({
    total,
    unread: emails.filter((m) => !m.is_read).length,
    today: emails.filter((m) => {
      const d = new Date(m.date);
      return !isNaN(d.getTime()) && d.toDateString() === new Date().toDateString();
    }).length,
    attach: emails.filter((m) => m.has_attachments).length,
  });

  async function openDetail(item: EmailItem) {
    const email = selectedEmail;
    detailItem = item;
    detail = null;
    detailTab = "html";
    detailLoading = true;
    try {
      const d = await api.getEmailDetails(email, item.message_id);
      if (email !== selectedEmail) return; // 账号已切换，丢弃
      detail = d;
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      detailLoading = false;
    }
  }
  function closeDetail() {
    detailItem = null;
    detail = null;
  }

  async function nextPage() {
    if (page * pageSize >= total) return;
    page += 1;
    await loadList();
  }
  async function prevPage() {
    if (page <= 1) return;
    page -= 1;
    await loadList();
  }

  function senderEmail(from: string): string {
    const m = from.match(/<([^>]+)>/);
    return (m ? m[1] : from).trim();
  }
  function initial(from: string): string {
    const e = senderEmail(from) || "?";
    return (e[0] || "?").toUpperCase();
  }
  function fmtDate(s: string): string {
    if (!s) return "";
    const d = new Date(s);
    return isNaN(d.getTime()) ? s : d.toLocaleString();
  }
</script>

<div class="mailview">
  <header class="bar card">
    <select class="acc-sel" value={selectedEmail} onchange={(e) => switchAccount((e.target as HTMLSelectElement).value)}>
      {#if accounts.length === 0}
        <option value="">{t("mail.pickAccount")}</option>
      {/if}
      {#each accounts as a (a.email)}
        <option value={a.email}>{a.email}</option>
      {/each}
    </select>
    <div class="tabs">
      <button class="sm" class:active={folder === "inbox"} onclick={() => switchFolder("inbox")}>{t("mail.inbox")}</button>
      <button class="sm" class:active={folder === "junk"} onclick={() => switchFolder("junk")}>{t("mail.junk")}</button>
      <button class="sm" class:active={folder === "all"} onclick={() => switchFolder("all")}>{t("mail.all")}</button>
    </div>
    <div class="search">
      <Icon name="search" size={15} />
      <input placeholder={t("mail.search")} bind:value={search} />
    </div>
    <select class="flt" bind:value={statusFilter}>
      <option value="all">{t("mail.fAllStatus")}</option>
      <option value="unread">{t("mail.fUnread")}</option>
      <option value="read">{t("mail.fRead")}</option>
    </select>
    <button class="sm" onclick={loadList} disabled={loading}><Icon name="refresh" size={15} /> {t("common.refresh")}</button>
    <button class="sm" onclick={pinQuick} disabled={!selectedEmail}><Icon name="pin" size={15} /> {t("mail.openInQuick")}</button>
  </header>

  <div class="stats">
    <div class="stat card"><span class="s-l">{t("mail.total")}</span><span class="s-v">{stats.total}</span></div>
    <div class="stat card"><span class="s-l">{t("mail.unread")}</span><span class="s-v">{stats.unread}</span></div>
    <div class="stat card"><span class="s-l">{t("mail.todayCount")}</span><span class="s-v">{stats.today}</span></div>
    <div class="stat card"><span class="s-l">{t("mail.attachments")}</span><span class="s-v">{stats.attach}</span></div>
  </div>

  <div class="list card">
    {#if loading}
      <p class="muted pad">{t("common.loading")}</p>
    {:else if !selectedEmail}
      <p class="muted pad">{t("mail.pickAccount")}</p>
    {:else if filtered.length === 0}
      <p class="muted pad">{t("mail.none")}</p>
    {:else}
      {#each filtered as m (m.message_id)}
        <button class="email-item" class:unread={!m.is_read} onclick={() => openDetail(m)}>
          <span class="avatar">{initial(m.from_email)}</span>
          <span class="body">
            <span class="top">
              <span class="subject">{m.subject || "(无主题)"}</span>
              <span class="date muted">{fmtDate(m.date)}</span>
            </span>
            <span class="from muted">{senderEmail(m.from_email)}</span>
            <span class="preview muted small">
              {folder === "all" ? m.folder : (m.is_read ? t("mail.fRead") : t("mail.fUnread"))}{m.has_attachments ? " · " + t("mail.attachments") : ""}
            </span>
          </span>
        </button>
      {/each}
    {/if}
  </div>

  {#if total > pageSize}
    <div class="pager">
      <button class="sm" onclick={prevPage} disabled={page <= 1}>{t("mail.prev")}</button>
      <span class="muted small">{t("mail.pageInfo", { page, total })}</span>
      <button class="sm" onclick={nextPage} disabled={page * pageSize >= total}>{t("mail.nextPage")}</button>
    </div>
  {/if}
</div>

{#if detailItem}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && closeDetail()}>
    <div class="modal" role="dialog" aria-modal="true">
      <header class="m-head">
        <h3>{detail?.subject || detailItem.subject || t("mail.detail")}</h3>
        <button class="ghost icon-btn" onclick={closeDetail} aria-label="close"><Icon name="x" size={18} /></button>
      </header>
      <div class="m-body">
        {#if detailLoading}
          <p class="muted">{t("common.loading")}</p>
        {:else if detail}
          <div class="meta">
            <div><span class="muted">{t("mail.from")}</span>{detail.from_email}</div>
            <div><span class="muted">{t("mail.to")}</span>{detail.to_email}</div>
            <div><span class="muted">{t("mail.time")}</span>{fmtDate(detail.date)}</div>
          </div>
          <div class="tabs det-tabs">
            {#if detail.body_html}<button class="sm" class:active={detailTab === "html"} onclick={() => (detailTab = "html")}>HTML</button>{/if}
            {#if detail.body_plain}<button class="sm" class:active={detailTab === "plain"} onclick={() => (detailTab = "plain")}>{t("mail.tabPlain")}</button>{/if}
            {#if detail.body_html}<button class="sm" class:active={detailTab === "raw"} onclick={() => (detailTab = "raw")}>{t("mail.tabRaw")}</button>{/if}
          </div>
          {#if detailTab === "html" && detail.body_html}
            <iframe class="frame" sandbox="allow-same-origin" srcdoc={detail.body_html} title="mail"></iframe>
          {:else if detailTab === "plain" && detail.body_plain}
            <pre class="plain">{detail.body_plain}</pre>
          {:else if detailTab === "raw" && detail.body_html}
            <pre class="plain raw">{detail.body_html}</pre>
          {:else if detail.body_plain}
            <pre class="plain">{detail.body_plain}</pre>
          {:else}
            <p class="muted">{t("mail.noContent")}</p>
          {/if}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .mailview {
    display: flex;
    flex-direction: column;
    gap: var(--s-sm);
    height: 100%;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: var(--s-sm);
    padding: var(--s-xs) var(--s-sm);
    flex-wrap: wrap;
  }
  .acc-sel {
    width: auto;
    min-width: 220px;
    max-width: 320px;
    height: 36px;
  }
  .tabs {
    display: flex;
    gap: 4px;
  }
  .tabs .active {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--on-primary);
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--s-xs);
    flex: 1;
    min-width: 160px;
    color: var(--mute);
  }
  .search input {
    border: none;
    background: transparent;
    height: 32px;
    padding: 0;
  }
  .search input:focus {
    box-shadow: none;
  }
  .flt {
    width: auto;
    min-width: 110px;
    height: 36px;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--s-sm);
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--s-sm) var(--s-md);
  }
  .s-l {
    font-size: 12px;
    color: var(--mute);
  }
  .s-v {
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .list {
    flex: 1;
    overflow: auto;
    padding: var(--s-xs);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .pad {
    padding: var(--s-lg);
  }
  .email-item {
    display: flex;
    gap: var(--s-sm);
    align-items: flex-start;
    width: 100%;
    height: auto;
    text-align: left;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--r-md);
    padding: var(--s-sm);
  }
  .email-item:hover {
    background: var(--canvas-soft);
    border-color: var(--hairline);
  }
  .email-item.unread {
    border-left: 3px solid var(--link);
  }
  .avatar {
    width: 40px;
    height: 40px;
    border-radius: var(--r-md);
    background: var(--canvas-soft-2);
    display: grid;
    place-items: center;
    font-weight: 600;
    font-size: 16px;
    flex-shrink: 0;
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .top {
    display: flex;
    justify-content: space-between;
    gap: var(--s-sm);
  }
  .subject {
    font-size: 14px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .email-item.unread .subject {
    font-weight: 700;
  }
  .date {
    font-size: 12px;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .from {
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pager {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  /* modal */
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: rgba(15, 23, 42, 0.42);
    display: grid;
    place-items: center;
    padding: var(--s-lg);
  }
  .modal {
    width: 960px;
    max-width: 100%;
    max-height: calc(100vh - 48px);
    background: var(--canvas);
    border-radius: var(--r-xl);
    box-shadow: var(--shadow-5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .m-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-sm);
    padding: var(--s-md) var(--s-lg);
    border-bottom: 1px solid var(--hairline);
  }
  .m-head h3 {
    margin: 0;
    font-size: 17px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .icon-btn {
    width: 34px;
    padding: 0;
    flex-shrink: 0;
  }
  .m-body {
    padding: var(--s-lg);
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: var(--s-sm);
  }
  .meta {
    display: grid;
    gap: 4px;
    font-size: 13px;
  }
  .meta .muted {
    display: inline-block;
    width: 64px;
  }
  .det-tabs {
    border-bottom: 1px solid var(--hairline);
    padding-bottom: var(--s-xs);
  }
  .det-tabs .active {
    background: var(--canvas-soft);
    border-color: var(--hairline);
  }
  .frame {
    width: 100%;
    min-height: 420px;
    flex: 1;
    border: 1px solid var(--hairline);
    border-radius: var(--r-md);
    background: #fff;
  }
  .plain {
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 13px;
    background: var(--canvas-soft);
    border-radius: var(--r-md);
    padding: var(--s-md);
    margin: 0;
    max-height: 60vh;
    overflow: auto;
  }
  .raw {
    font-family: var(--font-mono);
  }
  @media (max-width: 820px) {
    .stats {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
