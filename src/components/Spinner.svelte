<script lang="ts">
  // 轻量转圈圈：随 currentColor 自适应，用于按钮加载态等。
  let { size = 16, stroke = 2 }: { size?: number; stroke?: number } = $props();
</script>

<span
  class="spinner"
  style="width:{size}px;height:{size}px;border-width:{stroke}px"
  aria-label="loading"
  role="status"
></span>

<style>
  .spinner {
    display: inline-block;
    box-sizing: border-box;
    border-style: solid;
    border-color: color-mix(in srgb, currentColor 28%, transparent);
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex-shrink: 0;
    vertical-align: middle;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
