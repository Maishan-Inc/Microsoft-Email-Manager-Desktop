<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t } from "../lib/i18n.svelte";
  import { senderLogo, senderAddress } from "../lib/brandLogos";
  import { listen } from "@tauri-apps/api/event";
  import OtpInput from "./OtpInput.svelte";
  import MnemonicInput from "./MnemonicInput.svelte";
  import Spinner from "./Spinner.svelte";
  import Icon from "./Icon.svelte";
  import logo from "../assets/logo-black.png";
  import type { LockedMail, TotpSetup } from "../lib/types";

  let {
    initialized,
    onUnlocked,
    onPickLocked,
  }: {
    initialized: boolean;
    onUnlocked: () => void;
    onPickLocked?: (email: string, messageId: string) => void;
  } = $props();

  type Mode = "unlock" | "recoverMnemonic" | "recoverNewPw" | "recover2FA" | "recoverTotp";
  let mode = $state<Mode>("unlock");

  let password = $state("");
  let confirm = $state("");
  let busy = $state(false);

  // 2FA 第二步
  let need2fa = $state(false);
  let code = $state("");

  // 恢复
  let mnemonic = $state("");
  let newPw = $state("");
  let newPw2 = $state("");
  // 恢复后重设 2FA
  let totp = $state<TotpSetup | null>(null);
  let totpCode = $state("");
  function maskSecret(s: string): string {
    if (s.length <= 8) return s;
    return s.slice(0, 4) + "•".repeat(Math.max(4, s.length - 8)) + s.slice(-4);
  }

  // 锁定期间的新邮件（令牌即焚）
  let lockedMails = $state<LockedMail[]>([]);
  let pickedMsg = $state<string>("");
  async function loadLocked() {
    try {
      lockedMails = await api.lockedItems();
    } catch {
      lockedMails = [];
    }
  }
  $effect(() => {
    loadLocked();
    const un = listen("mail:locked-new", () => loadLocked());
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  });

  function pickLocked(m: LockedMail) {
    pickedMsg = m.message_id;
    onPickLocked?.(m.email, m.message_id);
    showToast(t("unlock.lockedHint"), "info");
  }
  function lockedInitial(from: string): string {
    const e = senderAddress(from) || from || "?";
    return (e[0] || "?").toUpperCase();
  }

  async function submit() {
    if (busy) return;
    if (password.length < 8) return showToast(t("unlock.minLen"), "error");
    if (!initialized && password !== confirm) return showToast(t("unlock.mismatch"), "error");
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

  async function recover() {
    if (busy) return;
    if (!mnemonic.trim()) return;
    busy = true;
    try {
      await api.recoverWithMnemonic(mnemonic.trim().toLowerCase());
      mnemonic = "";
      mode = "recoverNewPw";
      showToast(t("unlock.recovered"), "ok");
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      busy = false;
    }
  }

  async function saveNewPw() {
    if (busy) return;
    if (newPw.length < 8) return showToast(t("unlock.minLen"), "error");
    if (newPw !== newPw2) return showToast(t("unlock.mismatch"), "error");
    busy = true;
    try {
      await api.resetPassword(newPw);
      newPw = "";
      newPw2 = "";
      mode = "recover2FA";
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      busy = false;
    }
  }

  // 恢复后选择是否开启 2FA
  async function recoverSkip2fa() {
    if (busy) return;
    busy = true;
    try {
      await api.setTwoFactor(null);
      onUnlocked();
    } catch (e) {
      showToast(errMsg(e), "error");
      busy = false;
    }
  }
  async function recoverEnable2fa() {
    if (busy) return;
    busy = true;
    try {
      totp = await api.generateTotp();
      mode = "recoverTotp";
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      busy = false;
    }
  }
  async function recoverVerifyTotp() {
    if (busy || !totp) return;
    busy = true;
    try {
      const ok = await api.verifyTotpCode(totp.secret, totpCode.trim());
      if (ok) {
        await api.setTwoFactor(totp.secret);
        onUnlocked();
      } else {
        showToast(t("ob.totp.invalid"), "error");
        busy = false;
      }
    } catch (e) {
      showToast(errMsg(e), "error");
      busy = false;
    }
  }
  async function copyTotpSecret() {
    if (!totp) return;
    try {
      await navigator.clipboard.writeText(totp.secret);
      showToast(t("common.copied"), "ok");
    } catch {
      /* ignore */
    }
  }

  function backToUnlock() {
    mode = "unlock";
    need2fa = false;
    mnemonic = "";
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
      <OtpInput bind:value={code} autofocus oncomplete={verify} />
      <button class="primary" onclick={verify} disabled={busy || code.length < 6}>
        {#if busy}<Spinner size={16} />{:else}{t("unlock.verify2fa")}{/if}
      </button>
    {:else if mode === "recoverMnemonic"}
      <p class="muted">{t("unlock.recoverTitle")}</p>
      <p class="muted small">{t("unlock.recoverHint")}</p>
      <MnemonicInput bind:value={mnemonic} count={12} autofocus onsubmit={recover} />
      <button class="primary" onclick={recover} disabled={busy}>
        {#if busy}<Spinner size={16} />{:else}{t("unlock.recover")}{/if}
      </button>
      <button class="ghost link" onclick={backToUnlock}>{t("unlock.backToUnlock")}</button>
    {:else if mode === "recoverNewPw"}
      <p class="muted">{t("unlock.newPwTitle")}</p>
      <input type="password" placeholder={t("unlock.password")} bind:value={newPw} />
      <input type="password" placeholder={t("unlock.confirm")} bind:value={newPw2}
        onkeydown={(e) => e.key === "Enter" && saveNewPw()} />
      <button class="primary" onclick={saveNewPw} disabled={busy}>
        {#if busy}<Spinner size={16} />{:else}{t("unlock.setNewPw")}{/if}
      </button>
    {:else if mode === "recover2FA"}
      <p class="muted">{t("ob.2fa.title")}</p>
      <p class="muted small">{t("ob.2fa.desc")}</p>
      <button class="primary" disabled={busy} onclick={recoverEnable2fa}>
        {#if busy}<Spinner size={16} />{:else}{t("ob.2fa.enable")}{/if}
      </button>
      <button disabled={busy} onclick={recoverSkip2fa}>{t("ob.2fa.skip")}</button>
    {:else if mode === "recoverTotp"}
      <p class="muted">{t("ob.totp.title")}</p>
      <p class="muted small">{t("ob.totp.scanHint")}</p>
      {#if totp}
        <div class="qr">{@html totp.qr_svg}</div>
        <div class="secret">
          <code class="mono">{maskSecret(totp.secret)}</code>
          <button class="sm" onclick={copyTotpSecret}>{t("ob.totp.copy")}</button>
        </div>
      {/if}
      <OtpInput bind:value={totpCode} autofocus oncomplete={recoverVerifyTotp} />
      <button class="primary" disabled={busy || totpCode.length < 6} onclick={recoverVerifyTotp}>
        {#if busy}<Spinner size={16} />{:else}{t("ob.totp.verify")}{/if}
      </button>
    {:else}
      {#if initialized && lockedMails.length > 0}
        <div class="locked-box">
          <div class="locked-head">{t("unlock.lockedTitle")} <span class="muted">({lockedMails.length})</span></div>
          <div class="locked-list">
            {#each lockedMails as m (m.message_id)}
              {@const brand = senderLogo(m.from)}
              <button class="locked-row" class:picked={pickedMsg === m.message_id} onclick={() => pickLocked(m)}>
                <span class="lr-avatar" class:has-logo={!!brand}>
                  {#if brand}<img src={brand} alt="" />{:else}{lockedInitial(m.from)}{/if}
                </span>
                <span class="lr-body">
                  <span class="lr-from">{senderAddress(m.from) || m.from || "?"}</span>
                  <span class="lr-mailbox">{m.email}</span>
                </span>
                {#if m.has_code}
                  <span class="lr-code" title={t("unlock.lockedCopyLocked")}>
                    <Icon name="copy" size={12} /> {t("mail.codeDetected")}
                  </span>
                {/if}
              </button>
            {/each}
          </div>
        </div>
      {/if}
      <p class="muted">
        {initialized ? t("unlock.subtitleUnlock") : t("unlock.subtitleSetup")}
      </p>
      <p class="muted small">{t("unlock.hint")}</p>
      <input type="password" placeholder={t("unlock.password")} bind:value={password}
        onkeydown={(e) => e.key === "Enter" && submit()} />
      {#if !initialized}
        <input type="password" placeholder={t("unlock.confirm")} bind:value={confirm}
          onkeydown={(e) => e.key === "Enter" && submit()} />
      {/if}
      <button class="primary" onclick={submit} disabled={busy}>
        {#if busy}<Spinner size={16} />{:else}{initialized ? t("unlock.unlock") : t("unlock.setup")}{/if}
      </button>
      {#if initialized}
        <button class="ghost link" onclick={() => (mode = "recoverMnemonic")}>{t("unlock.forgot")}</button>
      {/if}
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
    width: 440px;
    display: flex;
    flex-direction: column;
    gap: var(--s-md);
    box-shadow: var(--shadow-4);
    padding: var(--s-xl);
  }
  .brand {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--s-sm);
  }
  .logo {
    width: 36px;
    height: 36px;
    object-fit: contain;
    border-radius: var(--r-md);
    filter: invert(var(--logo-invert));
  }
  .brand strong {
    font-size: 18px;
    white-space: nowrap;
  }
  .box input {
    height: 44px;
  }
  /* 标题下方的信息提示居中 */
  .box > p {
    text-align: center;
  }
  .small {
    font-size: 12px;
  }
  .link {
    align-self: center;
    height: 28px;
    color: var(--link);
    font-size: 13px;
  }
  .link:hover {
    background: transparent;
    text-decoration: underline;
  }

  /* 恢复后重设 2FA：二维码 + 密钥 */
  .qr {
    width: 180px;
    height: 180px;
    margin: 0 auto;
    background: #fff;
    border-radius: var(--r-md);
    padding: 8px;
  }
  .qr :global(svg) {
    width: 100%;
    height: 100%;
  }
  .secret {
    display: flex;
    align-items: center;
    gap: var(--s-xs);
    justify-content: center;
  }
  .secret code {
    background: var(--canvas-soft);
    padding: 4px 10px;
    border-radius: var(--r-sm);
    letter-spacing: 1px;
  }

  /* 锁定期间的新邮件列表 */
  .locked-box {
    border: 1px solid var(--hairline);
    border-radius: var(--r-md);
    background: var(--canvas-soft);
    padding: var(--s-xs);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .locked-head {
    font-size: 12px;
    font-weight: 600;
    padding: 2px 4px;
  }
  .locked-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 188px;
    overflow: auto;
  }
  .locked-row {
    display: flex;
    align-items: center;
    gap: var(--s-xs);
    width: 100%;
    height: auto;
    padding: 6px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--r-sm);
    text-align: left;
  }
  .locked-row:hover {
    background: var(--canvas);
    border-color: var(--hairline);
  }
  .locked-row.picked {
    border-color: var(--link);
    background: var(--link-bg-soft);
  }
  .lr-avatar {
    width: 28px;
    height: 28px;
    border-radius: var(--r-sm);
    background: var(--canvas-soft-2);
    display: grid;
    place-items: center;
    font-size: 12px;
    font-weight: 600;
    flex-shrink: 0;
    overflow: hidden;
  }
  .lr-avatar.has-logo {
    background: #fff;
    padding: 3px;
    box-shadow: 0 0 0 1px var(--hairline) inset;
  }
  .lr-avatar img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .lr-body {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }
  .lr-from {
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lr-mailbox {
    font-size: 11px;
    color: var(--mute);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lr-code {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
    padding: 2px 8px;
    border-radius: var(--r-full);
    font-size: 11px;
    /* 灰色：锁定时不可复制，需解锁 */
    background: var(--canvas-soft-2);
    color: var(--mute);
    border: 1px solid var(--hairline);
  }
</style>
