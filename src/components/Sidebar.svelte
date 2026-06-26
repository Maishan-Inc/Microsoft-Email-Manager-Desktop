<script lang="ts">
  import logoBlack from "../assets/logo-black.png";
  import { t, type MsgKey } from "../lib/i18n.svelte";

  type View = "dashboard" | "quick" | "accounts" | "add" | "categories" | "settings";

  let {
    current,
    onnavigate,
    onlock,
  }: {
    current: string;
    onnavigate: (v: View) => void;
    onlock: () => void;
  } = $props();

  // 内联图标（Lucide 风，24x24 stroke）
  const icons: Record<string, string> = {
    dashboard: "M3 3h7v7H3z M14 3h7v7h-7z M14 14h7v7h-7z M3 14h7v7H3z",
    quick: "M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z",
    accounts: "M3 5h18v14H3z M3 6l9 7 9-7",
    add: "M12 3a9 9 0 100 18 9 9 0 000-18z M12 8v8 M8 12h8",
    categories:
      "M20.59 13.41l-7.17 7.17a2 2 0 01-2.83 0L2 12V2h10l8.59 8.59a2 2 0 010 2.82z M7 7h.01",
    settings:
      "M4 21v-7 M4 10V3 M12 21v-9 M12 8V3 M20 21v-5 M20 12V3 M1 14h6 M9 8h6 M17 16h6",
    lock: "M5 11h14v10H5z M8 11V7a4 4 0 018 0v4",
  };

  const items: { key: View; label: MsgKey }[] = [
    { key: "dashboard", label: "nav.dashboard" },
    { key: "quick", label: "nav.quick" },
    { key: "accounts", label: "nav.accounts" },
    { key: "add", label: "nav.add" },
    { key: "categories", label: "nav.categories" },
    { key: "settings", label: "nav.settings" },
  ];
</script>

<aside class="sidebar">
  <div class="brand">
    <img src={logoBlack} alt="logo" class="logo" />
    <span class="brand-name">MSEmailManager</span>
  </div>

  <nav class="nav">
    {#each items as item (item.key)}
      <button
        class="nav-item"
        class:active={current === item.key}
        onclick={() => onnavigate(item.key)}
      >
        <svg viewBox="0 0 24 24" class="ico" fill="none" stroke="currentColor"
          stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d={icons[item.key]} />
        </svg>
        <span>{t(item.label)}</span>
      </button>
    {/each}
  </nav>

  <button class="nav-item lock" onclick={onlock}>
    <svg viewBox="0 0 24 24" class="ico" fill="none" stroke="currentColor"
      stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
      <path d={icons.lock} />
    </svg>
    <span>{t("nav.lock")}</span>
  </button>
</aside>

<style>
  .sidebar {
    width: 224px;
    flex-shrink: 0;
    height: 100%;
    background: var(--canvas);
    border-right: 1px solid var(--hairline);
    display: flex;
    flex-direction: column;
    padding: var(--s-md) var(--s-sm);
    gap: var(--s-xs);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--s-xs);
    padding: var(--s-xs) var(--s-xs) var(--s-md);
  }
  .logo {
    width: 30px;
    height: 30px;
    object-fit: contain;
    border-radius: var(--r-sm);
    filter: invert(var(--logo-invert));
  }
  .brand-name {
    font-weight: 600;
    font-size: 14px;
    letter-spacing: -0.01em;
    line-height: 1.15;
  }
  .nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }
  /* 覆盖全局 button：导航项为左对齐、无边框、可显示激活态 */
  .nav-item {
    position: relative;
    justify-content: flex-start;
    width: 100%;
    height: 38px;
    padding: 0 var(--s-sm);
    border: 1px solid transparent;
    background: transparent;
    color: var(--body);
    border-radius: var(--r-sm);
    font-weight: 500;
    gap: var(--s-sm);
  }
  .nav-item:hover {
    background: var(--canvas-soft-2);
    border-color: transparent;
    color: var(--ink);
  }
  .nav-item.active {
    background: var(--canvas-soft);
    color: var(--ink);
  }
  .nav-item.active::before {
    content: "";
    position: absolute;
    left: -1px;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: var(--r-full);
    background: var(--ink);
  }
  .ico {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }
  .lock {
    margin-top: auto;
    color: var(--body);
  }
  .lock:hover {
    color: var(--error);
    background: var(--error-soft);
  }
</style>
