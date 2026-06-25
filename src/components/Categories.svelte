<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t } from "../lib/i18n.svelte";
  import type { Catalog, ClassificationOption } from "../lib/types";

  let catalog = $state<Catalog>({ categories: [], tags: [] });
  let loading = $state(false);

  function blank(): ClassificationOption {
    return { key: "", name_zh: "", name_en: "", remark: "", created_at: null };
  }
  let newCat = $state<ClassificationOption>(blank());
  let newTag = $state<ClassificationOption>(blank());

  async function load() {
    loading = true;
    try {
      catalog = await api.getCatalog();
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      loading = false;
    }
  }
  load();

  async function add(kind: "category" | "tag") {
    const src = kind === "category" ? newCat : newTag;
    if (!src.key.trim() || !src.name_zh.trim()) {
      showToast(t("cat.key") + " / " + t("cat.nameZh"), "error");
      return;
    }
    const opt: ClassificationOption = {
      key: src.key.trim(),
      name_zh: src.name_zh.trim(),
      name_en: src.name_en.trim() || src.name_zh.trim(),
      remark: src.remark?.trim() || null,
      created_at: new Date().toISOString(),
    };
    try {
      if (kind === "category") {
        await api.addCategory(opt);
        newCat = blank();
      } else {
        await api.addTag(opt);
        newTag = blank();
      }
      await load();
    } catch (e) {
      showToast(errMsg(e), "error");
    }
  }

  async function del(kind: "category" | "tag", key: string) {
    if (!confirm(`${t("common.delete")} ${key}?`)) return;
    try {
      if (kind === "category") await api.deleteCategory(key);
      else await api.deleteTag(key);
      await load();
    } catch (e) {
      showToast(errMsg(e), "error");
    }
  }
</script>

<div class="view">
  <header class="view-head"><h1>{t("cat.title")}</h1></header>

  <div class="grid">
    <!-- 分类 -->
    <div class="card stack">
      <h2>{t("cat.categories")} <span class="muted small">({catalog.categories.length})</span></h2>
      {#if loading}
        <p class="muted">{t("common.loading")}</p>
      {:else if catalog.categories.length === 0}
        <p class="muted small">{t("cat.empty")}</p>
      {:else}
        <ul class="list">
          {#each catalog.categories as it (it.key)}
            <li>
              <span class="badge mono">{it.key}</span>
              <span class="names">{it.name_zh} · {it.name_en}</span>
              {#if it.remark}<span class="muted small rm">{it.remark}</span>{/if}
              <button class="ghost sm danger" onclick={() => del("category", it.key)} aria-label={t("common.delete")}>✕</button>
            </li>
          {/each}
        </ul>
      {/if}
      <div class="addform">
        <div class="row">
          <input class="k" placeholder={t("cat.key")} bind:value={newCat.key} />
          <input placeholder={t("cat.nameZh")} bind:value={newCat.name_zh} />
        </div>
        <div class="row">
          <input placeholder={t("cat.nameEn")} bind:value={newCat.name_en} />
          <input placeholder={t("cat.remark")} bind:value={newCat.remark} />
        </div>
        <button class="primary" onclick={() => add("category")}>＋ {t("common.add")}</button>
      </div>
    </div>

    <!-- 标签 -->
    <div class="card stack">
      <h2>{t("cat.tags")} <span class="muted small">({catalog.tags.length})</span></h2>
      {#if loading}
        <p class="muted">{t("common.loading")}</p>
      {:else if catalog.tags.length === 0}
        <p class="muted small">{t("cat.empty")}</p>
      {:else}
        <ul class="list">
          {#each catalog.tags as it (it.key)}
            <li>
              <span class="badge mono">{it.key}</span>
              <span class="names">{it.name_zh} · {it.name_en}</span>
              {#if it.remark}<span class="muted small rm">{it.remark}</span>{/if}
              <button class="ghost sm danger" onclick={() => del("tag", it.key)} aria-label={t("common.delete")}>✕</button>
            </li>
          {/each}
        </ul>
      {/if}
      <div class="addform">
        <div class="row">
          <input class="k" placeholder={t("cat.key")} bind:value={newTag.key} />
          <input placeholder={t("cat.nameZh")} bind:value={newTag.name_zh} />
        </div>
        <div class="row">
          <input placeholder={t("cat.nameEn")} bind:value={newTag.name_en} />
          <input placeholder={t("cat.remark")} bind:value={newTag.remark} />
        </div>
        <button class="primary" onclick={() => add("tag")}>＋ {t("common.add")}</button>
      </div>
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
    margin: 0;
    font-size: 15px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--s-md);
    align-items: start;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .list li {
    display: flex;
    align-items: center;
    gap: var(--s-xs);
    padding: var(--s-xs) 0;
    border-bottom: 1px solid var(--hairline);
  }
  .names {
    font-size: 13px;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rm {
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .addform {
    display: flex;
    flex-direction: column;
    gap: var(--s-xs);
    border-top: 1px solid var(--hairline);
    padding-top: var(--s-sm);
  }
  .k {
    max-width: 120px;
  }
  @media (max-width: 860px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
