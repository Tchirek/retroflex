<script lang="ts">
  import type { Snippet } from "svelte";
  import { fly } from "svelte/transition";
  import Icon from "./Icon.svelte";

  let { count, onclose, children }: { count: number; onclose: () => void; children: Snippet } = $props();
</script>

<!-- 選びたる　数と手立てを　硝子帯 -->
<div class="dock">
  <div class="picks" transition:fly={{ y: 18, duration: 240 }}>
    <button class="icon-btn" aria-label="Done" onclick={onclose}><Icon name="close" size={17} /></button>
    <span class="n">{count}</span>
    <span class="acts">{@render children()}</span>
  </div>
</div>

<style>
  .dock {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 12px;
    z-index: 5;
    display: flex;
    justify-content: center;
    pointer-events: none;
  }
  .picks {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 6px 4px 4px;
    border-radius: 999px;
    background: rgba(28, 28, 34, 0.72);
    border: 1px solid rgba(255, 255, 255, 0.08);
    backdrop-filter: blur(30px) saturate(170%);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
    pointer-events: auto;
  }
  .n {
    min-width: 28px;
    padding: 0 6px 0 2px;
    font-size: 14px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .acts {
    display: flex;
    gap: 2px;
    padding-left: 6px;
    border-left: 1px solid rgba(255, 255, 255, 0.1);
  }
</style>
