<script lang="ts">
  import type { Station } from "../lib/api";
  import Art from "./Art.svelte";

  let { station, cover, playing }: { station: Station | null; cover?: string; playing: boolean } = $props();
</script>

<!-- 止まる時　盤はわずかに　身を縮む -->
<div class="cover" class:rest={!playing}>
  <Art {station} {cover} full eager />
</div>

<style>
  .cover {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    border-radius: clamp(10px, 1.2vw, 16px);
    overflow: hidden;
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
</style>
