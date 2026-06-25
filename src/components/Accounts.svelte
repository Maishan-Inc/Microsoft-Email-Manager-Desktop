<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import type { AccountInfo, AccountCredentials } from "../lib/types";

  let accounts = $state<AccountInfo[]>([]);
  let loading = $state(false);

  // 添加单个
  let showAdd = $state(false);
  let form = $state<AccountCredentials>({
    email: "",
    refresh_token: "",
    client_id: "",
    auth_method: "imap",
  });
  let adding = $state(false);

  // 批量导入
  let showImport = $state(false);
  let importText = $state("");
  let importAuth = $state("imap");
  let importing = $state(false);

  let testingEmail = $state<string | null>(null);
  let checkingEmail = $state<string | null>(null);

  // 导出
  let showExport = $state(false);
  let exportFormat = $state<"json" | "csv">("json");
  let exportCreds = $state(false);
  let exportEncrypt = $state(true);
  let exporting = $state(false);

  export async function refresh() {
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

  async function addAccount() {
    if (adding) return;
    if (!form.email || !form.refresh_token || !form.client_id) {
      showToast("请填写完整凭据", "error");
      return;
    }
    adding = true;
    try {
      // 添加前做一次连接测试，避免存入无效凭据
      await api.testCredentials($state.snapshot(form));
      await api.addAccount($state.snapshot(form), null, []);
      showToast("已添加并验证：" + form.email, "ok");
      form = { email: "", refresh_token: "", client_id: "", auth_method: "imap" };
      showAdd = false;
      await refresh();
    } catch (e) {
      showToast("添加失败：" + errMsg(e), "error");
    } finally {
      adding = false;
    }
  }

  async function doImport() {
    if (importing) return;
    if (!importText.trim()) {
      showToast("请粘贴导入内容", "error");
      return;
    }
    importing = true;
    try {
      const res = await api.importAccounts(importText, importAuth);
      let msg = `成功导入 ${res.added}/${res.total}`;
      if (res.errors.length) msg += `，${res.errors.length} 条失败`;
      showToast(msg, res.errors.length ? "info" : "ok");
      if (res.errors.length) console.warn("导入错误:", res.errors);
      importText = "";
      showImport = false;
      await refresh();
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      importing = false;
    }
  }

  async function del(email: string) {
    if (!confirm(`确定删除账号 ${email}？`)) return;
    try {
      await api.deleteAccount(email);
      showToast("已删除", "ok");
      await refresh();
    } catch (e) {
      showToast(errMsg(e), "error");
    }
  }

  async function test(email: string) {
    testingEmail = email;
    try {
      await api.testAccount(email);
      showToast(email + " 连接正常", "ok");
    } catch (e) {
      showToast(email + " 连接失败：" + errMsg(e), "error");
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
      }
      showToast(`${email}：${res.summary}`, res.score >= 100 ? "ok" : "info");
    } catch (e) {
      showToast("检查失败：" + errMsg(e), "error");
    } finally {
      checkingEmail = null;
    }
  }

  async function doExport() {
    if (exporting) return;
    if (accounts.length === 0) {
      showToast("没有可导出的账号", "error");
      return;
    }
    const ext = exportFormat;
    const suggested = `accounts-export.${exportCreds && exportEncrypt ? ext + ".memenc" : ext}`;
    try {
      const path = await save({
        defaultPath: suggested,
        filters: [{ name: exportFormat.toUpperCase(), extensions: [ext, "memenc"] }],
      });
      if (!path) return; // 用户取消
      exporting = true;
      await api.exportAccounts(
        path,
        exportFormat,
        exportCreds,
        exportCreds && exportEncrypt,
      );
      showToast("已导出到：" + path, "ok");
      showExport = false;
    } catch (e) {
      showToast("导出失败：" + errMsg(e), "error");
    } finally {
      exporting = false;
    }
  }
</script>


<div class="toolbar">
  <h2>账号管理 <span class="muted">（{accounts.length}）</span></h2>
  <div class="row">
    <button onclick={() => (showAdd = !showAdd)}>+ 添加账号</button>
    <button onclick={() => (showImport = !showImport)}>批量导入</button>
    <button onclick={() => (showExport = !showExport)}>导出</button>
    <button onclick={refresh} disabled={loading}>刷新</button>
  </div>
</div>

{#if showExport}
  <div class="card form">
    <div class="row">
      <label class="row inline">
        格式：
        <select bind:value={exportFormat} style="max-width:120px">
          <option value="json">JSON</option>
          <option value="csv">CSV</option>
        </select>
      </label>
      <label class="row inline">
        <input type="checkbox" bind:checked={exportCreds} style="width:auto" />
        包含凭据（refresh_token）
      </label>
      {#if exportCreds}
        <label class="row inline">
          <input type="checkbox" bind:checked={exportEncrypt} style="width:auto" />
          加密导出（主密码可解）
        </label>
      {/if}
    </div>
    {#if exportCreds && !exportEncrypt}
      <p class="warn-text small">
        ⚠️ 你将导出明文凭据，文件包含 refresh_token，请务必妥善保管、勿外传。
      </p>
    {/if}
    <div class="row">
      <button class="primary" onclick={doExport} disabled={exporting}>
        {exporting ? "导出中…" : "选择位置并导出"}
      </button>
      <button onclick={() => (showExport = false)}>取消</button>
    </div>
  </div>
{/if}

{#if showAdd}
  <div class="card form">
    <div class="row">
      <select bind:value={form.auth_method} style="max-width:140px">
        <option value="imap">IMAP</option>
        <option value="graph">Graph API</option>
      </select>
      <input placeholder="邮箱" bind:value={form.email} />
    </div>
    <input placeholder="client_id（客户端ID）" bind:value={form.client_id} />
    <input placeholder="refresh_token（刷新令牌）" bind:value={form.refresh_token} />
    <div class="row">
      <button class="primary" onclick={addAccount} disabled={adding}>
        {adding ? "验证中…" : "测试并添加"}
      </button>
      <button onclick={() => (showAdd = false)}>取消</button>
    </div>
  </div>
{/if}

{#if showImport}
  <div class="card form">
    <div class="row">
      <select bind:value={importAuth} style="max-width:140px">
        <option value="imap">IMAP</option>
        <option value="graph">Graph API</option>
      </select>
      <span class="muted small">
        IMAP：邮箱----刷新令牌----客户端ID ｜ Graph：邮箱----密码----client_id----令牌
      </span>
    </div>
    <textarea
      rows="8"
      placeholder="每行一个账号，使用 ---- 分隔"
      bind:value={importText}
    ></textarea>
    <div class="row">
      <button class="primary" onclick={doImport} disabled={importing}>
        {importing ? "导入中…" : "开始导入"}
      </button>
      <button onclick={() => (showImport = false)}>取消</button>
    </div>
  </div>
{/if}

<div class="card">
  {#if loading}
    <p class="muted">加载中…</p>
  {:else if accounts.length === 0}
    <p class="muted">还没有账号，点击「添加账号」或「批量导入」开始。</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>邮箱</th>
          <th>接入</th>
          <th>状态</th>
          <th>健康</th>
          <th>操作</th>
        </tr>
      </thead>
      <tbody>
        {#each accounts as a (a.email)}
          <tr>
            <td>{a.email}</td>
            <td><span class="badge">{a.auth_method}</span></td>
            <td>{a.status}</td>
            <td class="muted">{a.health_summary}</td>
            <td class="row">
              <button
                onclick={() => test(a.email)}
                disabled={testingEmail === a.email}
              >
                {testingEmail === a.email ? "测试中…" : "测试"}
              </button>
              <button
                onclick={() => checkHealth(a.email)}
                disabled={checkingEmail === a.email}
              >
                {checkingEmail === a.email ? "检查中…" : "健康"}
              </button>
              <button class="danger" onclick={() => del(a.email)}>删除</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 14px;
  }
  h2 {
    margin: 0;
    font-size: 18px;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 14px;
  }
  .small {
    font-size: 12px;
  }
  .inline {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .warn-text {
    color: var(--warn);
    margin: 4px 0 0;
  }
</style>
