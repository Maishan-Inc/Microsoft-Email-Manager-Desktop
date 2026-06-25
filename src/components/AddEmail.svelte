<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t } from "../lib/i18n.svelte";
  import type { AccountCredentials } from "../lib/types";

  let { onadded }: { onadded?: () => void } = $props();

  // 单个添加
  let form = $state<AccountCredentials>({
    email: "",
    refresh_token: "",
    client_id: "",
    auth_method: "imap",
  });
  let adding = $state(false);

  // 批量导入
  let importText = $state("");
  let importAuth = $state("imap");
  let importing = $state(false);

  async function addOne() {
    if (adding) return;
    if (!form.email || !form.refresh_token || !form.client_id) {
      showToast(t("add.testAdd"), "error");
      return;
    }
    adding = true;
    try {
      await api.testCredentials($state.snapshot(form));
      await api.addAccount($state.snapshot(form), null, []);
      showToast(form.email, "ok");
      form = { email: "", refresh_token: "", client_id: "", auth_method: "imap" };
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
      const res = await api.importAccounts(importText, importAuth);
      let msg = `${res.added}/${res.total}`;
      if (res.errors.length) msg += ` (${res.errors.length})`;
      showToast(msg, res.errors.length ? "info" : "ok");
      if (res.errors.length) console.warn("import errors:", res.errors);
      importText = "";
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
    <div class="card stack">
      <h2>{t("add.single")}</h2>
      <select bind:value={form.auth_method}>
        <option value="imap">IMAP</option>
        <option value="graph">Graph API</option>
        <option value="oauth2">OAuth2</option>
      </select>
      <input placeholder={t("add.email")} bind:value={form.email} />
      <input placeholder={t("add.clientId")} bind:value={form.client_id} />
      <input placeholder={t("add.refreshToken")} bind:value={form.refresh_token} />
      <button class="primary" onclick={addOne} disabled={adding}>
        {adding ? t("add.verifying") : t("add.testAdd")}
      </button>
    </div>

    <!-- 批量导入 -->
    <div class="card stack">
      <h2>{t("add.bulk")}</h2>
      <select bind:value={importAuth}>
        <option value="imap">IMAP</option>
        <option value="graph">Graph API</option>
        <option value="oauth2">OAuth2</option>
      </select>
      <p class="muted small fmt">
        IMAP：邮箱----刷新令牌----客户端ID<br />
        Graph：邮箱----密码----client_id----令牌<br />
        OAuth2：邮箱----密码----client_id----refresh_token
      </p>
      <textarea rows="8" placeholder={t("add.bulkHint")} bind:value={importText}
      ></textarea>
      <button class="primary" onclick={doImport} disabled={importing}>
        {importing ? t("add.importing") : t("add.startImport")}
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
    align-items: start;
  }
  .fmt {
    margin: 0;
    line-height: 1.7;
    font-family: var(--font-mono);
  }
  @media (max-width: 860px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
