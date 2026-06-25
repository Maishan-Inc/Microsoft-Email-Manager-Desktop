<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t } from "../lib/i18n.svelte";
  import type { AccountInfo, EmailItem, EmailDetails } from "../lib/types";

  let { initialEmail }: { initialEmail?: string } = $props();

  let accounts = $state<AccountInfo[]>([]);
  let selectedEmail = $state<string>(initialEmail ?? "");
  let folder = $state<"inbox" | "junk" | "all">("inbox");
  let page = $state(1);
  const pageSize = 25;

  let emails = $state<EmailItem[]>([]);
  let total = $state(0);
  let loading = $state(false);

  let detail = $state<EmailDetails | null>(null);
  let detailLoading = $state(false);

  async function init() {
    try {
      accounts = await api.listAccounts();
      if (!selectedEmail && accounts.length) selectedEmail = accounts[0].email;
      if (selectedEmail) await loadList();
    } catch (e) {
      showToast(errMsg(e), "error");
    }
  }
  init();

  async function loadList() {
    if (!selectedEmail) return;
    loading = true;
    detail = null;
    try {
      const res = await api.listEmails(selectedEmail, folder, page, pageSize);
      emails = res.emails;
      total = res.total_emails;
    } catch (e) {
      emails = [];
      total = 0;
      showToast(errMsg(e), "error");
    } finally {
      loading = false;
    }
  }

  async function switchAccount(email: string) {
    selectedEmail = email;
    page = 1;
    await loadList();
  }
  async function switchFolder(f: "inbox" | "junk" | "all") {
    folder = f;
    page = 1;
    await loadList();
  }
  async function openDetail(item: EmailItem) {
    detailLoading = true;
    detail = null;
    try {
      detail = await api.getEmailDetails(selectedEmail, item.message_id);
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      detailLoading = false;
    }
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
  function fmtDate(s: string): string {
    if (!s) return "";
    const d = new Date(s);
    return isNaN(d.getTime()) ? s : d.toLocaleString();
  }
</script>

<div class="mail">
  <aside class="accounts-col card">
    <div class="col-title">{t("nav.accounts")}</div>
    {#if accounts.length === 0}
      <p class="muted small">{t("mail.pickAccount")}</p>
    {:else}
      {#each accounts as a (a.email)}
        <button class="acc {selectedEmail === a.email ? 'active' : ''}" onclick={() => switchAccount(a.email)}>
          <span class="acc-mail">{a.email}</span>
          <span class="badge">{a.auth_method}</span>
        </button>
      {/each}
    {/if}
  </aside>

  <section class="list-col card">
    <div class="tabs">
      <button class="sm" class:active={folder === "inbox"} onclick={() => switchFolder("inbox")}>{t("mail.inbox")}</button>
      <button class="sm" class:active={folder === "junk"} onclick={() => switchFolder("junk")}>{t("mail.junk")}</button>
      <button class="sm" class:active={folder === "all"} onclick={() => switchFolder("all")}>{t("mail.all")}</button>
      <span class="spacer"></span>
      <button class="sm" onclick={loadList} disabled={loading}>{t("common.refresh")}</button>
    </div>

    {#if loading}
      <p class="muted">{t("common.loading")}</p>
    {:else if emails.length === 0}
      <p class="muted">{t("mail.none")}</p>
    {:else}
      <ul class="emails">
        {#each emails as m (m.message_id)}
          <li>
            <button class="email-row" onclick={() => openDetail(m)}>
              <div class="row-top">
                <span class="from" class:unread={!m.is_read}>{m.from_email}</span>
                <span class="date muted">{fmtDate(m.date)}</span>
              </div>
              <div class="subject" class:unread={!m.is_read}>{m.subject}</div>
            </button>
          </li>
        {/each}
      </ul>
      <div class="pager">
        <button class="sm" onclick={prevPage} disabled={page <= 1}>{t("mail.prev")}</button>
        <span class="muted small">{t("mail.pageInfo", { page, total })}</span>
        <button class="sm" onclick={nextPage} disabled={page * pageSize >= total}>{t("mail.nextPage")}</button>
      </div>
    {/if}
  </section>

  <section class="detail-col card">
    {#if detailLoading}
      <p class="muted">{t("common.loading")}</p>
    {:else if detail}
      <h3>{detail.subject}</h3>
      <div class="meta muted small">
        <div>{t("mail.from")}: {detail.from_email}</div>
        <div>{t("mail.to")}: {detail.to_email}</div>
        <div>{t("mail.time")}: {fmtDate(detail.date)}</div>
      </div>
      <hr />
      {#if detail.body_html}
        <iframe class="body-frame" sandbox="allow-same-origin" srcdoc={detail.body_html} title="mail"></iframe>
      {:else if detail.body_plain}
        <pre class="body-plain">{detail.body_plain}</pre>
      {:else}
        <p class="muted">—</p>
      {/if}
    {:else}
      <p class="muted">{t("mail.selectOne")}</p>
    {/if}
  </section>
</div>

<style>
  .mail {
    display: grid;
    grid-template-columns: 210px 330px 1fr;
    gap: var(--s-sm);
    height: 100%;
  }
  .card {
    overflow: auto;
    padding: var(--s-sm);
  }
  .col-title {
    font-weight: 600;
    margin-bottom: var(--s-xs);
    font-size: 13px;
  }
  .acc {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    width: 100%;
    height: auto;
    text-align: left;
    margin-bottom: 6px;
    border: 1px solid transparent;
    background: transparent;
    padding: var(--s-xs);
  }
  .acc.active {
    background: var(--canvas-soft);
    border-color: var(--hairline);
  }
  .acc-mail {
    font-size: 12px;
    word-break: break-all;
  }
  .tabs {
    display: flex;
    gap: 6px;
    align-items: center;
    margin-bottom: var(--s-sm);
  }
  .tabs .active {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--on-primary);
  }
  .spacer {
    flex: 1;
  }
  .emails {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .email-row {
    width: 100%;
    height: auto;
    text-align: left;
    border: none;
    border-bottom: 1px solid var(--hairline);
    border-radius: 0;
    background: transparent;
    padding: var(--s-sm) 6px;
    display: block;
  }
  .email-row:hover {
    background: var(--canvas-soft);
  }
  .row-top {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .from {
    font-size: 13px;
  }
  .subject {
    font-size: 13px;
    color: var(--mute);
    margin-top: 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .unread {
    color: var(--ink);
    font-weight: 600;
  }
  .date {
    font-size: 11px;
    white-space: nowrap;
  }
  .pager {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: var(--s-sm);
  }
  .detail-col h3 {
    margin: 0 0 8px;
    font-size: 16px;
  }
  .meta div {
    margin: 2px 0;
  }
  .body-frame {
    width: 100%;
    height: calc(100% - 120px);
    min-height: 360px;
    border: 1px solid var(--hairline);
    border-radius: var(--r-md);
    background: #fff;
  }
  .body-plain {
    white-space: pre-wrap;
    word-break: break-word;
    font-family: inherit;
    font-size: 13px;
  }
  hr {
    border: none;
    border-top: 1px solid var(--hairline);
    margin: var(--s-sm) 0;
  }
</style>
