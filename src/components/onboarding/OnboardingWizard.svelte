<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";
  import { t, i18n } from "../lib/i18n.svelte";
  import logo from "../assets/logo-black.svg";
  import type { TotpSetup } from "../lib/types";

  let { initialized, onComplete }: { initialized: boolean; onComplete: () => void } =
    $props();

  type Step =
    | "splash"
    | "agreement"
    | "password"
    | "ask2fa"
    | "totp"
    | "authmode"
    | "mnemonicIntro"
    | "mnemonicShow"
    | "mnemonicVerify"
    | "done"
    | "firstrun"
    | "tutorial";

  // 已初始化但未完成引导 → 直接到「第一次/老用户」
  let step = $state<Step>(initialized ? "firstrun" : "splash");
  let busy = $state(false);

  // 协议
  let reachedBottom = $state(false);
  function onScroll(e: Event) {
    const el = e.target as HTMLElement;
    if (el.scrollTop + el.clientHeight >= el.scrollHeight - 8) reachedBottom = true;
  }
  const AGREEMENT: Record<"zh" | "en", string[]> = {
    zh: [
      "欢迎使用 Microsoft Email Manager（开发商：Maishan Inc.）。在使用本软件前，请仔细阅读并同意以下条款。",
      "1. 本地存储与加密：本软件将你的邮箱账户凭据加密后保存在本机，不上传至任何服务器。主密码用于派生加密密钥，开发者无法获取，也无法为你找回。",
      "2. 恢复责任：若你配置了恢复助记词，请离线妥善保管；任何持有助记词或主密码的人都可解密你的数据。遗失主密码且未配置助记词将导致数据无法恢复。",
      "3. 使用范围：本软件仅供学习、研究与自用，请遵守 Microsoft 服务条款及当地法律法规，勿用于任何未授权或违法用途。",
      "4. 免责声明：本软件按「现状」提供，不对因使用造成的任何数据丢失或损失承担责任。",
      "5. 隐私：本软件不收集、不上传你的邮件内容与账户信息；所有数据处理均在本机完成。",
      "滚动到此处即表示你已阅读全部条款。点击下方按钮以表示同意并继续。",
    ],
    en: [
      "Welcome to Microsoft Email Manager (by Maishan Inc.). Please read and agree to the terms below before use.",
      "1. Local storage & encryption: your mailbox credentials are encrypted and stored on this device only, never uploaded. The master password derives the encryption key; the developer cannot access or recover it.",
      "2. Recovery responsibility: if you set a recovery phrase, store it offline and safely; anyone holding it (or your password) can decrypt your data. Losing the password without a phrase means data cannot be recovered.",
      "3. Acceptable use: for learning, research and personal use only. Comply with Microsoft's terms and your local laws; do not use for unauthorized or unlawful purposes.",
      "4. Disclaimer: the software is provided “as is”, without liability for any data loss or damages.",
      "5. Privacy: your mail content and account info are never collected or uploaded; all processing happens on this device.",
      "Scrolling here means you have read all terms. Click the button below to agree and continue.",
    ],
  };

  // 密码
  let password = $state("");
  let confirm = $state("");
  let strength = $derived(scorePwd(password));
  function scorePwd(p: string): 0 | 1 | 2 {
    if (p.length < 8) return 0;
    let v = 0;
    if (/[a-z]/.test(p)) v++;
    if (/[A-Z]/.test(p)) v++;
    if (/[0-9]/.test(p)) v++;
    if (/[^a-zA-Z0-9]/.test(p)) v++;
    return p.length >= 12 && v >= 3 ? 2 : 1;
  }

  // 2FA
  let use2fa = $state(false);
  let totp = $state<TotpSetup | null>(null);
  let totpCode = $state("");
  let authMode = $state("password_only");

  // 助记词
  let mnemonic = $state<string[]>([]);
  let useMnemonic = $state(false);
  let hiddenIdx = $state<number[]>([]);
  let fillWords = $state<Record<number, string>>({});

  function maskSecret(s: string): string {
    if (s.length <= 8) return s;
    return s.slice(0, 4) + "•".repeat(Math.max(4, s.length - 8)) + s.slice(-4);
  }

  // ---- 流程 ----
  async function agree() {
    try {
      await api.acceptAgreement();
    } catch (e) {
      console.warn("accept_agreement:", errMsg(e));
    }
    step = "password";
  }

  function passwordNext() {
    if (password.length < 8) return showToast(t("unlock.minLen"), "error");
    if (password !== confirm) return showToast(t("unlock.mismatch"), "error");
    step = "ask2fa";
  }

  function skip2fa() {
    use2fa = false;
    authMode = "password_only";
    step = "mnemonicIntro";
  }

  async function enable2fa() {
    busy = true;
    try {
      totp = await api.generateTotp();
      use2fa = true;
      step = "totp";
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      busy = false;
    }
  }

  async function verifyTotp() {
    if (!totp) return;
    busy = true;
    try {
      const ok = await api.verifyTotpCode(totp.secret, totpCode.trim());
      if (ok) {
        authMode = "password_2fa";
        step = "authmode";
      } else showToast(t("ob.totp.invalid"), "error");
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      busy = false;
    }
  }

  function pickAuthMode(m: string) {
    authMode = m;
    step = "mnemonicIntro";
  }

  async function copySecret() {
    if (!totp) return;
    try {
      await navigator.clipboard.writeText(totp.secret);
      showToast(t("common.copied"), "ok");
    } catch {
      /* ignore */
    }
  }

  async function genMnemonic() {
    busy = true;
    try {
      const r = await api.generateMnemonic();
      mnemonic = r.words;
      useMnemonic = true;
      step = "mnemonicShow";
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      busy = false;
    }
  }

  function skipMnemonic() {
    useMnemonic = false;
    doCompleteSetup();
  }

  function downloadTxt() {
    const blob = new Blob([mnemonic.join(" ")], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "recovery-phrase.txt";
    a.click();
    URL.revokeObjectURL(url);
  }

  function toVerify() {
    const idx = new Set<number>();
    const need = Math.min(4, mnemonic.length);
    while (idx.size < need) idx.add(Math.floor(Math.random() * mnemonic.length));
    hiddenIdx = [...idx].sort((a, b) => a - b);
    fillWords = {};
    step = "mnemonicVerify";
  }

  function confirmMnemonic() {
    for (const i of hiddenIdx) {
      if ((fillWords[i] ?? "").trim().toLowerCase() !== mnemonic[i].toLowerCase()) {
        return showToast(t("ob.mnem.verifyWrong"), "error");
      }
    }
    doCompleteSetup();
  }

  async function doCompleteSetup() {
    busy = true;
    try {
      await api.completeSetup({
        password,
        totp_secret: use2fa && totp ? totp.secret : null,
        auth_mode: authMode,
        mnemonic: useMnemonic ? mnemonic.join(" ") : null,
      });
      password = "";
      confirm = "";
      step = "done";
    } catch (e) {
      showToast(errMsg(e), "error");
    } finally {
      busy = false;
    }
  }

  async function finishOnboarding(seeTutorial: boolean) {
    if (seeTutorial) {
      step = "tutorial";
      return;
    }
    busy = true;
    try {
      await api.completeOnboarding();
      onComplete();
    } catch (e) {
      showToast(errMsg(e), "error");
      busy = false;
    }
  }

  let tutStep = $state(0);
  const tutTips = ["ob.tutorial.t1", "ob.tutorial.t2", "ob.tutorial.t3", "ob.tutorial.t4"] as const;
  async function finishTutorial() {
    busy = true;
    try {
      await api.setTutorialSeen();
      await api.completeOnboarding();
      onComplete();
    } catch (e) {
      showToast(errMsg(e), "error");
      busy = false;
    }
  }
</script>

<div class="onb">
  <div class="panel" class:wide={step === "mnemonicShow" || step === "mnemonicVerify"}>
    {#if step === "splash"}
      <div class="splash">
        <img src={logo} alt="logo" class="splash-logo" />
        <h1 class="splash-title">{t("app.name")}</h1>
        <p class="muted">{t("ob.splash.tagline")}</p>
        <button class="primary big" onclick={() => (step = "agreement")}>{t("ob.splash.start")}</button>
      </div>
    {:else if step === "agreement"}
      <h2>{t("ob.agreement.title")}</h2>
      <div class="agreement" onscroll={onScroll}>
        {#each AGREEMENT[i18n.lang] as para}
          <p>{para}</p>
        {/each}
      </div>
      <p class="muted small hint">{reachedBottom ? "" : t("ob.agreement.scrollHint")}</p>
      <button class="primary big" disabled={!reachedBottom} onclick={agree}>{t("ob.agreement.agree")}</button>
    {:else if step === "password"}
      <h2>{t("ob.password.title")}</h2>
      <p class="muted small">{t("ob.password.desc")}</p>
      <input type="password" placeholder={t("ob.password.pwd")} bind:value={password} />
      <input type="password" placeholder={t("ob.password.confirm")} bind:value={confirm}
        onkeydown={(e) => e.key === "Enter" && passwordNext()} />
      {#if password.length > 0}
        <div class="strength s{strength}">
          <span></span><span></span><span></span>
          <em>{strength === 0 ? t("ob.password.weak") : strength === 1 ? t("ob.password.medium") : t("ob.password.strong")}</em>
        </div>
      {/if}
      <button class="primary big" onclick={passwordNext}>{t("common.next")}</button>
    {:else if step === "ask2fa"}
      <h2>{t("ob.2fa.title")}</h2>
      <p class="muted small">{t("ob.2fa.desc")}</p>
      <div class="btn-col">
        <button class="primary big" disabled={busy} onclick={enable2fa}>{t("ob.2fa.enable")}</button>
        <button class="big" disabled={busy} onclick={skip2fa}>{t("ob.2fa.skip")}</button>
      </div>
    {:else if step === "totp"}
      <h2>{t("ob.totp.title")}</h2>
      <p class="muted small">{t("ob.totp.scanHint")}</p>
      {#if totp}
        <div class="qr">{@html totp.qr_svg}</div>
        <div class="secret">
          <code class="mono">{maskSecret(totp.secret)}</code>
          <button class="sm" onclick={copySecret}>{t("ob.totp.copy")}</button>
        </div>
      {/if}
      <input class="code" placeholder={t("ob.totp.tokenPlaceholder")} bind:value={totpCode} inputmode="numeric"
        onkeydown={(e) => e.key === "Enter" && verifyTotp()} />
      <div class="btn-row">
        <button onclick={() => (step = "ask2fa")}>{t("common.back")}</button>
        <button class="primary" disabled={busy} onclick={verifyTotp}>{t("ob.totp.verify")}</button>
      </div>
    {:else if step === "authmode"}
      <h2>{t("ob.authmode.title")}</h2>
      <div class="choices">
        <button class="choice" onclick={() => pickAuthMode("password_2fa")}>
          <strong>{t("ob.authmode.pw2fa")}</strong>
          <span class="muted small">{t("ob.authmode.pw2faDesc")}</span>
        </button>
        <button class="choice" onclick={() => pickAuthMode("standalone_2fa")}>
          <strong>{t("ob.authmode.standalone")}</strong>
          <span class="muted small">{t("ob.authmode.standaloneDesc")}</span>
        </button>
      </div>
      <p class="warn-text small">{t("ob.authmode.standaloneWarn")}</p>
    {:else if step === "mnemonicIntro"}
      <h2>{t("ob.mnem.introTitle")}</h2>
      <p class="muted small">{t("ob.mnem.introQ")}</p>
      <div class="btn-col">
        <button class="primary big" disabled={busy} onclick={genMnemonic}>{t("ob.mnem.generate")}</button>
        <button class="big" disabled={busy} onclick={skipMnemonic}>{t("ob.mnem.remember")}</button>
      </div>
    {:else if step === "mnemonicShow"}
      <h2>{t("ob.mnem.showTitle")}</h2>
      <p class="warn-text small">{t("ob.mnem.showDesc")}</p>
      <div class="words">
        {#each mnemonic as w, i (i)}
          <div class="word"><span class="wi">{i + 1}</span>{w}</div>
        {/each}
      </div>
      <div class="btn-row">
        <button onclick={downloadTxt}>⬇ {t("ob.mnem.download")}</button>
        <button class="primary" onclick={toVerify}>{t("common.next")}</button>
      </div>
    {:else if step === "mnemonicVerify"}
      <h2>{t("ob.mnem.verifyTitle")}</h2>
      <p class="muted small">{t("ob.mnem.verifyDesc")}</p>
      <div class="words">
        {#each mnemonic as w, i (i)}
          {#if hiddenIdx.includes(i)}
            <input class="word-in" placeholder={t("ob.mnem.word", { n: i + 1 })} bind:value={fillWords[i]} />
          {:else}
            <div class="word"><span class="wi">{i + 1}</span>{w}</div>
          {/if}
        {/each}
      </div>
      <div class="btn-row">
        <button onclick={() => (step = "mnemonicShow")}>{t("common.back")}</button>
        <button class="primary" disabled={busy} onclick={confirmMnemonic}>{t("common.confirm")}</button>
      </div>
    {:else if step === "done"}
      <div class="done">
        <div class="check">✓</div>
        <h2>{t("ob.done.title")}</h2>
        <p class="muted">{t("ob.done.desc")}</p>
        <button class="primary big" onclick={() => (step = "firstrun")}>{t("ob.done.start")}</button>
      </div>
    {:else if step === "firstrun"}
      <h2>{t("ob.firstrun.title")}</h2>
      <div class="choices">
        <button class="choice" disabled={busy} onclick={() => finishOnboarding(true)}>
          <strong>{t("ob.firstrun.first")}</strong>
          <span class="muted small">{t("ob.firstrun.firstDesc")}</span>
        </button>
        <button class="choice" disabled={busy} onclick={() => finishOnboarding(false)}>
          <strong>{t("ob.firstrun.returning")}</strong>
          <span class="muted small">{t("ob.firstrun.returningDesc")}</span>
        </button>
      </div>
    {:else if step === "tutorial"}
      <div class="tut">
        <div class="tut-n">{tutStep + 1} / {tutTips.length}</div>
        <p class="tut-tip">{t(tutTips[tutStep])}</p>
        <div class="btn-row">
          <button onclick={finishTutorial}>{t("ob.tutorial.skip")}</button>
          {#if tutStep < tutTips.length - 1}
            <button class="primary" onclick={() => (tutStep += 1)}>{t("ob.tutorial.next")}</button>
          {:else}
            <button class="primary" disabled={busy} onclick={finishTutorial}>{t("ob.tutorial.done")}</button>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .onb {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: center;
    background:
      radial-gradient(70% 60% at 50% -10%, color-mix(in srgb, var(--link) 10%, transparent), transparent),
      var(--canvas-soft);
    padding: var(--s-lg);
  }
  .panel {
    width: 440px;
    max-width: 100%;
    background: var(--canvas);
    border-radius: var(--r-xl);
    padding: var(--s-xl);
    box-shadow: var(--shadow-5);
    display: flex;
    flex-direction: column;
    gap: var(--s-sm);
    animation: rise 0.35s ease both;
  }
  .panel.wide {
    width: 560px;
  }
  @keyframes rise {
    from { opacity: 0; transform: translateY(12px); }
    to { opacity: 1; transform: translateY(0); }
  }
  h2 {
    margin: 0;
    font-size: 19px;
  }
  .big {
    height: 44px;
    font-size: 15px;
  }
  .btn-col {
    display: flex;
    flex-direction: column;
    gap: var(--s-xs);
    margin-top: var(--s-xs);
  }
  .btn-row {
    display: flex;
    gap: var(--s-xs);
    margin-top: var(--s-xs);
  }
  .btn-row button {
    flex: 1;
  }

  /* splash */
  .splash {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-xs);
    text-align: center;
    padding: var(--s-lg) 0;
  }
  .splash-logo {
    width: 72px;
    height: 72px;
    animation: pop 0.6s cubic-bezier(0.2, 0.9, 0.3, 1.3) both;
  }
  .splash-title {
    margin: var(--s-xs) 0 0;
    font-size: 22px;
  }
  @keyframes pop {
    from { opacity: 0; transform: scale(0.6) translateY(8px); }
    to { opacity: 1; transform: scale(1) translateY(0); }
  }

  /* agreement */
  .agreement {
    max-height: 240px;
    overflow: auto;
    border: 1px solid var(--hairline);
    border-radius: var(--r-md);
    padding: var(--s-sm) var(--s-md);
    background: var(--canvas-soft);
    font-size: 13px;
    line-height: 1.7;
  }
  .agreement p {
    margin: 0 0 var(--s-sm);
  }
  .hint {
    min-height: 16px;
    margin: 0;
    text-align: center;
  }

  /* strength */
  .strength {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .strength span {
    height: 4px;
    flex: 1;
    border-radius: var(--r-full);
    background: var(--hairline);
  }
  .strength em {
    font-style: normal;
    font-size: 11px;
    color: var(--mute);
    margin-left: var(--s-xs);
    white-space: nowrap;
  }
  .strength.s0 span:nth-child(1) { background: var(--error); }
  .strength.s1 span:nth-child(1),
  .strength.s1 span:nth-child(2) { background: var(--warning); }
  .strength.s2 span { background: var(--success); }

  /* totp */
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
  .code {
    text-align: center;
    letter-spacing: 4px;
    font-size: 18px;
  }

  /* choices */
  .choices {
    display: flex;
    flex-direction: column;
    gap: var(--s-xs);
  }
  .choice {
    height: auto;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: var(--s-sm) var(--s-md);
    text-align: left;
  }
  .choice strong {
    font-size: 14px;
  }
  .warn-text {
    color: var(--warning-deep);
    margin: var(--s-xs) 0 0;
  }

  /* mnemonic */
  .words {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--s-xs);
  }
  .word {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--canvas-soft);
    border: 1px solid var(--hairline);
    border-radius: var(--r-sm);
    padding: 6px 10px;
    font-size: 13px;
    font-family: var(--font-mono);
  }
  .wi {
    color: var(--mute);
    font-size: 11px;
    width: 16px;
  }
  .word-in {
    height: 34px;
    font-family: var(--font-mono);
    font-size: 13px;
  }

  /* done */
  .done {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--s-xs);
    padding: var(--s-md) 0;
  }
  .check {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: var(--success-soft);
    color: var(--success-ink);
    display: grid;
    place-items: center;
    font-size: 28px;
    animation: pop 0.5s cubic-bezier(0.2, 0.9, 0.3, 1.3) both;
  }

  /* tutorial */
  .tut {
    display: flex;
    flex-direction: column;
    gap: var(--s-sm);
  }
  .tut-n {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--mute);
  }
  .tut-tip {
    font-size: 16px;
    line-height: 1.6;
    min-height: 60px;
    margin: 0;
  }
</style>
