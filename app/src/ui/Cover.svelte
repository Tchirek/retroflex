<script lang="ts">
  import { fade } from "svelte/transition";
  import Icon from "./Icon.svelte";

  let { src, playing }: { src?: string; playing: boolean } = $props();
</script>

<!-- 止まる時　盤はわずかに　身を縮む -->
<div class="cover" class:rest={!playing}>
  {#key src}
    {#if src}
      <img {src} alt="" in:fade={{ duration: 500 }} />
    {:else}
      <span class="empty"><Icon name="disc" size={64} stroke={1.1} /></span>
    {/if}
  {/key}
</div>

<style>
  .cover {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    border-radius: clamp(10px, 1.2vw, 16px);
    overflow: hidden;
    background: var(--glass);
    box-shadow:
      0 30px 70px rgba(0, 0, 0, 0.42),
      0 0 0 0.5px rgba(255, 255, 255, 0.12);
    transition:
      transform 0.7s var(--spring),
      box-shadow 0.7s var(--ease);
  }
  .cover.rest {
    transform: scale(0.84);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.3);
  }
  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--fg-4);
  }
</style>
