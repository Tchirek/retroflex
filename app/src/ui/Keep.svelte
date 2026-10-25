<script lang="ts">
  import type { Snap, Station } from "../lib/api";
  import { haptic } from "../lib/haptics";
  import { fetching, keep, kept, keyOf } from "../lib/offline.svelte";
  import Icon from "./Icon.svelte";

  let {
    station,
    tracks,
    size = 18,
    variant = "icon",
  }: { station: Station; tracks: (Snap & { mime?: string })[]; size?: number; variant?: "icon" | "pill" } = $props();

  const keys = $derived(tracks.map((t) => keyOf(station.name, t.id)));
  const done = $derived(keys.length > 0 && keys.every((k) => kept[k]));
  const busy = $derived(keys.some((k) => k in fetching));
  // 汲む程を　束の平均で　輪に描く
  const progress = $derived(busy ? keys.reduce((n, k) => n + (kept[k] ? 1 : (fetching[k] ?? 0)), 0) / keys.length : 0);
  const R = 8.5;
  const C = 2 * Math.PI * R;

  function go() {
    if (done || busy) return;
    haptic();
    keep(station, tracks);
  }
</script>

<button class={variant} class:done class:busy aria-label="Download" onclick={go}>
  {#if busy}
    <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
      <circle class="rim" cx="12" cy="12" r={R} />
      <circle class="fill" cx="12" cy="12" r={R} stroke-dasharray={C} stroke-dashoffset={C * (1 - Math.max(0.04, progress))} />
      <rect x="9.6" y="9.6" width="4.8" height="4.8" rx="1" />
    </svg>
  {:else}
    <Icon name={done ? "kept" : "download"} {size} />
  {/if}
</button>

<style>
  button {
    display: grid;
    place-items: center;
    color: var(--fg-2);
    transition:
      color 0.2s var(--ease),
      background 0.2s var(--ease),
      transform 0.25s var(--spring);
  }
  .icon {
    width: 38px;
    height: 38px;
    border-radius: 50%;
  }
  .pill {
    width: 52px;
    height: 36px;
    border-radius: 999px;
    color: var(--fg);
    background: var(--glass-2);
  }
  @media (hover: hover) {
    .icon:hover {
      color: var(--fg);
      background: var(--glass);
    }
    .pill:hover {
      background: var(--glass-3);
    }
  }
  button:active {
    transform: scale(0.9);
  }
  .done,
  .busy {
    color: var(--fg);
    cursor: default;
  }
  .done:active,
  .busy:active {
    transform: none;
  }
  svg {
    display: block;
    transform: rotate(-90deg);
  }
  circle {
    fill: none;
    stroke-width: 2;
  }
  .rim {
    stroke: rgba(255, 255, 255, 0.18);
  }
  .fill {
    stroke: currentColor;
    stroke-linecap: round;
    transition: stroke-dashoffset 0.25s linear;
  }
  rect {
    fill: currentColor;
  }
</style>
