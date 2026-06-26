<script lang="ts" module>
  export type SelectBadge = { text: string; tone?: "primary" | "soft" };
  export type SelectOption = {
    value: string;
    label: string;
    sublabel?: string;
    badges?: SelectBadge[];
  };
</script>

<script lang="ts">
  import Icon from "./Icon.svelte";

  let {
    value = $bindable(""),
    options,
    placeholder = "",
    disabled = false,
    onchange,
    width = "",
    menuWidth = "",
    size = "md",
    align = "left",
  }: {
    value?: string;
    options: SelectOption[];
    placeholder?: string;
    disabled?: boolean;
    onchange?: (v: string) => void;
    width?: string;
    menuWidth?: string;
    size?: "sm" | "md";
    align?: "left" | "right";
  } = $props();

  let open = $state(false);
  let root = $state<HTMLDivElement>();
  let selected = $derived(options.find((o) => o.value === value) ?? null);

  function choose(v: string) {
    if (v !== value) {
      value = v;
      onchange?.(v);
    }
    open = false;
  }
  function toggle() {
    if (!disabled) open = !open;
  }
  function onDocClick(e: MouseEvent) {
    if (root && !root.contains(e.target as Node)) open = false;
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") open = false;
  }
  $effect(() => {
    if (!open) return;
    window.addEventListener("click", onDocClick, true);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("click", onDocClick, true);
      window.removeEventListener("keydown", onKey);
    };
  });
</script>

<div class="sel {size}" class:open bind:this={root} style:width={width || undefined}>
  <button
    type="button"
    class="trigger"
    {disabled}
    onclick={toggle}
    aria-haspopup="listbox"
    aria-expanded={open}
  >
    <span class="cur">
      {#if selected}
        <span class="cur-label">{selected.label}</span>
        {#if selected.badges?.length}
          <span class="cur-badges">
            {#each selected.badges as b}
              <span class="mini-badge" class:soft={b.tone !== "primary"}>{b.text}</span>
            {/each}
          </span>
        {/if}
      {:else}
        <span class="ph">{placeholder}</span>
      {/if}
    </span>
    <svg class="chev" viewBox="0 0 24 24" width="14" height="14" fill="none"
      stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="m6 9 6 6 6-6" />
    </svg>
  </button>

  {#if open}
    <div class="menu {align}" style:min-width={menuWidth || undefined} role="listbox">
      {#if options.length === 0}
        <div class="empty muted">{placeholder || "—"}</div>
      {:else}
        {#each options as o (o.value)}
          <button
            type="button"
            class="opt"
            class:on={o.value === value}
            role="option"
            aria-selected={o.value === value}
            onclick={() => choose(o.value)}
          >
            <span class="opt-main">
              <span class="opt-label">{o.label}</span>
              {#if o.sublabel}<span class="opt-sub">{o.sublabel}</span>{/if}
            </span>
            {#if o.badges?.length}
              <span class="opt-badges">
                {#each o.badges as b}
                  <span class="mini-badge" class:soft={b.tone !== "primary"}>{b.text}</span>
                {/each}
              </span>
            {/if}
            <span class="tick">
              {#if o.value === value}<Icon name="check" size={15} />{/if}
            </span>
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .sel {
    position: relative;
    display: inline-flex;
  }
  .trigger {
    width: 100%;
    justify-content: space-between;
    gap: var(--s-xs);
    height: 40px;
    padding: 0 var(--s-sm);
    font-weight: 400;
  }
  .sm .trigger {
    height: 36px;
  }
  .open .trigger {
    border-color: var(--ink);
    box-shadow: 0 0 0 3px var(--focus-ring);
  }
  .cur {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    overflow: hidden;
  }
  .cur-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cur-badges {
    display: inline-flex;
    gap: 4px;
    flex-shrink: 0;
  }
  .ph {
    color: var(--mute);
  }
  .chev {
    flex-shrink: 0;
    color: var(--mute);
    transition: transform 0.18s ease;
  }
  .open .chev {
    transform: rotate(180deg);
  }
  .menu {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 60;
    min-width: 100%;
    max-height: 320px;
    overflow: auto;
    padding: 5px;
    background: var(--canvas);
    border: 1px solid var(--hairline);
    border-radius: var(--r-lg);
    box-shadow: var(--shadow-5);
    display: flex;
    flex-direction: column;
    gap: 2px;
    animation: pop-in 0.14s ease both;
  }
  .menu.right {
    left: auto;
    right: 0;
  }
  @keyframes pop-in {
    from {
      opacity: 0;
      transform: translateY(-6px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
  .opt {
    width: 100%;
    justify-content: flex-start;
    gap: var(--s-xs);
    height: auto;
    min-height: 36px;
    padding: 7px var(--s-xs) 7px var(--s-sm);
    border: 1px solid transparent;
    background: transparent;
    border-radius: var(--r-sm);
    text-align: left;
    font-weight: 400;
  }
  .opt:hover {
    background: var(--canvas-soft-2);
    border-color: transparent;
  }
  .opt.on {
    background: var(--canvas-soft);
  }
  .opt-main {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }
  .opt-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .opt-sub {
    font-size: 11px;
    color: var(--mute);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .opt-badges {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 4px;
    justify-content: flex-end;
    flex-shrink: 0;
    max-width: 50%;
  }
  .tick {
    width: 15px;
    flex-shrink: 0;
    color: var(--link);
    display: inline-flex;
  }
  .mini-badge {
    display: inline-flex;
    align-items: center;
    padding: 1px 7px;
    border-radius: var(--r-full);
    font-size: 11px;
    line-height: 1.5;
    background: var(--primary);
    color: var(--on-primary);
    white-space: nowrap;
  }
  .mini-badge.soft {
    background: var(--canvas-soft-2);
    color: var(--body);
    border: 1px solid var(--hairline);
  }
  .empty {
    padding: var(--s-sm);
    font-size: 13px;
    text-align: center;
  }
</style>
