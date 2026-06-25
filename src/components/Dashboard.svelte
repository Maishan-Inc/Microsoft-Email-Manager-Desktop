<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t } from "../lib/i18n.svelte";
  import { listen } from "@tauri-apps/api/event";
  import type { DashboardStats } from "../lib/types";

  let { onnavigate }: { onnavigate: (v: "accounts" | "add") => void } = $props();

  let stats = $state<DashboardStats | null>(null);
  let loading = $state(false);
  let syncing = $state(false);

  async function load() {
    loading = true;
    try {
      stats = await api.dashboardStats();
    } catch (e) {
      // 后端命令未就绪/出错时优雅降级
      stats = {
        account_count: 0,
        health_avg: 0,
        healthy_count: 0,
        unchecked_count: 0,
        today_mail: 0,
        recent: [],
      };
      console.warn("dashboard_stats:", errMsg(e));
    } finally {
      loading = false;
    }
  }
  load();

  // 后台检测到新邮件时刷新统计
  $effect(() => {
    const un = listen("mail:new", () => load());
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  });

  async function refreshStats() {
    if (syncing) return;
    syncing = true;
    try {
      await api.syncMailNow();
      await load();
      showToast(t("dash.refreshStats"), "ok");
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      syncing = false;
    }
  }

  function healthClass(avg: number): string {
    if (avg >= 90) return "ok";
    if (avg >= 60) return "warn";
    return "bad";
  }
  function fmtTime(s: string): string {
    if (!s) return "";
    const d = new Date(s);
    return isNaN(d.getTime()) ? s : d.toLocaleString();
  }
</script>

<div class="view">
  <header class="view-head">
    <h1>{t("dash.title")}</h1>
    <button onclick={refreshStats} disabled={syncing}>
      {syncing ? t("common.loading") : t("dash.refreshStats")}
    </button>
  </header>

  <div class="cards">
    <!-- 邮箱数量 -->
    <div class="card stat">
      <div class="stat-label">{t("dash.accounts")}</div>
      <div class="stat-num">{stats?.account_count ?? "—"}</div>
      <div class="stat-sub muted small">
        {t("dash.unchecked")}: {stats?.unchecked_count ?? 0}
      </div>
    </div>

    <!-- 健康度 -->
    <div class="card stat">
      <div class="stat-label">{t("dash.health")}</div>
      <div class="health-row">
        <div
          class="ring"
          style="--p:{stats?.health_avg ?? 0}"
          class:ok={(stats?.health_avg ?? 0) >= 90}
          class:warn={(stats?.health_avg ?? 0) >= 60 && (stats?.health_avg ?? 0) < 90}
          class:bad={(stats?.health_avg ?? 0) < 60}
        >
          <span>{stats?.health_avg ?? 0}</span>
        </div>
        <div class="stack" style="gap:4px">
          <span class="pill {healthClass(stats?.health_avg ?? 0)}">
            {t("dash.healthy")} {stats?.healthy_count ?? 0}/{stats?.account_count ?? 0}
          </span>
        </div>
      </div>
    </div>

    <!-- 当日接收 -->
    <div class="card stat">
      <div class="stat-label">{t("dash.todayMail")}</div>
      <div class="stat-num">{stats?.today_mail ?? "—"}</div>
      <div class="stat-sub muted small">{new Date().toLocaleDateString()}</div>
    </div>
  </div>

  <!-- 最近活动 -->
  <div class="card recent">
    <div class="recent-head">
      <h2>{t("dash.recent")}</h2>
    </div>
    {#if loading}
      <p class="muted">{t("common.loading")}</p>
    {:else if !stats || stats.recent.length === 0}
      <p class="muted small">{t("dash.noActivity")}</p>
    {:else}
      <ul class="recent-list">
        {#each stats.recent as m (m.email + m.message_id)}
          <li>
            <span class="r-from">{m.from_email}</span>
            <span class="r-subj">{m.subject}</span>
            <span class="r-time muted small">{fmtTime(m.received_at)}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <!-- 快捷入口 -->
  <div class="quick">
    <button class="primary" onclick={() => onnavigate("add")}>＋ {t("dash.goAdd")}</button>
    <button onclick={() => onnavigate("accounts")}>{t("dash.goAccounts")}</button>
  </div>
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
  h1 {
    margin: 0;
    font-size: 22px;
  }
  h2 {
    margin: 0;
    font-size: 15px;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--s-md);
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: var(--s-xs);
    min-height: 116px;
  }
  .stat-label {
    font-size: 13px;
    color: var(--body);
    font-weight: 500;
  }
  .stat-num {
    font-size: 40px;
    font-weight: 600;
    letter-spacing: -0.03em;
    line-height: 1;
    margin-top: auto;
  }
  .stat-sub {
    margin-top: 2px;
  }
  .health-row {
    display: flex;
    align-items: center;
    gap: var(--s-md);
    margin-top: auto;
  }
  /* 健康环：conic-gradient 进度 */
  .ring {
    --p: 0;
    --col: var(--mute);
    width: 64px;
    height: 64px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: conic-gradient(
      var(--col) calc(var(--p) * 1%),
      var(--canvas-soft-2) 0
    );
    flex-shrink: 0;
  }
  .ring.ok {
    --col: var(--success);
  }
  .ring.warn {
    --col: var(--warning);
  }
  .ring.bad {
    --col: var(--error);
  }
  .ring span {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: var(--canvas);
    display: grid;
    place-items: center;
    font-weight: 600;
    font-size: 16px;
  }
  .recent-head {
    margin-bottom: var(--s-sm);
  }
  .recent-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .recent-list li {
    display: grid;
    grid-template-columns: 180px 1fr auto;
    gap: var(--s-sm);
    align-items: center;
    padding: var(--s-xs) 0;
    border-bottom: 1px solid var(--hairline);
  }
  .recent-list li:last-child {
    border-bottom: none;
  }
  .r-from {
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .r-subj {
    font-size: 13px;
    color: var(--body);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .quick {
    display: flex;
    gap: var(--s-sm);
  }
</style>
