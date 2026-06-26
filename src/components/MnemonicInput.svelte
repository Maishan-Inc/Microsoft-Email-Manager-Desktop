<script lang="ts">
  // 助记词输入：一词一框（默认 12 框）。空格/回车跳下一格，退格回退，整段粘贴自动分配。
  let {
    value = $bindable(""),
    count = 12,
    autofocus = false,
    onsubmit,
  }: {
    value?: string;
    count?: number;
    autofocus?: boolean;
    onsubmit?: () => void;
  } = $props();

  let words = $state<string[]>([]);
  let inputs = $state<HTMLInputElement[]>([]);

  // 按 count 初始化/调整框数（保留已填内容）
  $effect(() => {
    if (words.length !== count) {
      words = Array.from({ length: count }, (_, i) => words[i] ?? "");
    }
  });

  function sync() {
    value = words.map((w) => w.trim()).filter(Boolean).join(" ");
  }

  function onInput(i: number) {
    // 单框内不允许空格（空格用于跳格）
    if (/\s/.test(words[i])) words[i] = words[i].replace(/\s+/g, "");
    sync();
  }

  function onKeydown(i: number, e: KeyboardEvent) {
    if (e.key === " " || e.key === "Spacebar") {
      e.preventDefault();
      if (i < count - 1) inputs[i + 1]?.focus();
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (i < count - 1 && words[i]) inputs[i + 1]?.focus();
      else onsubmit?.();
    } else if (e.key === "Backspace" && !words[i] && i > 0) {
      e.preventDefault();
      inputs[i - 1]?.focus();
    } else if (e.key === "ArrowLeft" && i > 0) {
      inputs[i - 1]?.focus();
    } else if (e.key === "ArrowRight" && i < count - 1) {
      inputs[i + 1]?.focus();
    }
  }

  function onPaste(i: number, e: ClipboardEvent) {
    const txt = e.clipboardData?.getData("text") || "";
    const parts = txt.trim().split(/\s+/).filter(Boolean);
    if (parts.length <= 1) return; // 单词：走默认输入
    e.preventDefault();
    for (let k = 0; k < parts.length && i + k < count; k++) {
      words[i + k] = parts[k].toLowerCase();
    }
    sync();
    inputs[Math.min(i + parts.length, count - 1)]?.focus();
  }

  $effect(() => {
    if (autofocus) inputs[0]?.focus();
  });
</script>

<div class="mn">
  {#each Array(count) as _, i (i)}
    <div class="mn-cell">
      <span class="mn-i">{i + 1}</span>
      <input
        bind:this={inputs[i]}
        class="mn-in"
        bind:value={words[i]}
        autocomplete="off"
        autocapitalize="none"
        autocorrect="off"
        spellcheck="false"
        oninput={() => onInput(i)}
        onkeydown={(e) => onKeydown(i, e)}
        onpaste={(e) => onPaste(i, e)}
      />
    </div>
  {/each}
</div>

<style>
  .mn {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--s-xs);
  }
  .mn-cell {
    display: flex;
    align-items: center;
    gap: 5px;
    background: var(--canvas-soft);
    border: 1px solid var(--hairline);
    border-radius: var(--r-sm);
    padding: 0 6px;
    height: 38px;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .mn-cell:focus-within {
    border-color: var(--ink);
    box-shadow: 0 0 0 3px var(--focus-ring);
    background: var(--canvas);
  }
  .mn-i {
    font-size: 11px;
    color: var(--mute);
    width: 16px;
    text-align: right;
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }
  .mn-in {
    border: none;
    background: transparent;
    height: auto;
    padding: 0;
    width: 100%;
    font-size: 13px;
    font-family: var(--font-mono);
  }
  .mn-in:focus {
    box-shadow: none;
  }
</style>
