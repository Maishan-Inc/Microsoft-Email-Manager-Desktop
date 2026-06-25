<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import { getVersion } from "@tauri-apps/api/app";
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t, i18n, setLang, type Lang } from "../lib/i18n.svelte";
  import { theme, setTheme, type Theme } from "../lib/theme.svelte";
  import Icon from "./Icon.svelte";
  import type { AppSettings } from "../lib/types";

  // 后台刷新设置
  let settings = $state<AppSettings>({
    bg_refresh_enabled: true,
    bg_refresh_interval_secs: 300,
  });
  async function loadSettings() {
    try {
      settings = await api.getSettings();
    } catch (e) {
      console.warn("get_settings:", errMsg(e));
    }
  }
  loadSettings();

  async function saveSettings() {
    try {
      await api.setSettings($state.snapshot(settings));
      showToast(t("common.save"), "ok");
    } catch (e) {
      showToast(errMsg(e), "error");
    }
  }

  // 导出
  let exportFormat = $state<"json" | "csv">("json");
  let exportCreds = $state(false);
  let exportEncrypt = $state(true);
  let exporting = $state(false);
  async function doExport() {
    if (exporting) return;
    const ext = exportFormat;
    const suggested = `accounts-export.${exportCreds && exportEncrypt ? ext + ".memenc" : ext}`;
    try {
      const path = await save({
        defaultPath: suggested,
        filters: [{ name: ext.toUpperCase(), extensions: [ext, "memenc"] }],
      });
      if (!path) return;
      exporting = true;
      await api.exportAccounts(path, exportFormat, exportCreds, exportCreds && exportEncrypt);
      showToast(path, "ok");
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      exporting = false;
    }
  }

  // 关于
  let version = $state("0.1.0");
  getVersion().then((v) => (version = v)).catch(() => {});

  const langs: { v: Lang; label: string }[] = [
    { v: "zh", label: "中文" },
    { v: "en", label: "English" },
  ];
  const themes: { v: Theme; key: "set.themeLight" | "set.themeDark" | "set.themeSystem" }[] = [
    { v: "light", key: "set.themeLight" },
    { v: "dark", key: "set.themeDark" },
    { v: "system", key: "set.themeSystem" },
  ];
</script>

<div class="view">
  <header class="view-head"><h1>{t("set.title")}</h1></header>

  <!-- 语言 -->
  <div class="card sec">
    <div class="sec-label">{t("set.language")}</div>
    <div class="seg">
      {#each langs as l (l.v)}
        <button class:active={i18n.lang === l.v} onclick={() => setLang(l.v)}>{l.label}</button>
      {/each}
    </div>
  </div>

  <!-- 主题 -->
  <div class="card sec">
    <div class="sec-label">{t("set.theme")}</div>
    <div class="seg">
      {#each themes as th (th.v)}
        <button class:active={theme.mode === th.v} onclick={() => setTheme(th.v)}>{t(th.key)}</button>
      {/each}
    </div>
  </div>

  <!-- 后台刷新 -->
  <div class="card sec">
    <div class="sec-label">{t("set.bgRefresh")}</div>
    <div class="row wrap">
      <label class="switch">
        <input type="checkbox" bind:checked={settings.bg_refresh_enabled} onchange={saveSettings} />
        <span>{settings.bg_refresh_enabled ? t("common.enabled") : t("common.disabled")}</span>
      </label>
      <label class="row inline">
        {t("set.bgInterval")}
        <input
          class="num"
          type="number"
          min="30"
          bind:value={settings.bg_refresh_interval_secs}
          onchange={saveSettings}
        />
      </label>
    </div>
  </div>

  <!-- 导出 -->
  <div class="card sec">
    <div class="sec-label">{t("set.export")}</div>
    <div class="row wrap">
      <select bind:value={exportFormat} class="w-auto">
        <option value="json">JSON</option>
        <option value="csv">CSV</option>
      </select>
      <label class="row inline">
        <input type="checkbox" bind:checked={exportCreds} class="cb" /> refresh_token
      </label>
      {#if exportCreds}
        <label class="row inline">
          <input type="checkbox" bind:checked={exportEncrypt} class="cb" /> 加密
        </label>
      {/if}
      <button class="primary" onclick={doExport} disabled={exporting}>
        {exporting ? t("common.loading") : t("set.export")}
      </button>
    </div>
    {#if exportCreds && !exportEncrypt}
      <p class="warn-text small"><Icon name="alert-triangle" size={14} /> 将导出明文 refresh_token，请妥善保管。</p>
    {/if}
  </div>

  <!-- 关于 -->
  <div class="card sec">
    <div class="sec-label">{t("set.about")}</div>
    <div class="about">
      <div><span class="muted">{t("set.publisher")}</span><b>Maishan Inc.</b></div>
      <div><span class="muted">{t("set.version")}</span><span class="mono">{version}</span></div>
      <div><span class="muted">{t("set.license")}</span><span>CC BY-NC 4.0</span></div>
    </div>
  </div>
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: var(--s-md);
    max-width: 720px;
  }
  .view-head h1 {
    margin: 0;
    font-size: 22px;
  }
  .sec {
    display: flex;
    flex-direction: column;
    gap: var(--s-sm);
  }
  .sec-label {
    font-weight: 600;
    font-size: 14px;
  }
  .seg {
    display: inline-flex;
    gap: 4px;
    background: var(--canvas-soft);
    padding: 4px;
    border-radius: var(--r-md);
    width: fit-content;
  }
  .seg button {
    height: 30px;
    border: 1px solid transparent;
    background: transparent;
  }
  .seg button.active {
    background: var(--canvas);
    border-color: var(--hairline);
    color: var(--ink);
    box-shadow: var(--shadow-2);
  }
  .wrap {
    flex-wrap: wrap;
  }
  .inline {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .switch {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  .cb,
  .switch input {
    width: auto;
    height: auto;
  }
  .num {
    width: 96px;
  }
  .w-auto {
    width: auto;
    min-width: 110px;
  }
  .warn-text {
    color: var(--warning-deep);
    margin: 0;
  }
  .about {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
  }
  .about div {
    display: flex;
    gap: 10px;
  }
  .about .muted {
    width: 80px;
    display: inline-block;
  }
</style>
