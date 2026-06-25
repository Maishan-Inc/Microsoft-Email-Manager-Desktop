<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import type { AccountInfo, EmailItem, EmailDetails } from "../lib/types";

  let accounts = $state<AccountInfo[]>([]);
  let selectedEmail = $state<string>("");
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
      if (accounts.length && !selectedEmail) {
        selectedEmail = accounts[0].email;
        await loadList();
      }
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
      showToast("加载邮件失败：" + errMsg(e), "error");
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
      showToast("加载详情失败：" + errMsg(e), "error");
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
    <div class="col-title">账号</div>
    {#if accounts.length === 0}
      <p class="muted small">请先在「账号管理」添加账号</p>
    {:else}
      {#each accounts as a (a.email)}
        <button
          class="acc {selectedEmail === a.email ? 'active' : ''}"
          onclick={() => switchAccount(a.email)}
        >
          <span class="acc-mail">{a.email}</span>
          <span class="badge">{a.auth_method}</span>
        </button>
      {/each}
    {/if}
  </aside>

  <section class="list-col card">
    <div class="tabs">
      <button class:active={folder === "inbox"} onclick={() => switchFolder("inbox")}>
        收件箱
      </button>
      <button class:active={folder === "junk"} onclick={() => switchFolder("junk")}>
        垃圾箱
      </button>
      <button class:active={folder === "all"} onclick={() => switchFolder("all")}>
        全部
      </button>
      <span class="spacer"></span>
      <button onclick={loadList} disabled={loading}>刷新</button>
    </div>

    {#if loading}
      <p class="muted">加载中…</p>
    {:else if emails.length === 0}
      <p class="muted">暂无邮件</p>
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
        <button onclick={prevPage} disabled={page <= 1}>上一页</button>
        <span class="muted small">第 {page} 页 / 共 {total} 封</span>
        <button onclick={nextPage} disabled={page * pageSize >= total}>下一页</button>
      </div>
    {/if}
  </section>

  <section class="detail-col card">
    {#if detailLoading}
      <p class="muted">加载详情…</p>
    {:else if detail}
      <h3>{detail.subject}</h3>
      <div class="meta muted small">
        <div>发件人：{detail.from_email}</div>
        <div>收件人：{detail.to_email}</div>
        <div>时间：{fmtDate(detail.date)}</div>
      </div>
      <hr />
      {#if detail.body_html}
        <iframe
          class="body-frame"
          sandbox="allow-same-origin"
          srcdoc={detail.body_html}
          title="邮件正文"
        ></iframe>
      {:else if detail.body_plain}
        <pre class="body-plain">{detail.body_plain}</pre>
      {:else}
        <p class="muted">（无正文）</p>
      {/if}
    {:else}
      <p class="muted">选择一封邮件查看详情</p>
    {/if}
  </section>
</div>

<style>
  .mail {
    display: grid;
    grid-template-columns: 200px 320px 1fr;
    gap: 12px;
    height: 100%;
  }
  .card {
    overflow: auto;
    padding: 12px;
  }
  .col-title {
    font-weight: 600;
    margin-bottom: 8px;
  }
  .acc {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    width: 100%;
    text-align: left;
    margin-bottom: 6px;
    border: 1px solid transparent;
    background: transparent;
  }
  .acc.active {
    background: var(--panel-2);
    border-color: var(--accent);
  }
  .acc-mail {
    font-size: 12px;
    word-break: break-all;
  }
  .tabs {
    display: flex;
    gap: 6px;
    align-items: center;
    margin-bottom: 10px;
  }
  .tabs .active {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
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
    text-align: left;
    border: none;
    border-bottom: 1px solid var(--border);
    border-radius: 0;
    background: transparent;
    padding: 10px 6px;
  }
  .email-row:hover {
    background: var(--panel-2);
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
    color: var(--muted);
    margin-top: 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .unread {
    color: var(--text);
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
    margin-top: 10px;
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
    border: 1px solid var(--border);
    border-radius: 8px;
    background: #fff;
  }
  .body-plain {
    white-space: pre-wrap;
    word-break: break-word;
    font-family: inherit;
    font-size: 13px;
  }
  .small {
    font-size: 12px;
  }
  hr {
    border: none;
    border-top: 1px solid var(--border);
    margin: 10px 0;
  }
</style>
