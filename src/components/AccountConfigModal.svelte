<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";
  import type { AccountInfo, Catalog } from "../lib/types";

  let {
    account,
    catalog,
    onclose,
    onsaved,
  }: {
    account: AccountInfo;
    catalog: Catalog;
    onclose: () => void;
    onsaved: () => void;
  } = $props();

  // 凭据揭示（需二次校验）
  let needs2fa = $state(false);
  let secret = $state("");
  let revealed = $state<string | null>(null);
  let revealing = $state(false);
  api.authModeInfo().then((v) => (needs2fa = v)).catch(() => {});

  // 分类与标签（首次从 account 初始化一次）
  let categoryKey = $state<string>("");
  let tagKeys = $state<string[]>([]);
  let saving = $state(false);
  let inited = false;
  $effect(() => {
    if (!inited) {
      inited = true;
      categoryKey = account.category_key ?? "";
      tagKeys = [...account.tag_keys];
    }
  });

  function maskRight(s: string): string {
    const half = Math.ceil(s.length / 2);
    return s.slice(0, half) + "•".repeat(Math.max(8, s.length - half));
  }

  async function reveal() {
    if (revealing || !secret.trim()) return;
    revealing = true;
    try {
      const r = await api.revealCredentials(account.email, secret.trim());
      revealed = r.combined;
      secret = "";
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      revealing = false;
    }
  }

  async function copyToken() {
    if (!revealed) return;
    try {
      await navigator.clipboard.writeText(revealed);
      showToast(t("common.copied"), "ok");
    } catch {
      /* ignore */
    }
  }

  function toggleTag(key: string) {
    tagKeys = tagKeys.includes(key)
      ? tagKeys.filter((k) => k !== key)
      : [...tagKeys, key];
  }

  async function save() {
    if (saving) return;
    saving = true;
    try {
      await api.updateClassification(
        account.email,
        categoryKey || null,
        $state.snapshot(tagKeys),
      );
      showToast(t("common.save"), "ok");
      onsaved();
      onclose();
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      saving = false;
    }
  }
</script>

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal" role="dialog" aria-modal="true">
    <header class="m-head">
      <h2>{t("cfg.title")}</h2>
      <button class="ghost icon-btn" onclick={onclose} aria-label="close"><Icon name="x" size={18} /></button>
    </header>

    <div class="m-body stack">
      <!-- 邮箱地址 -->
      <div class="field">
        <span class="lbl">{t("cfg.email")}</span>
        <div class="value mono">{account.email} <span class="badge">{account.auth_method}</span></div>
      </div>

      <!-- 组合令牌（右半隐藏，验证后显示） -->
      <div class="field">
        <span class="lbl">{t("cfg.token")}</span>
        {#if revealed}
          <div class="token-row">
            <code class="value mono token">{revealed}</code>
            <button class="sm" onclick={copyToken}><Icon name="copy" size={14} /> {t("common.copy")}</button>
            <button class="sm ghost" onclick={() => (revealed = null)}><Icon name="eye-off" size={14} /> {t("cfg.hide")}</button>
          </div>
        {:else}
          <div class="value mono masked">••••••••••••••••••••••••</div>
          <div class="reveal-row">
            <input
              class="rev-input"
              type={needs2fa ? "text" : "password"}
              inputmode={needs2fa ? "numeric" : undefined}
              placeholder={needs2fa ? t("cfg.prompt2fa") : t("cfg.promptPw")}
              bind:value={secret}
              onkeydown={(e) => e.key === "Enter" && reveal()}
            />
            <button class="primary sm" disabled={revealing} onclick={reveal}>
              <Icon name="eye" size={14} /> {t("cfg.verifyShow")}
            </button>
          </div>
        {/if}
      </div>

      <!-- 分类与标签 -->
      <div class="field">
        <span class="lbl">{t("cfg.classification")}</span>
        <div class="cls">
          <span class="sub">{t("cfg.category")}</span>
          <select bind:value={categoryKey}>
            <option value="">{t("cfg.noCategory")}</option>
            {#each catalog.categories as c (c.key)}
              <option value={c.key}>{c.name_zh} / {c.name_en}</option>
            {/each}
          </select>
        </div>
        <div class="cls">
          <span class="sub">{t("cfg.tags")}</span>
          <div class="chips">
            {#if catalog.tags.length === 0}
              <span class="muted small">{t("cat.empty")}</span>
            {:else}
              {#each catalog.tags as tg (tg.key)}
                <button
                  class="chip-toggle"
                  class:on={tagKeys.includes(tg.key)}
                  onclick={() => toggleTag(tg.key)}
                >
                  {tg.name_zh}
                </button>
              {/each}
            {/if}
          </div>
        </div>
      </div>
    </div>

    <footer class="m-foot">
      <button onclick={onclose}>{t("common.cancel")}</button>
      <button class="primary" disabled={saving} onclick={save}>
        {saving ? t("unlock.processing") : t("common.save")}
      </button>
    </footer>
  </div>
</div>

<style>
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
    width: 620px;
    max-width: 100%;
    max-height: calc(100vh - 48px);
    background: var(--canvas);
    border-radius: var(--r-xl);
    box-shadow: var(--shadow-5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: rise 0.25s ease both;
  }
  @keyframes rise {
    from { opacity: 0; transform: translateY(12px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .m-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--s-md) var(--s-lg);
    border-bottom: 1px solid var(--hairline);
  }
  .m-head h2 {
    margin: 0;
    font-size: 17px;
  }
  .icon-btn {
    width: 34px;
    padding: 0;
  }
  .m-body {
    padding: var(--s-lg);
    overflow: auto;
    gap: var(--s-lg);
  }
  .m-foot {
    display: flex;
    justify-content: flex-end;
    gap: var(--s-xs);
    padding: var(--s-md) var(--s-lg);
    border-top: 1px solid var(--hairline);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--s-xs);
  }
  .lbl {
    font-weight: 600;
    font-size: 13px;
  }
  .sub {
    font-size: 12px;
    color: var(--mute);
  }
  .value {
    background: var(--canvas-soft);
    border: 1px solid var(--hairline);
    border-radius: var(--r-sm);
    padding: 10px 12px;
    font-size: 13px;
    word-break: break-all;
  }
  .masked {
    color: var(--mute);
    letter-spacing: 2px;
  }
  .token-row,
  .reveal-row {
    display: flex;
    gap: var(--s-xs);
    align-items: center;
  }
  .token {
    flex: 1;
    max-height: 96px;
    overflow: auto;
  }
  .rev-input {
    flex: 1;
  }
  .cls {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: var(--s-xs);
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
</style>
