<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";
  import AccountConfigModal from "./AccountConfigModal.svelte";
  import type { AccountInfo, Catalog } from "../lib/types";

  let { onopenmail }: { onopenmail: (email: string) => void } = $props();

  let accounts = $state<AccountInfo[]>([]);
  let catalog = $state<Catalog>({ categories: [], tags: [] });
  let loading = $state(false);
  let testingEmail = $state<string | null>(null);
  let checkingEmail = $state<string | null>(null);
  let togglingEmail = $state<string | null>(null);

  // 搜索 / 筛选
  let searchTerm = $state("");
  let domainFilter = $state("");
  let categoryFilter = $state("");
  let tagFilter = $state("");

  // 批量
  let batchMode = $state(false);
  let selected = $state<Set<string>>(new Set());

  // 配置弹窗
  let configEmail = $state<string | null>(null);
  let configAccount = $derived(accounts.find((a) => a.email === configEmail) ?? null);

  async function refresh() {
    loading = true;
    try {
      accounts = await api.listAccounts();
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      loading = false;
    }
  }
  async function loadCatalog() {
    try {
      catalog = await api.getCatalog();
    } catch {
      /* ignore */
    }
  }
  refresh();
  loadCatalog();

  let domains = $derived(
    [...new Set(accounts.map((a) => a.email.split("@")[1]).filter(Boolean))].sort(),
  );
  let filtered = $derived(
    accounts.filter((a) => {
      const q = searchTerm.trim().toLowerCase();
      if (q && !a.email.toLowerCase().includes(q)) return false;
      if (domainFilter && a.email.split("@")[1] !== domainFilter) return false;
      if (categoryFilter && a.category_key !== categoryFilter) return false;
      if (tagFilter && !a.tag_keys.includes(tagFilter)) return false;
      return true;
    }),
  );

  function healthClass(a: AccountInfo): string {
    if (!a.health_checked_at) return "";
    if (a.health_score >= 100) return "ok";
    if (a.health_score >= 60) return "warn";
    return "bad";
  }
  function fmt(s: string | null): string {
    if (!s) return t("acc.never");
    const d = new Date(s);
    return isNaN(d.getTime()) ? s : d.toLocaleDateString();
  }

  // 批量选择
  function toggleSelect(email: string) {
    const s = new Set(selected);
    s.has(email) ? s.delete(email) : s.add(email);
    selected = s;
  }
  function selectAll() {
    selected = new Set(filtered.map((a) => a.email));
  }
  function clearSel() {
    selected = new Set();
  }
  function exitBatch() {
    batchMode = false;
    clearSel();
  }
  async function batchDelete() {
    if (!selected.size) return;
    if (!confirm(`${t("acc.deleteSel")} (${selected.size})?`)) return;
    let failed = 0;
    for (const email of selected) {
      try {
        await api.deleteAccount(email);
      } catch {
        failed++;
      }
    }
    showToast(t("acc.deleteSel"), failed ? "info" : "ok");
    clearSel();
    await refresh();
  }

  function rowClick(e: MouseEvent, email: string) {
    if ((e.target as HTMLElement).closest(".actions, .row-check")) return;
    if (batchMode) toggleSelect(email);
    else onopenmail(email);
  }

  async function test(email: string) {
    testingEmail = email;
    try {
      await api.testAccount(email);
      showToast(email, "ok");
    } catch (e) {
      showToast(email + ": " + errMsg(e), "error");
    } finally {
      testingEmail = null;
    }
  }
  async function checkHealth(email: string) {
    checkingEmail = email;
    try {
      const res = await api.checkAccountHealth(email);
      const a = accounts.find((x) => x.email === email);
      if (a) {
        a.health_summary = res.summary;
        a.health_score = res.score;
        a.health_checked_at = new Date().toISOString();
      }
      showToast(`${email}: ${res.summary}`, res.score >= 100 ? "ok" : "info");
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      checkingEmail = null;
    }
  }
  async function toggleNotify(a: AccountInfo) {
    togglingEmail = a.email;
    const next = !a.notify_enabled;
    try {
      await api.setAccountNotify(a.email, next, null);
      a.notify_enabled = next;
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      togglingEmail = null;
    }
  }
  async function del(email: string) {
    if (!confirm(`${t("common.delete")} ${email}?`)) return;
    try {
      await api.deleteAccount(email);
      showToast(t("common.delete"), "ok");
      await refresh();
    } catch (e) {
      showToast(errMsg(e), "error");
    }
  }
</script>

<div class="view">
  <header class="view-head">
    <h1>{t("acc.title")} <span class="muted count">{t("acc.count", { n: accounts.length })}</span></h1>
    <div class="row">
      <button class:active={batchMode} onclick={() => (batchMode ? exitBatch() : (batchMode = true))}>
        {batchMode ? t("acc.batchExit") : t("acc.batch")}
      </button>
      <button onclick={refresh} disabled={loading}><Icon name="refresh" size={15} /> {t("common.refresh")}</button>
    </div>
  </header>

  <!-- 搜索 + 筛选 -->
  <div class="filters card">
    <div class="search">
      <Icon name="search" size={16} />
      <input placeholder={t("acc.search")} bind:value={searchTerm} />
    </div>
    <select bind:value={domainFilter} class="flt">
      <option value="">{t("acc.allDomains")}</option>
      {#each domains as d (d)}<option value={d}>@{d}</option>{/each}
    </select>
    <select bind:value={categoryFilter} class="flt">
      <option value="">{t("acc.allCategories")}</option>
      {#each catalog.categories as c (c.key)}<option value={c.key}>{c.name_zh}</option>{/each}
    </select>
    <select bind:value={tagFilter} class="flt">
      <option value="">{t("acc.allTags")}</option>
      {#each catalog.tags as tg (tg.key)}<option value={tg.key}>{tg.name_zh}</option>{/each}
    </select>
  </div>

  {#if batchMode}
    <div class="batchbar card">
      <span>{t("acc.selectedN", { n: selected.size })}</span>
      <button class="sm" onclick={selectAll}>{t("acc.selectAll")}</button>
      <button class="sm" onclick={clearSel}>{t("acc.clearSel")}</button>
      <span class="spacer"></span>
      <button class="sm danger" disabled={!selected.size} onclick={batchDelete}>
        <Icon name="trash" size={14} /> {t("acc.deleteSel")}
      </button>
    </div>
  {/if}

  {#if loading}
    <p class="muted">{t("common.loading")}</p>
  {:else if accounts.length === 0}
    <div class="card empty muted">{t("acc.empty")}</div>
  {:else if filtered.length === 0}
    <div class="card empty muted">{t("acc.noMatch")}</div>
  {:else}
    <div class="rows">
      {#each filtered as a (a.email)}
        <div
          class="acc-row card"
          class:sel={batchMode && selected.has(a.email)}
          role="button"
          tabindex="0"
          onclick={(e) => rowClick(e, a.email)}
          onkeydown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), rowClick(e as unknown as MouseEvent, a.email))}
        >
          {#if batchMode}
            <input class="row-check" type="checkbox" checked={selected.has(a.email)} onchange={() => toggleSelect(a.email)} />
          {:else}
            <span class="dot {a.status === 'active' ? 'on' : 'off'}"></span>
          {/if}
          <div class="id">
            <span class="mail">{a.email}</span>
            <span class="meta">
              <span class="badge">{a.auth_method}</span>
              {#if a.health_checked_at}
                <span class="pill {healthClass(a)}">{a.health_score}</span>
              {:else}
                <span class="muted small">{t("dash.unchecked")}</span>
              {/if}
              {#if a.notify_enabled}<Icon name="bell" size={13} />{/if}
            </span>
          </div>

          <span class="last muted small">{t("acc.lastCheck")}: {fmt(a.health_checked_at)}</span>

          <div class="actions">
            <button class="ghost sm" class:active={a.notify_enabled} title={t("acc.notify")}
              disabled={togglingEmail === a.email} onclick={() => toggleNotify(a)} aria-label={t("acc.notify")}>
              <Icon name={a.notify_enabled ? "bell" : "bell-off"} size={16} />
            </button>
            <button class="sm" onclick={() => (configEmail = a.email)} aria-label={t("acc.config")}>
              <Icon name="settings" size={15} />
            </button>
            <button class="sm" onclick={() => test(a.email)} disabled={testingEmail === a.email}>
              {testingEmail === a.email ? t("acc.testing") : t("acc.test")}
            </button>
            <button class="sm" onclick={() => checkHealth(a.email)} disabled={checkingEmail === a.email}>
              {checkingEmail === a.email ? t("acc.checking") : t("acc.health")}
            </button>
            <button class="sm danger ghost" onclick={() => del(a.email)} aria-label={t("common.delete")}>
              <Icon name="trash" size={15} />
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if configAccount}
  <AccountConfigModal
    account={configAccount}
    {catalog}
    onclose={() => (configEmail = null)}
    onsaved={refresh}
  />
{/if}

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: var(--s-md);
  }
  .view-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .view-head h1 {
    margin: 0;
    font-size: 22px;
  }
  .count {
    font-size: 14px;
    font-weight: 400;
  }
  button.active {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--on-primary);
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-sm);
    align-items: center;
    padding: var(--s-sm) var(--s-md);
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--s-xs);
    flex: 1;
    min-width: 200px;
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
    min-width: 130px;
    height: 36px;
  }
  .batchbar {
    display: flex;
    align-items: center;
    gap: var(--s-sm);
    padding: var(--s-xs) var(--s-md);
    font-size: 13px;
  }
  .empty {
    text-align: center;
    padding: var(--s-2xl);
  }
  .rows {
    display: flex;
    flex-direction: column;
    gap: var(--s-xs);
  }
  .acc-row {
    display: flex;
    align-items: center;
    gap: var(--s-md);
    padding: var(--s-sm) var(--s-md);
    cursor: pointer;
    transition: box-shadow 0.15s;
  }
  .acc-row:hover {
    box-shadow: var(--shadow-3);
  }
  .acc-row:focus-visible {
    outline: 2px solid var(--link);
    outline-offset: 1px;
  }
  .acc-row.sel {
    box-shadow: 0 0 0 2px var(--link) inset, var(--shadow-2);
  }
  .row-check {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .dot.on {
    background: var(--success);
  }
  .dot.off {
    background: var(--mute);
  }
  .id {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    flex: 1;
  }
  .mail {
    font-size: 14px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--link);
  }
  .last {
    white-space: nowrap;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
  .actions .active {
    color: var(--link);
  }
  @media (max-width: 820px) {
    .last {
      display: none;
    }
  }
</style>
