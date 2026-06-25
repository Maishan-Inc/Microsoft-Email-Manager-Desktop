<script lang="ts">
  import { api, errMsg } from "./lib/api";
  import { toast, showToast } from "./lib/toast.svelte";
  import { t } from "./lib/i18n.svelte";
  import { fade } from "svelte/transition";
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

  function onUnlocked() {
    unlocked = true;
    initialized = true;
    view = "dashboard";
  }

  function onWizardDone() {
    unlocked = true;
    initialized = true;
    showWizard = false;
    view = "dashboard";
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
  <Unlock {initialized} {onUnlocked} />
{:else}
  <div class="shell">
    <Sidebar current={sidebarCurrent} onnavigate={navigate} onlock={lock} />
    <main class:full={view === "emails"}>
      {#key view}
        <div class="route" in:fade={{ duration: 150 }}>
          {#if view === "dashboard"}
            <Dashboard onnavigate={navigate} />
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
            {#key selectedEmail}
              <Emails initialEmail={selectedEmail} />
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
