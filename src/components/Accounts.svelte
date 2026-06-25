<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t } from "../lib/i18n.svelte";
  import type { AccountInfo } from "../lib/types";

  let { onopenmail }: { onopenmail: (email: string) => void } = $props();

  let accounts = $state<AccountInfo[]>([]);
  let loading = $state(false);
  let testingEmail = $state<string | null>(null);
  let checkingEmail = $state<string | null>(null);
  let togglingEmail = $state<string | null>(null);

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
  refresh();

  function healthClass(a: AccountInfo): string {
    if (!a.health_checked_at) return "";
    if (a.health_score >= 100) return "ok";
    if (a.health_score >= 60) return "warn";
    return "bad";
  }

  async function test(email: string) {
    testingEmail = email;
    try {
      await api.testAccount(email);
      showToast(email + " ✓", "ok");
    } catch (e) {
      showToast(email + " ✗ " + errMsg(e), "error");
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
      showToast(`${a.email} ${t("acc.notify")} ${next ? t("common.enabled") : t("common.disabled")}`, "ok");
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
      showToast(t("common.delete") + " ✓", "ok");
      await refresh();
    } catch (e) {
      showToast(errMsg(e), "error");
    }
  }

  function fmt(s: string | null): string {
    if (!s) return t("acc.never");
    const d = new Date(s);
    return isNaN(d.getTime()) ? s : d.toLocaleDateString();
  }
</script>

<div class="view">
  <header class="view-head">
    <h1>{t("acc.title")} <span class="muted count">{t("acc.count", { n: accounts.length })}</span></h1>
    <button onclick={refresh} disabled={loading}>{t("common.refresh")}</button>
  </header>

  {#if loading}
    <p class="muted">{t("common.loading")}</p>
  {:else if accounts.length === 0}
    <div class="card empty muted">{t("acc.empty")}</div>
  {:else}
    <div class="rows">
      {#each accounts as a (a.email)}
        <div
          class="acc-row card"
          role="button"
          tabindex="0"
          onclick={(e) => {
            if (!(e.target as HTMLElement).closest(".actions")) onopenmail(a.email);
          }}
          onkeydown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), onopenmail(a.email))}
        >
          <span class="dot {a.status === 'active' ? 'on' : 'off'}"></span>
          <div class="id">
            <span class="mail">{a.email}</span>
            <span class="meta">
              <span class="badge">{a.auth_method}</span>
              {#if a.health_checked_at}
                <span class="pill {healthClass(a)}">{a.health_score}</span>
              {:else}
                <span class="muted small">{t("dash.unchecked")}</span>
              {/if}
            </span>
          </div>

          <span class="last muted small">{t("acc.lastCheck")}: {fmt(a.health_checked_at)}</span>

          <div class="actions">
            <button
              class="ghost sm bell"
              class:active={a.notify_enabled}
              title={t("acc.notify")}
              disabled={togglingEmail === a.email}
              onclick={() => toggleNotify(a)}
              aria-label={t("acc.notify")}
            >
              {a.notify_enabled ? "🔔" : "🔕"}
            </button>
            <button class="sm" onclick={() => test(a.email)} disabled={testingEmail === a.email}>
              {testingEmail === a.email ? t("acc.testing") : t("acc.test")}
            </button>
            <button class="sm" onclick={() => checkHealth(a.email)} disabled={checkingEmail === a.email}>
              {checkingEmail === a.email ? t("acc.checking") : t("acc.health")}
            </button>
            <button class="sm" onclick={() => onopenmail(a.email)}>{t("acc.openMail")}</button>
            <button class="sm danger ghost" onclick={() => del(a.email)} aria-label={t("common.delete")}>✕</button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

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
    transition: box-shadow 0.15s, transform 0.05s;
  }
  .acc-row:hover {
    box-shadow: var(--shadow-3);
  }
  .acc-row:focus-visible {
    outline: 2px solid var(--link);
    outline-offset: 1px;
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
  .bell.active {
    color: var(--link);
  }
  @media (max-width: 760px) {
    .last {
      display: none;
    }
  }
</style>
