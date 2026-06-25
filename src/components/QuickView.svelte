<script lang="ts">
  import { quickview } from "../lib/quickview.svelte";
  import { t } from "../lib/i18n.svelte";
  import Emails from "./Emails.svelte";
</script>

{#if !quickview.email}
  <div class="empty-wrap">
    <div class="card empty muted">{t("quick.empty")}</div>
  </div>
{:else}
  {#key quickview.email}
    <Emails initialEmail={quickview.email} />
  {/key}
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
