<script lang="ts">
  import { api, errMsg } from "../lib/api";
  import { showToast } from "../lib/toast.svelte";

  let { initialized, onUnlocked }: { initialized: boolean; onUnlocked: () => void } =
    $props();

  let password = $state("");
  let confirm = $state("");
  let busy = $state(false);

  async function submit() {
    if (busy) return;
    if (password.length < 8) {
      showToast("主密码至少 8 位", "error");
      return;
    }
    if (!initialized && password !== confirm) {
      showToast("两次输入不一致", "error");
      return;
    }
    busy = true;
    try {
      if (initialized) {
        await api.unlock(password);
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
</script>

<div class="wrap">
  <div class="card box">
    <h1>🔐 Microsoft Email Manager Desktop</h1>
    <p class="muted">
      {initialized ? "请输入主密码解锁本地数据库" : "首次使用，请设置一个主密码"}
    </p>
    <p class="muted small">
      数据全部加密保存在本机，主密码不会上传也无法找回，请牢记。
    </p>

    <input
      type="password"
      placeholder="主密码（至少 8 位）"
      bind:value={password}
      onkeydown={(e) => e.key === "Enter" && submit()}
    />
    {#if !initialized}
      <input
        type="password"
        placeholder="再次输入主密码"
        bind:value={confirm}
        onkeydown={(e) => e.key === "Enter" && submit()}
      />
    {/if}

    <button class="primary" onclick={submit} disabled={busy}>
      {busy ? "处理中…" : initialized ? "解锁" : "设置并进入"}
    </button>
  </div>
</div>

<style>
  .wrap {
    height: 100%;
    display: grid;
    place-items: center;
  }
  .box {
    width: 380px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h1 {
    margin: 0;
    font-size: 20px;
  }
  .small {
    font-size: 12px;
  }
</style>
