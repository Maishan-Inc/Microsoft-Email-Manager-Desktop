<script lang="ts">
  import { api, errMsg } from "./lib/api";
  import { toast, showToast } from "./lib/toast.svelte";
  import Unlock from "./components/Unlock.svelte";
  import Accounts from "./components/Accounts.svelte";
  import Emails from "./components/Emails.svelte";

  let ready = $state(false);
  let initialized = $state(false);
  let unlocked = $state(false);
  let view = $state<"accounts" | "emails">("accounts");

  async function loadStatus() {
    try {
      const s = await api.getStatus();
      initialized = s.initialized;
      unlocked = s.unlocked;
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      ready = true;
    }
  }

  loadStatus();

  function onUnlocked() {
    unlocked = true;
    initialized = true;
  }

  async function lock() {
    try {
      await api.lock();
    } finally {
      unlocked = false;
    }
  }
</script>

{#if !ready}
  <div class="center muted">启动中…</div>
{:else if !unlocked}
  <Unlock {initialized} {onUnlocked} />
{:else}
  <div class="app">
    <header>
      <strong>Microsoft Email Manager Desktop</strong>
      <nav>
        <button class:active={view === "accounts"} onclick={() => (view = "accounts")}>
          账号管理
        </button>
        <button class:active={view === "emails"} onclick={() => (view = "emails")}>
          邮件查看
        </button>
      </nav>
      <button onclick={lock}>🔒 锁定</button>
    </header>
    <main class:full={view === "emails"}>
      {#if view === "accounts"}
        <Accounts />
      {:else}
        <Emails />
      {/if}
    </main>
  </div>
{/if}

{#if toast.visible}
  <div class="toast {toast.kind}">{toast.msg}</div>
{/if}

<style>
  .center {
    height: 100%;
    display: grid;
    place-items: center;
  }
  .app {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }
  nav {
    display: flex;
    gap: 8px;
  }
  nav .active {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  main {
    flex: 1;
    overflow: auto;
    padding: 18px;
    max-width: 1000px;
    width: 100%;
    margin: 0 auto;
  }
  main.full {
    max-width: none;
    overflow: hidden;
  }
</style>

