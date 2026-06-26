<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t } from "../lib/i18n.svelte";
  import {
    quickview,
    toggleQuick,
    setCurrentQuick,
  } from "../lib/quickview.svelte";
  import { listen } from "@tauri-apps/api/event";
  import { appstate } from "../lib/appstate.svelte";
  import { senderLogo } from "../lib/brandLogos";
  import { extractCode } from "../lib/verifyCode";
  import Icon from "./Icon.svelte";
  import Spinner from "./Spinner.svelte";
  import Select, { type SelectOption } from "./Select.svelte";
  import type { AccountInfo, Catalog, EmailItem, EmailDetails } from "../lib/types";

  let { initialEmail, quick = false, openMessageId }: { initialEmail?: string; quick?: boolean; openMessageId?: string } = $props();

  let accounts = $state<AccountInfo[]>([]);
  let catalog = $state<Catalog>({ categories: [], tags: [] });
  let selectedEmail = $state<string>("");
  let folder = $state<"inbox" | "junk" | "all">("inbox");
  let page = $state(1);
  const pageSize = 50;

  let emails = $state<EmailItem[]>([]);
  let total = $state(0);
  let loading = $state(false);     // 首次加载（列表为空时）
  let refreshing = $state(false);  // 已有邮件时的刷新

  // 新邮件入场动画
  let animatingIds = $state<Set<string>>(new Set());
  let enteredIds = $state<Set<string>>(new Set());

  // 搜索 / 状态筛选
  let search = $state("");
  let statusFilter = $state<"all" | "unread" | "read">("all");

  // 详情模态
  let detailItem = $state<EmailItem | null>(null);
  let detail = $state<EmailDetails | null>(null);
  let detailLoading = $state(false);
  let detailTab = $state<"html" | "plain" | "raw">("html");

  // 竞态防护：仅最新请求可写入
  let reqId = 0;

  // 快捷查看模式：仅显示已固定的邮箱；详细查看：全部
  let visible = $derived(
    quick ? accounts.filter((a) => quickview.emails.includes(a.email)) : accounts,
  );

  let catNameMap = $derived(new Map(catalog.categories.map((c) => [c.key, c.name_zh])));
  let tagNameMap = $derived(new Map(catalog.tags.map((tg) => [tg.key, tg.name_zh])));

  let accountOptions = $derived<SelectOption[]>(
    visible.map((a) => ({
      value: a.email,
      label: a.email,
      sublabel: a.category_key ? catNameMap.get(a.category_key) ?? a.category_key : undefined,
      badges: a.tag_keys.map((k) => ({ text: tagNameMap.get(k) ?? k, tone: "soft" as const })),
    })),
  );

  let statusOptions = $derived<SelectOption[]>([
    { value: "all", label: t("mail.fAllStatus") },
    { value: "unread", label: t("mail.fUnread") },
    { value: "read", label: t("mail.fRead") },
  ]);

  let isPinned = $derived(quickview.emails.includes(selectedEmail));

  async function loadCatalog() {
    try {
      catalog = await api.getCatalog();
    } catch {
      /* ignore */
    }
  }

  async function init() {
    try {
      accounts = await api.listAccounts();
      loadCatalog();
      if (!selectedEmail) {
        if (quick) {
          selectedEmail =
            quickview.current && quickview.emails.includes(quickview.current)
              ? quickview.current
              : quickview.emails[0] ?? "";
        } else {
          selectedEmail = initialEmail ?? (accounts.length ? accounts[0].email : "");
        }
      }
      if (selectedEmail) {
        api.setActiveMailbox(selectedEmail).catch(() => {});
        await loadList();
        if (openMessageId) openByMessageId(openMessageId);
      }
    } catch (e) {
      showToast(errMsg(e), "error");
    }
  }
  init();

  // 找到对应邮件并打开详情（从锁屏「跳转打开」过来时用）
  async function openByMessageId(id: string) {
    const item = emails.find((m) => m.message_id === id);
    if (item) {
      openDetail(item);
      return;
    }
    openDetail({
      message_id: id,
      subject: "",
      from_email: "",
      date: "",
      folder: "",
      is_read: true,
      has_attachments: false,
    });
  }

  // 快捷查看：当前邮箱被移出固定集时，自动切到下一个
  $effect(() => {
    if (!quick || accounts.length === 0) return;
    if (selectedEmail && !visible.some((a) => a.email === selectedEmail)) {
      const next =
        (quickview.current && visible.some((a) => a.email === quickview.current)
          ? quickview.current
          : visible[0]?.email) ?? "";
      if (next) selectAccount(next);
      else {
        selectedEmail = "";
        emails = [];
        total = 0;
      }
    }
  });

  // 后台检测到本账号新邮件时刷新并播放入场动画
  $effect(() => {
    const un = listen<{ email: string; count: number }>("mail:new", (e) => {
      if (e.payload?.email === selectedEmail) loadList({ animate: page === 1 });
    });
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  });

  function animateNew(ids: string[]) {
    if (!ids.length) return;
    animatingIds = new Set(ids);
    enteredIds = new Set();
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        enteredIds = new Set(animatingIds);
      }),
    );
    setTimeout(() => {
      animatingIds = new Set();
      enteredIds = new Set();
    }, 720);
  }

  async function loadList(opts: { animate?: boolean } = {}) {
    if (!selectedEmail) return;
    const email = selectedEmail;
    const my = ++reqId;
    const hadEmails = emails.length > 0;
    if (hadEmails) refreshing = true;
    else loading = true;
    try {
      const res = await api.listEmails(email, folder, page, pageSize);
      if (my !== reqId || email !== selectedEmail) return; // 过期请求
      if (opts.animate && hadEmails) {
        const existing = new Set(emails.map((m) => m.message_id));
        const newIds = res.emails.filter((m) => !existing.has(m.message_id)).map((m) => m.message_id);
        emails = res.emails;
        total = res.total_emails;
        if (newIds.length) {
          animateNew(newIds);
          showToast(t("mail.newMailN", { n: newIds.length }), "ok");
        } else {
          showToast(t("mail.upToDate"), "info");
        }
      } else {
        emails = res.emails;
        total = res.total_emails;
      }
    } catch (e) {
      if (my !== reqId) return;
      if (!emails.length) {
        emails = [];
        total = 0;
      }
      showToast(errMsg(e), "error");
    } finally {
      if (my === reqId) {
        loading = false;
        refreshing = false;
      }
    }
  }

  function refresh() {
    loadList({ animate: page === 1 });
  }

  async function selectAccount(email: string) {
    if (email === selectedEmail) return;
    selectedEmail = email;
    page = 1;
    closeDetail();
    emails = [];
    total = 0;
    if (quick) setCurrentQuick(email);
    api.setActiveMailbox(email).catch(() => {});
    await loadList();
  }

  async function switchFolder(f: "inbox" | "junk" | "all") {
    folder = f;
    page = 1;
    emails = [];
    total = 0;
    await loadList();
  }

  function toggleQuickPin() {
    if (!selectedEmail) return;
    toggleQuick(selectedEmail);
    showToast(
      quickview.emails.includes(selectedEmail) ? t("mail.inQuick") : t("mail.removeQuick"),
      "ok",
    );
  }

  async function copyEmail() {
    if (!selectedEmail) return;
    try {
      await navigator.clipboard.writeText(selectedEmail);
      showToast(t("common.copied"), "ok");
    } catch {
      /* ignore */
    }
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

  // 每行附加品牌 LOGO 与（主题中的）验证码
  let rows = $derived(
    filtered.map((m) => ({
      m,
      logo: senderLogo(m.from_email),
      code: extractCode(m.subject),
    })),
  );

  // 详情里的验证码（含正文，识别更准）
  let detailCode = $derived(
    detail
      ? extractCode(detail.subject, detail.body_plain ?? "", (detail.body_html ?? "").replace(/<[^>]+>/g, " "))
      : null,
  );
  let detailLogo = $derived(detail ? senderLogo(detail.from_email) : null);

  async function copyText(e: Event | null, text: string | null | undefined) {
    e?.stopPropagation();
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      showToast(t("common.copied"), "ok");
    } catch {
      /* ignore */
    }
  }

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
      if (email !== selectedEmail) return;
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
    emails = [];
    await loadList();
  }
  async function prevPage() {
    if (page <= 1) return;
    page -= 1;
    emails = [];
    await loadList();
  }

  function withCsp(html: string): string {
    const m = '<meta http-equiv="Content-Security-Policy" content="img-src data: cid:">';
    const h = html.match(/(<head[^>]*>)/i);
    return h ? html.replace(h[1], h[1] + m) : m + html;
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
    <div class="acc-pick">
      <button class="ghost icon-btn copy-btn" onclick={copyEmail} disabled={!selectedEmail}
        title={t("mail.copyEmail")} aria-label={t("mail.copyEmail")}>
        <Icon name="copy" size={15} />
      </button>
      <Select
        value={selectedEmail}
        options={accountOptions}
        onchange={selectAccount}
        placeholder={t("mail.pickAccount")}
        width="260px"
        menuWidth="340px"
      />
    </div>
    <div class="tabs">
      <button class="sm" class:active={folder === "inbox"} onclick={() => switchFolder("inbox")}>{t("mail.inbox")}</button>
      <button class="sm" class:active={folder === "junk"} onclick={() => switchFolder("junk")}>{t("mail.junk")}</button>
      <button class="sm" class:active={folder === "all"} onclick={() => switchFolder("all")}>{t("mail.all")}</button>
    </div>
    <div class="search">
      <Icon name="search" size={15} />
      <input placeholder={t("mail.search")} bind:value={search} />
    </div>
    <Select bind:value={statusFilter} options={statusOptions} width="116px" />
    <button class="sm" onclick={refresh} disabled={loading || refreshing}>
      {#if refreshing}<Spinner size={15} />{:else}<Icon name="refresh" size={15} />{/if}
      {t("common.refresh")}
    </button>
    <button class="sm" class:active={isPinned} onclick={toggleQuickPin} disabled={!selectedEmail}>
      <Icon name="pin" size={15} /> {isPinned ? t("mail.removeQuick") : t("mail.addQuick")}
    </button>
  </header>

  <div class="stats">
    <div class="stat card"><span class="s-l">{t("mail.total")}</span><span class="s-v">{stats.total}</span></div>
    <div class="stat card"><span class="s-l">{t("mail.unread")}</span><span class="s-v">{stats.unread}</span></div>
    <div class="stat card"><span class="s-l">{t("mail.todayCount")}</span><span class="s-v">{stats.today}</span></div>
    <div class="stat card"><span class="s-l">{t("mail.attachments")}</span><span class="s-v">{stats.attach}</span></div>
  </div>

  <div class="list card">
    {#if loading && emails.length === 0}
      <p class="muted pad">{t("common.loading")}</p>
    {:else if !selectedEmail}
      <p class="muted pad">{t("mail.pickAccount")}</p>
    {:else if filtered.length === 0}
      <p class="muted pad">{t("mail.none")}</p>
    {:else}
      {#each rows as r (r.m.message_id)}
        <div
          class="email-item"
          class:unread={!r.m.is_read}
          class:anim={animatingIds.has(r.m.message_id)}
          class:anim-in={enteredIds.has(r.m.message_id)}
          role="button"
          tabindex="0"
          onclick={() => openDetail(r.m)}
          onkeydown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), openDetail(r.m))}
        >
          <span class="avatar" class:has-logo={!!r.logo}>
            {#if r.logo}
              <img src={r.logo} alt="" loading="lazy" />
            {:else}
              {initial(r.m.from_email)}
            {/if}
          </span>
          <span class="body">
            <span class="top">
              <span class="subject">{r.m.subject || "(无主题)"}</span>
              <span class="date muted">{fmtDate(r.m.date)}</span>
            </span>
            <span class="from muted">{senderEmail(r.m.from_email)}</span>
            <span class="preview muted small">
              {folder === "all" ? r.m.folder : (r.m.is_read ? t("mail.fRead") : t("mail.fUnread"))}{r.m.has_attachments ? " · " + t("mail.attachments") : ""}
            </span>
          </span>
          {#if r.code}
            <button class="code-chip" title={t("mail.copyCode")} onclick={(e) => copyText(e, r.code)}>
              <span class="code-val">{r.code}</span>
              <Icon name="copy" size={13} />
            </button>
          {/if}
        </div>
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
          {#if detailCode}
            <div class="code-banner">
              <span class="cb-label">{t("mail.codeDetected")}</span>
              <span class="cb-code">{detailCode}</span>
              <button class="sm" onclick={() => copyText(null, detailCode)}>
                <Icon name="copy" size={14} /> {t("common.copy")}
              </button>
            </div>
          {/if}
          <div class="meta">
            <div>
              <span class="muted">{t("mail.from")}</span>
              {#if detailLogo}<img class="from-logo" src={detailLogo} alt="" />{/if}
              {detail.from_email}
            </div>
            <div><span class="muted">{t("mail.to")}</span>{detail.to_email}</div>
            <div><span class="muted">{t("mail.time")}</span>{fmtDate(detail.date)}</div>
          </div>
          <div class="tabs det-tabs">
            {#if detail.body_html}<button class="sm" class:active={detailTab === "html"} onclick={() => (detailTab = "html")}>HTML</button>{/if}
            {#if detail.body_plain}<button class="sm" class:active={detailTab === "plain"} onclick={() => (detailTab = "plain")}>{t("mail.tabPlain")}</button>{/if}
            {#if detail.body_html}<button class="sm" class:active={detailTab === "raw"} onclick={() => (detailTab = "raw")}>{t("mail.tabRaw")}</button>{/if}
          </div>
          {#if detailTab === "html" && detail.body_html}
            <iframe class="frame" sandbox="allow-same-origin" srcdoc={appstate.blockRemoteImages ? withCsp(detail.body_html) : detail.body_html} title="mail"></iframe>
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
  .acc-pick {
    display: flex;
    align-items: center;
    gap: var(--s-xs);
  }
  .copy-btn {
    width: 34px;
    padding: 0;
    flex-shrink: 0;
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
  .bar .active {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--on-primary);
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
    align-items: center;
    width: 100%;
    height: auto;
    text-align: left;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--r-md);
    padding: var(--s-sm);
    cursor: pointer;
  }
  .email-item:hover {
    background: var(--canvas-soft);
    border-color: var(--hairline);
  }
  .email-item:focus-visible {
    outline: 2px solid var(--link);
    outline-offset: 1px;
  }
  .email-item.unread {
    border-left: 3px solid var(--link);
  }
  /* 新邮件入场动画 */
  .email-item.anim {
    overflow: hidden;
    opacity: 0;
    max-height: 0;
    padding-top: 0;
    padding-bottom: 0;
    transform: translateY(-18px);
    transition:
      max-height 0.42s cubic-bezier(0.22, 1, 0.36, 1),
      opacity 0.28s ease,
      transform 0.32s ease,
      padding 0.32s ease;
  }
  .email-item.anim.anim-in {
    opacity: 1;
    max-height: 240px;
    padding-top: var(--s-sm);
    padding-bottom: var(--s-sm);
    transform: translateY(0);
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
    overflow: hidden;
  }
  /* 品牌 LOGO：白底圆角，深浅主题都清晰可见 */
  .avatar.has-logo {
    background: #fff;
    padding: 5px;
    box-shadow: 0 0 0 1px var(--hairline) inset;
  }
  .avatar img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    display: block;
    border-radius: var(--r-sm);
  }
  .code-chip {
    align-self: center;
    flex-shrink: 0;
    height: 30px;
    padding: 0 10px;
    gap: 6px;
    border-radius: var(--r-full);
    background: var(--link-bg-soft);
    border: 1px solid transparent;
    color: var(--link-deep);
  }
  .code-chip:hover {
    border-color: var(--link);
    background: var(--link-bg-soft);
    filter: none;
  }
  .code-val {
    font-family: var(--font-mono);
    font-weight: 700;
    letter-spacing: 1px;
    font-size: 13px;
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
  .from-logo {
    width: 16px;
    height: 16px;
    object-fit: contain;
    vertical-align: middle;
    margin-right: 5px;
    border-radius: 3px;
    background: #fff;
  }
  .code-banner {
    display: flex;
    align-items: center;
    gap: var(--s-sm);
    padding: var(--s-sm) var(--s-md);
    background: var(--link-bg-soft);
    border-radius: var(--r-md);
  }
  .cb-label {
    font-size: 12px;
    color: var(--link-deep);
    white-space: nowrap;
  }
  .cb-code {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 20px;
    font-weight: 700;
    letter-spacing: 2px;
    color: var(--ink);
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
