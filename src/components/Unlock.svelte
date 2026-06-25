<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t } from "../lib/i18n.svelte";
  import logo from "../assets/logo-black.svg";

  let { initialized, onUnlocked }: { initialized: boolean; onUnlocked: () => void } =
    $props();

  let password = $state("");
  let confirm = $state("");
  let busy = $state(false);

  // 2FA 第二步
  let need2fa = $state(false);
  let code = $state("");

  async function submit() {
    if (busy) return;
    if (password.length < 8) {
      showToast(t("unlock.minLen"), "error");
      return;
    }
    if (!initialized && password !== confirm) {
      showToast(t("unlock.mismatch"), "error");
      return;
    }
    busy = true;
    try {
      if (initialized) {
        const r = await api.unlock(password);
        if (r?.needs_2fa) {
          need2fa = true;
          return;
        }
      } else {
        await api.setupMasterPassword(password);
      }
      password = "";
      confirm = "";
      onUnlocked();
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      busy = false;
    }
  }

  async function verify() {
    if (busy) return;
    busy = true;
    try {
      await api.verify2fa(code.trim());
      code = "";
      password = "";
      need2fa = false;
      onUnlocked();
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      busy = false;
    }
  }
</script>

<div class="wrap">
  <div class="card box">
    <div class="brand">
      <img src={logo} alt="logo" class="logo" />
      <strong>{t("app.name")}</strong>
    </div>

    {#if need2fa}
      <p class="muted">{t("unlock.need2fa")}</p>
      <input
        class="code"
        placeholder={t("ob.totp.tokenPlaceholder")}
        bind:value={code}
        inputmode="numeric"
        onkeydown={(e) => e.key === "Enter" && verify()}
      />
      <button class="primary" onclick={verify} disabled={busy}>
        {busy ? t("unlock.processing") : t("unlock.verify2fa")}
      </button>
    {:else}
      <p class="muted">
        {initialized ? t("unlock.subtitleUnlock") : t("unlock.subtitleSetup")}
      </p>
      <p class="muted small">{t("unlock.hint")}</p>

      <input
        type="password"
        placeholder={t("unlock.password")}
        bind:value={password}
        onkeydown={(e) => e.key === "Enter" && submit()}
      />
      {#if !initialized}
        <input
          type="password"
          placeholder={t("unlock.confirm")}
          bind:value={confirm}
          onkeydown={(e) => e.key === "Enter" && submit()}
        />
      {/if}

      <button class="primary" onclick={submit} disabled={busy}>
        {busy ? t("unlock.processing") : initialized ? t("unlock.unlock") : t("unlock.setup")}
      </button>
    {/if}
  </div>
</div>

<style>
  .wrap {
    height: 100%;
    display: grid;
    place-items: center;
    background:
      radial-gradient(60% 50% at 50% 0%, color-mix(in srgb, var(--link) 8%, transparent), transparent),
      var(--canvas-soft);
  }
  .box {
    width: 380px;
    display: flex;
    flex-direction: column;
    gap: var(--s-sm);
    box-shadow: var(--shadow-4);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--s-xs);
  }
  .logo {
    width: 32px;
    height: 32px;
  }
  .brand strong {
    font-size: 17px;
  }
  .small {
    font-size: 12px;
  }
  .code {
    text-align: center;
    letter-spacing: 4px;
    font-size: 18px;
  }
</style>
