<script lang="ts">
  // 6 位（可配）一框一位的验证码输入：自动跳格、退格回退、粘贴分发、方向键移动。
  let {
    value = $bindable(""),
    length = 6,
    disabled = false,
    autofocus = false,
    oncomplete,
  }: {
    value?: string;
    length?: number;
    disabled?: boolean;
    autofocus?: boolean;
    oncomplete?: (v: string) => void;
  } = $props();

  let inputs = $state<HTMLInputElement[]>([]);

  // 每个框对应的字符（从受控 value 派生，保证一致性）
  let digits = $derived.by(() => {
    const arr = (value || "").replace(/\D/g, "").slice(0, length).split("");
    return Array.from({ length }, (_, i) => arr[i] ?? "");
  });

  function commit(next: string) {
    const cleaned = next.replace(/\D/g, "").slice(0, length);
    value = cleaned;
    if (cleaned.length === length) oncomplete?.(cleaned);
  }

  function onInput(i: number, e: Event) {
    const el = e.target as HTMLInputElement;
    const typed = el.value.replace(/\D/g, "");
    const cur = digits.slice();
    if (typed.length === 0) {
      cur[i] = "";
      commit(cur.join(""));
      return;
    }
    // 取最后一位输入（覆盖当前框）
    cur[i] = typed[typed.length - 1];
    commit(cur.join(""));
    if (i < length - 1) inputs[i + 1]?.focus();
  }

  function onKeydown(i: number, e: KeyboardEvent) {
    if (e.key === "Backspace") {
      if (!digits[i] && i > 0) {
        inputs[i - 1]?.focus();
        e.preventDefault();
      }
    } else if (e.key === "ArrowLeft" && i > 0) {
      inputs[i - 1]?.focus();
      e.preventDefault();
    } else if (e.key === "ArrowRight" && i < length - 1) {
      inputs[i + 1]?.focus();
      e.preventDefault();
    }
  }

  function onPaste(e: ClipboardEvent) {
    e.preventDefault();
    const txt = (e.clipboardData?.getData("text") || "")
      .replace(/\D/g, "")
      .slice(0, length);
    if (!txt) return;
    commit(txt);
    const idx = Math.min(txt.length, length - 1);
    inputs[idx]?.focus();
  }

  $effect(() => {
    if (autofocus) inputs[0]?.focus();
  });
</script>

<div class="otp" role="group" aria-label="verification code">
  {#each Array(length) as _, i (i)}
    <input
      bind:this={inputs[i]}
      class="otp-box"
      class:filled={!!digits[i]}
      type="text"
      inputmode="numeric"
      autocomplete="one-time-code"
      maxlength="1"
      value={digits[i]}
      {disabled}
      oninput={(e) => onInput(i, e)}
      onkeydown={(e) => onKeydown(i, e)}
      onpaste={onPaste}
      onfocus={(e) => (e.target as HTMLInputElement).select()}
      aria-label={`digit ${i + 1}`}
    />
  {/each}
</div>

<style>
  .otp {
    display: flex;
    gap: var(--s-xs);
    justify-content: center;
  }
  .otp-box {
    width: 50px;
    height: 58px;
    padding: 0;
    text-align: center;
    font-size: 24px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    border-radius: var(--r-md);
    background: var(--canvas-soft);
    transition: border-color 0.15s, box-shadow 0.15s, background 0.15s;
  }
  .otp-box.filled {
    background: var(--canvas);
    border-color: var(--hairline-strong);
  }
  .otp-box:focus {
    border-color: var(--ink);
    background: var(--canvas);
    box-shadow: 0 0 0 3px var(--focus-ring);
  }
  @media (max-width: 420px) {
    .otp-box {
      width: 40px;
      height: 48px;
      font-size: 19px;
    }
  }
</style>
