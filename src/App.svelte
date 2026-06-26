<script lang="ts">
  import { api, errMsg } from "./lib/api";
  import { toast, showToast } from "./lib/toast.svelte";
  import { t } from "./lib/i18n.svelte";
  import { fade } from "svelte/transition";
  import { appstate } from "./lib/appstate.svelte";
  import Unlock from "./components/Unlock.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Dashboard from "./components/Dashboard.svelte";
  import QuickView from "./components/QuickView.svelte";
  import Accounts from "./components/Accounts.svelte";
  import AddEmail from "./components/AddEmail.svelte";
  import Categories from "./components/Categories.svelte";
  import Settings from "./components/Settings.svelte";
  import Emails from "./components/Emails.svelte";
  import OnboardingWizard from "./components/onboarding/OnboardingWizard.svelte";

  type View =
    | "dashboard"
    | "quick"
    | "accounts"
    | "add"
    | "categories"
    | "settings"
    | "emails";

  let ready = $state(false);
  let initialized = $state(false);
  let unlocked = $state(false);
  let showWizard = $state(false); // 新后端 + 全新安装时显示引导向导
  let view = $state<View>("dashboard");
  let selectedEmail = $state<string>("");
  let selectedOpenMsg = $state<string>("");
  // 锁屏点击某封新邮件：解锁后跳转打开它
  let pendingOpen = $state<{ email: string; messageId: string } | null>(null);

  // 空闲自动锁定
  let lastActivity = Date.now();
  function resetActivity() { lastActivity = Date.now(); }

  $effect(() => {
    if (!unlocked) return;
    const EVENTS = ["mousemove", "keydown", "mousedown", "touchstart"] as const;
    EVENTS.forEach(e => window.addEventListener(e, resetActivity, { passive: true }));
    return () => EVENTS.forEach(e => window.removeEventListener(e, resetActivity));
  });

  $effect(() => {
    const mins = appstate.autoLockMins; // reactive dependency
    if (!unlocked || mins <= 0) return;
    const id = setInterval(() => {
      if (Date.now() - lastActivity > mins * 60_000) lock();
    }, 60_000);
    return () => clearInterval(id);
  });

  async function loadSettings() {
    try {
      const s = await api.getSettings();
      appstate.blockRemoteImages = s.block_remote_images;
      appstate.autoLockMins = s.auto_lock_mins;
    } catch { /* ignore */ }
  }

  async function loadStatus() {
    try {
      const s = await api.getStatus();
      initialized = s.initialized;
      unlocked = s.unlocked;
      // 探测新后端是否支持引导；不支持则回退原解锁流程（保持 v1.0.0 行为）
      try {
        await api.onboardingStatus();
        showWizard = !initialized;
      } catch {
        showWizard = false;
      }
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      ready = true;
    }
  }
  loadStatus();

  async function onUnlocked() {
    unlocked = true;
    initialized = true;
    if (pendingOpen) {
      selectedEmail = pendingOpen.email;
      selectedOpenMsg = pendingOpen.messageId;
      pendingOpen = null;
      view = "emails";
    } else {
      view = "dashboard";
    }
    await loadSettings();
  }

  async function onWizardDone() {
    unlocked = true;
    initialized = true;
    showWizard = false;
    view = "dashboard";
    await loadSettings();
  }

  async function lock() {
    try {
      await api.lock();
    } finally {
      unlocked = false;
    }
  }

  function navigate(v: View) {
    view = v;
  }
  function openMail(email: string) {
    selectedEmail = email;
    selectedOpenMsg = "";
    view = "emails";
  }
  function openMailMsg(email: string, messageId: string) {
    selectedEmail = email;
    selectedOpenMsg = messageId;
    view = "emails";
  }

  // 侧栏高亮：邮件视图归属「账户」
  let sidebarCurrent = $derived(view === "emails" ? "accounts" : view);
</script>

{#if !ready}
  <div class="center muted">{t("app.starting")}</div>
{:else if showWizard && !unlocked}
  <OnboardingWizard onComplete={onWizardDone} />
{:else if !unlocked}
  <Unlock {initialized} {onUnlocked} onPickLocked={(email, messageId) => (pendingOpen = { email, messageId })} />
{:else}
  <div class="shell">
    <Sidebar current={sidebarCurrent} onnavigate={navigate} onlock={lock} />
    <main class:full={view === "emails"}>
      {#key view}
        <div class="route" in:fade={{ duration: 150 }}>
          {#if view === "dashboard"}
            <Dashboard onnavigate={navigate} onopenmail={openMailMsg} />
          {:else if view === "quick"}
            <QuickView />
          {:else if view === "accounts"}
            <Accounts onopenmail={openMail} />
          {:else if view === "add"}
            <AddEmail onadded={() => navigate("accounts")} />
          {:else if view === "categories"}
            <Categories />
          {:else if view === "settings"}
            <Settings />
          {:else if view === "emails"}
            {#key selectedEmail + "|" + selectedOpenMsg}
              <Emails initialEmail={selectedEmail} openMessageId={selectedOpenMsg} />
            {/key}
          {/if}
        </div>
      {/key}
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
  .shell {
    height: 100%;
    display: flex;
  }
  main {
    flex: 1;
    min-width: 0;
    overflow: auto;
    padding: var(--s-lg);
  }
  main.full {
    padding: var(--s-md);
    overflow: hidden;
  }
  .route {
    min-height: 100%;
  }
</style>
