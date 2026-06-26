<script lang="ts">
  import { quickview } from "../lib/quickview.svelte";
  import { t } from "../lib/i18n.svelte";
  import Emails from "./Emails.svelte";
</script>

{#if quickview.emails.length === 0}
  <div class="empty-wrap">
    <div class="card empty muted">{t("quick.emptyPinned")}</div>
  </div>
{:else}
  <Emails quick={true} />
{/if}

<style>
  .empty-wrap {
    display: grid;
    place-items: center;
    height: 100%;
  }
  .empty {
    text-align: center;
    padding: var(--s-2xl);
    max-width: 520px;
  }
</style>
