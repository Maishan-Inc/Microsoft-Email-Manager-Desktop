<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import { getVersion } from "@tauri-apps/api/app";
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t, i18n, setLang, type Lang } from "../lib/i18n.svelte";
  import { theme, setTheme, type Theme } from "../lib/theme.svelte";
  import { appstate } from "../lib/appstate.svelte";
  import Icon from "./Icon.svelte";
  import OtpInput from "./OtpInput.svelte";
  import Spinner from "./Spinner.svelte";
  import Select, { type SelectOption } from "./Select.svelte";
  import type { AppSettings } from "../lib/types";

  const REPO_URL = "https://github.com/Maishan-Inc/Microsoft-Email-Manager-Desktop";
  const MAISHAN_URL = "https://maishanzero.com";

  // 后台刷新设置
  let settings = $state<AppSettings>({
    bg_refresh_enabled: true,
    bg_refresh_interval_secs: 5,
    auto_lock_mins: 30,
    block_remote_images: true,
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
      appstate.blockRemoteImages = settings.block_remote_images;
      appstate.autoLockMins = settings.auto_lock_mins;
      showToast(t("common.save"), "ok");
    } catch (e) {
      showToast(errMsg(e), "error");
    }
  }

  // 远程图片屏蔽关闭确认流程
  let imgStep = $state<null | "countdown" | "auth">(null);
  let imgCountdown = $state(5);
  let imgSecret = $state("");
  let imgBusy = $state(false);
  let needs2fa = $state(false);
  let imgTimer: ReturnType<typeof setInterval> | null = null;
  api.authModeInfo().then(v => (needs2fa = v)).catch(() => {});

  function startDisableFlow() {
    imgStep = "countdown";
    imgCountdown = 5;
    imgTimer = setInterval(() => {
      imgCountdown -= 1;
      if (imgCountdown <= 0 && imgTimer) { clearInterval(imgTimer); imgTimer = null; }
    }, 1000);
  }
  function cancelImgModal() {
    if (imgTimer) { clearInterval(imgTimer); imgTimer = null; }
    imgStep = null;
    imgSecret = "";
  }
  async function verifyAndDisable() {
    if (!imgSecret.trim() || imgBusy) return;
    imgBusy = true;
    try {
      await api.verifyAuth(imgSecret.trim());
      settings.block_remote_images = false;
      await api.setSettings($state.snapshot(settings));
      appstate.blockRemoteImages = false;
      imgStep = null;
      imgSecret = "";
      showToast(t("common.save"), "ok");
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      imgBusy = false;
    }
  }

  // 导出
  let exportFormat = $state<"json" | "csv" | "txt">("json");
  let exportCreds = $state(false);
  let exportEncrypt = $state(true);
  let exporting = $state(false);

  const exportFormatOptions: SelectOption[] = [
    { value: "json", label: "JSON" },
    { value: "csv", label: "CSV" },
    { value: "txt", label: "TXT" },
  ];

  // 导出前身份验证（开启 2FA 用动态码，否则用主密码）
  let expVerify = $state(false);
  let expSecret = $state("");
  let expBusy = $state(false);

  function startExport() {
    if (exporting) return;
    expSecret = "";
    expVerify = true;
  }
  function cancelExport() {
    expVerify = false;
    expSecret = "";
  }
  async function confirmExport() {
    if (!expSecret.trim() || expBusy) return;
    expBusy = true;
    try {
      await api.verifyAuth(expSecret.trim());
      expVerify = false;
      expSecret = "";
      await runExport();
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      expBusy = false;
    }
  }
  async function runExport() {
    const ext = exportFormat;
    const encrypt = exportCreds && exportEncrypt;
    const suggested = `accounts-export.${encrypt ? ext + ".memenc" : ext}`;
    try {
      const path = await save({
        defaultPath: suggested,
        filters: [{ name: ext.toUpperCase(), extensions: [ext, "memenc"] }],
      });
      if (!path) return;
      exporting = true;
      await api.exportAccounts(path, exportFormat, exportCreds, encrypt);
      showToast(path, "ok");
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      exporting = false;
    }
  }

  // 关于 / 外链
  async function openExt(url: string) {
    try {
      await api.openUrl(url);
    } catch {
      try {
        await navigator.clipboard.writeText(url);
        showToast(t("common.copied"), "ok");
      } catch {
        /* ignore */
      }
    }
  }
  async function copyRepo() {
    try {
      await navigator.clipboard.writeText(REPO_URL);
      showToast(t("common.copied"), "ok");
    } catch {
      /* ignore */
    }
  }

  // 重新生成恢复助记词（验证身份 → 展示新助记词）
  let regenStep = $state<null | "auth" | "show">(null);
  let regenSecret = $state("");
  let regenBusy = $state(false);
  let regenWords = $state<string[]>([]);
  function startRegen() {
    regenSecret = "";
    regenWords = [];
    regenStep = "auth";
  }
  function cancelRegen() {
    regenStep = null;
    regenSecret = "";
  }
  async function confirmRegen() {
    if (!regenSecret.trim() || regenBusy) return;
    regenBusy = true;
    try {
      const r = await api.regenerateMnemonic(regenSecret.trim());
      regenWords = r.words;
      regenSecret = "";
      regenStep = "show";
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      regenBusy = false;
    }
  }
  function downloadRegen() {
    const blob = new Blob([regenWords.join(" ")], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "recovery-phrase.txt";
    a.click();
    URL.revokeObjectURL(url);
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
          min="5"
          bind:value={settings.bg_refresh_interval_secs}
          onchange={saveSettings}
        />
      </label>
    </div>
  </div>

  <!-- 自动锁定 -->
  <div class="card sec">
    <div class="sec-label">{t("set.autoLock")}</div>
    <label class="row inline">
      {t("set.autoLockMins")}
      <input class="num" type="number" min="0" bind:value={settings.auto_lock_mins} onchange={saveSettings} />
    </label>
  </div>

  <!-- 远程图片屏蔽 -->
  <div class="card sec">
    <div class="sec-label">{t("set.blockImages")}</div>
    <p class="muted small" style="margin:0">{t("set.blockImagesDesc")}</p>
    <label class="switch">
      <input type="checkbox" checked={settings.block_remote_images}
        onchange={(e) => {
          if ((e.target as HTMLInputElement).checked) {
            settings.block_remote_images = true;
            saveSettings();
          } else {
            (e.target as HTMLInputElement).checked = true;
            startDisableFlow();
          }
        }}
      />
      <span>{settings.block_remote_images ? t("common.enabled") : t("common.disabled")}</span>
    </label>
  </div>

  <!-- 导出 -->
  <div class="card sec">
    <div class="sec-label">{t("set.export")}</div>
    <div class="row wrap">
      <Select bind:value={exportFormat} options={exportFormatOptions} width="120px" />
      <label class="row inline">
        <input type="checkbox" bind:checked={exportCreds} class="cb" /> refresh_token
      </label>
      {#if exportCreds}
        <label class="row inline">
          <input type="checkbox" bind:checked={exportEncrypt} class="cb" /> 加密
        </label>
      {/if}
      <button class="primary" onclick={startExport} disabled={exporting}>
        {#if exporting}<Spinner size={16} />{:else}{t("set.export")}{/if}
      </button>
    </div>
    {#if exportCreds && !exportEncrypt}
      <p class="warn-text small"><Icon name="alert-triangle" size={14} /> 将导出明文 refresh_token，请妥善保管。</p>
    {/if}
  </div>

  <!-- 恢复助记词 -->
  <div class="card sec">
    <div class="sec-label">{t("set.recovery")}</div>
    <p class="muted small" style="margin:0">{t("set.recoveryDesc")}</p>
    <button class="fit" onclick={startRegen}><Icon name="refresh" size={15} /> {t("set.regenerate")}</button>
  </div>

  <!-- 关于 -->
  <div class="card sec">
    <div class="sec-label">{t("set.about")}</div>
    <div class="about">
      <div>
        <span class="muted">{t("set.publisher")}</span>
        <button class="repo-link" onclick={() => openExt(MAISHAN_URL)} title={t("set.openInBrowser")}>Maishan Inc.</button>
      </div>
      <div><span class="muted">{t("set.version")}</span><span class="mono">v{version}</span></div>
      <div><span class="muted">{t("set.license")}</span><span>CC BY-NC 4.0</span></div>
      <div class="repo-row">
        <span class="muted">{t("set.repo")}</span>
        <button class="repo-link" onclick={() => openExt(REPO_URL)} title={t("set.openInBrowser")}>
          <Icon name="github" size={15} /> Maishan-Inc/Microsoft-Email-Manager-Desktop
        </button>
        <button class="ghost sm" onclick={copyRepo} aria-label={t("common.copy")}>
          <Icon name="copy" size={14} />
        </button>
      </div>
    </div>
  </div>
</div>

{#if expVerify}  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && cancelExport()}>
    <div class="modal" role="dialog" aria-modal="true">
      <header class="m-head">
        <h3>{t("set.exportVerify")}</h3>
        <button class="ghost icon-btn" onclick={cancelExport} aria-label="close"><Icon name="x" size={18} /></button>
      </header>
      <div class="m-body">
        <p class="muted small">{needs2fa ? t("set.exportVerifyDesc2fa") : t("set.exportVerifyDescPw")}</p>
        {#if needs2fa}
          <OtpInput bind:value={expSecret} autofocus oncomplete={confirmExport} />
        {:else}
          <input
            type="password"
            placeholder={t("set.unblockPromptPw")}
            bind:value={expSecret}
            onkeydown={(e) => e.key === "Enter" && confirmExport()}
          />
        {/if}
        <div class="btn-row">
          <button onclick={cancelExport}>{t("common.cancel")}</button>
          <button class="primary" disabled={expBusy || !expSecret.trim()} onclick={confirmExport}>
            {#if expBusy}<Spinner size={16} />{:else}{t("set.export")}{/if}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

{#if regenStep}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && cancelRegen()}>
    <div class="modal" role="dialog" aria-modal="true">
      <header class="m-head">
        <h3>{regenStep === "show" ? t("set.regenDone") : t("set.regenVerify")}</h3>
        <button class="ghost icon-btn" onclick={cancelRegen} aria-label="close"><Icon name="x" size={18} /></button>
      </header>
      <div class="m-body">
        {#if regenStep === "auth"}
          <p class="muted small">{needs2fa ? t("set.unblockPrompt2fa") : t("set.unblockPromptPw")}</p>
          {#if needs2fa}
            <OtpInput bind:value={regenSecret} autofocus oncomplete={confirmRegen} />
          {:else}
            <input
              type="password"
              placeholder={t("set.unblockPromptPw")}
              bind:value={regenSecret}
              onkeydown={(e) => e.key === "Enter" && confirmRegen()}
            />
          {/if}
          <div class="btn-row">
            <button onclick={cancelRegen}>{t("common.cancel")}</button>
            <button class="primary" disabled={regenBusy || !regenSecret.trim()} onclick={confirmRegen}>
              {#if regenBusy}<Spinner size={16} />{:else}{t("common.confirm")}{/if}
            </button>
          </div>
        {:else}
          <p class="warn-text small">{t("ob.mnem.showDesc")}</p>
          <div class="words">
            {#each regenWords as w, i (i)}
              <div class="word"><span class="wi">{i + 1}</span>{w}</div>
            {/each}
          </div>
          <div class="btn-row">
            <button onclick={downloadRegen}><Icon name="download" size={16} /> {t("ob.mnem.download")}</button>
            <button class="primary" onclick={cancelRegen}>{t("common.confirm")}</button>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if imgStep}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && cancelImgModal()}>
    <div class="modal" role="dialog" aria-modal="true">
      <header class="m-head">
        <h3>{imgStep === "auth" ? t("set.unblockVerify") : t("set.blockImages")}</h3>
        <button class="ghost icon-btn" onclick={cancelImgModal} aria-label="close"><Icon name="x" size={18} /></button>
      </header>
      <div class="m-body">
        {#if imgStep === "countdown"}
          <p class="warn-text"><Icon name="alert-triangle" size={15} /> {t("set.unblockWarn")}</p>
          <button class="primary" disabled={imgCountdown > 0} onclick={() => (imgStep = "auth")}>
            {imgCountdown > 0 ? t("set.unblockCountdown", { s: imgCountdown }) : t("set.unblockConfirm")}
          </button>
          <button onclick={cancelImgModal}>{t("common.cancel")}</button>
        {:else}
          <p class="muted small">{needs2fa ? t("set.unblockPrompt2fa") : t("set.unblockPromptPw")}</p>
          {#if needs2fa}
            <OtpInput bind:value={imgSecret} autofocus oncomplete={verifyAndDisable} />
          {:else}
            <input
              type="password"
              placeholder={t("set.unblockPromptPw")}
              bind:value={imgSecret}
              onkeydown={(e) => e.key === "Enter" && verifyAndDisable()}
            />
          {/if}
          <div class="btn-row">
            <button onclick={cancelImgModal}>{t("common.cancel")}</button>
            <button class="primary" disabled={imgBusy || !imgSecret.trim()} onclick={verifyAndDisable}>
              {#if imgBusy}<Spinner size={16} />{:else}{t("common.confirm")}{/if}
            </button>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

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
    align-items: center;
  }
  .about .muted {
    width: 80px;
    display: inline-block;
    flex-shrink: 0;
  }
  .repo-row {
    flex-wrap: wrap;
  }
  .repo-link {
    height: 30px;
    padding: 0 10px;
    color: var(--link);
    border-color: var(--hairline);
    font-size: 13px;
    max-width: 100%;
    overflow: hidden;
  }
  .repo-link:hover {
    border-color: var(--link);
    background: var(--link-bg-soft);
  }
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
    width: 440px;
    max-width: 100%;
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
    padding: var(--s-md) var(--s-lg);
    border-bottom: 1px solid var(--hairline);
  }
  .m-head h3 { margin: 0; font-size: 16px; }
  .icon-btn { width: 34px; padding: 0; }
  .m-body {
    padding: var(--s-lg);
    display: flex;
    flex-direction: column;
    gap: var(--s-sm);
  }
  .btn-row { display: flex; gap: var(--s-xs); }
  .btn-row button { flex: 1; }
  .fit {
    width: fit-content;
  }
  .words {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--s-xs);
  }
  .word {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--canvas-soft);
    border: 1px solid var(--hairline);
    border-radius: var(--r-sm);
    padding: 8px 10px;
    font-size: 14px;
    font-family: var(--font-mono);
  }
  .wi {
    color: var(--mute);
    font-size: 11px;
    width: 16px;
  }
</style>
