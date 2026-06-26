<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t } from "../lib/i18n.svelte";
  import Select, { type SelectOption } from "./Select.svelte";
  import Spinner from "./Spinner.svelte";
  import type { AccountCredentials, Catalog } from "../lib/types";

  let { onadded }: { onadded?: () => void } = $props();

  const authOptions: SelectOption[] = [
    { value: "imap", label: "IMAP" },
    { value: "graph", label: "Graph API" },
    { value: "oauth2", label: "OAuth2" },
  ];

  // 分类 / 标签目录
  let catalog = $state<Catalog>({ categories: [], tags: [] });
  async function loadCatalog() {
    try {
      catalog = await api.getCatalog();
    } catch {
      /* ignore */
    }
  }
  loadCatalog();

  let categoryOptions = $derived<SelectOption[]>([
    { value: "", label: t("cfg.noCategory") },
    ...catalog.categories.map((c) => ({ value: c.key, label: `${c.name_zh} / ${c.name_en}` })),
  ]);

  // 单个添加
  let form = $state<AccountCredentials>({
    email: "",
    refresh_token: "",
    client_id: "",
    auth_method: "imap",
  });
  let singleCategory = $state("");
  let singleTags = $state<string[]>([]);
  let adding = $state(false);

  function toggleSingleTag(key: string) {
    singleTags = singleTags.includes(key)
      ? singleTags.filter((k) => k !== key)
      : [...singleTags, key];
  }

  // 批量导入
  let importText = $state("");
  let importAuth = $state("imap");
  let importCategory = $state("");
  let importTags = $state<string[]>([]);
  let importing = $state(false);

  function toggleImportTag(key: string) {
    importTags = importTags.includes(key)
      ? importTags.filter((k) => k !== key)
      : [...importTags, key];
  }

  async function addOne() {
    if (adding) return;
    if (!form.email || !form.refresh_token || !form.client_id) {
      showToast(t("add.testAdd"), "error");
      return;
    }
    adding = true;
    try {
      await api.testCredentials($state.snapshot(form));
      await api.addAccount(
        $state.snapshot(form),
        singleCategory || null,
        $state.snapshot(singleTags),
      );
      showToast(form.email, "ok");
      form = { email: "", refresh_token: "", client_id: "", auth_method: "imap" };
      singleCategory = "";
      singleTags = [];
      onadded?.();
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      adding = false;
    }
  }

  async function doImport() {
    if (importing) return;
    if (!importText.trim()) {
      showToast(t("add.bulkHint"), "error");
      return;
    }
    importing = true;
    try {
      const res = await api.importAccounts(
        importText,
        importAuth,
        importCategory || null,
        $state.snapshot(importTags),
      );
      let msg = `${res.added}/${res.total}`;
      if (res.errors.length) msg += ` (${res.errors.length})`;
      showToast(msg, res.errors.length ? "info" : "ok");
      if (res.errors.length) console.warn("import errors:", res.errors);
      importText = "";
      importCategory = "";
      importTags = [];
      onadded?.();
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      importing = false;
    }
  }
</script>

<div class="view">
  <header class="view-head"><h1>{t("add.title")}</h1></header>

  <div class="grid">
    <!-- 单个添加 -->
    <div class="card stack addcard">
      <h2>{t("add.single")}</h2>
      <Select bind:value={form.auth_method} options={authOptions} width="100%" />
      <input placeholder={t("add.email")} bind:value={form.email} />
      <input placeholder={t("add.clientId")} bind:value={form.client_id} />
      <input placeholder={t("add.refreshToken")} bind:value={form.refresh_token} />

      <div class="classify">
        <span class="sub">{t("add.classify")}</span>
        <Select bind:value={singleCategory} options={categoryOptions} width="100%" />
        {#if catalog.tags.length}
          <div class="chips">
            {#each catalog.tags as tg (tg.key)}
              <button class="chip-toggle" class:on={singleTags.includes(tg.key)} onclick={() => toggleSingleTag(tg.key)}>
                {tg.name_zh}
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <button class="primary" onclick={addOne} disabled={adding}>
        {#if adding}<Spinner size={16} />{:else}{t("add.testAdd")}{/if}
      </button>
    </div>

    <!-- 批量导入 -->
    <div class="card stack addcard">
      <h2>{t("add.bulk")}</h2>
      <Select bind:value={importAuth} options={authOptions} width="100%" />
      <p class="muted small fmt">
        {t("add.fmtImap")}<br />
        {t("add.fmtGraph")}<br />
        {t("add.fmtOauth2")}<br />
        {t("add.bulkHint")}
      </p>
      <textarea rows="6" placeholder="" bind:value={importText}></textarea>

      <div class="classify">
        <span class="sub">{t("add.classify")} · <span class="muted small">{t("add.applyAll")}</span></span>
        <Select bind:value={importCategory} options={categoryOptions} width="100%" />
        {#if catalog.tags.length}
          <div class="chips">
            {#each catalog.tags as tg (tg.key)}
              <button class="chip-toggle" class:on={importTags.includes(tg.key)} onclick={() => toggleImportTag(tg.key)}>
                {tg.name_zh}
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <button class="primary" onclick={doImport} disabled={importing}>
        {#if importing}<Spinner size={16} />{:else}{t("add.startImport")}{/if}
      </button>
    </div>
  </div>
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: var(--s-md);
  }
  .view-head h1 {
    margin: 0;
    font-size: 22px;
  }
  h2 {
    margin: 0 0 4px;
    font-size: 15px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--s-md);
    align-items: stretch;
  }
  /* 两栏等高：把提交按钮压到卡片底部对齐 */
  .addcard > button {
    margin-top: auto;
  }
  .fmt {
    margin: 0;
    line-height: 1.7;
    font-family: var(--font-mono);
  }
  .classify {
    display: flex;
    flex-direction: column;
    gap: var(--s-xs);
    padding-top: var(--s-xs);
    border-top: 1px solid var(--hairline);
  }
  .sub {
    font-size: 12px;
    color: var(--mute);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip-toggle {
    height: 30px;
    padding: 0 12px;
    border-radius: var(--r-full);
    background: var(--canvas-soft);
    border: 1px solid var(--hairline);
    font-size: 13px;
  }
  .chip-toggle.on {
    background: var(--primary);
    color: var(--on-primary);
    border-color: var(--primary);
  }
  @media (max-width: 860px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
