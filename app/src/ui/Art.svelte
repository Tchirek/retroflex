<script lang="ts" module>
  // 一度見し　絵は二度目には　ただ其処に
  const seen = new Set<string>();
</script>

<script lang="ts">
  import { blurs } from "../lib/art.svelte";
  import type { Station } from "../lib/api";
  import Icon from "./Icon.svelte";

  let {
    station,
    cover,
    full = false,
    eager = false,
  }: { station: Station | null; cover?: string; full?: boolean; eager?: boolean } = $props();

  const src = $derived(station?.cover(cover, !full));
  const blur = $derived(cover ? blurs[cover] : undefined);
  let shown = $state("");
  let instant = $state(false);
  $effect.pre(() => {
    instant = !!src && seen.has(src);
    if (instant && src) shown = src;
  });
  let tries = 0;

  // 解けるまで　待ちて静かに　浮かび出る
  function reveal(e: Event) {
    const img = e.currentTarget as HTMLImageElement;
    const done = () => {
      const s = img.dataset.src ?? "";
      seen.add(s);
      if (img.isConnected) shown = s;
    };
    img.decode ? img.decode().then(done, done) : done();
  }

  // 届かねば　間を置きてまた　呼びにゆく
  function retry(e: Event) {
    const img = e.currentTarget as HTMLImageElement;
    if (tries >= 3 || !src) return;
    tries += 1;
    setTimeout(() => img.isConnected && (img.src = `${src}&r=${tries}`), tries * 900);
  }
</script>

<span class="art">
  {#if blur}<img class="ph" src={blur} alt="" aria-hidden="true" />{/if}
  {#if src}
    {#key src}
      <img
        class="img"
        class:on={shown === src}
        class:still={instant}
        {src}
        data-src={src}
        alt=""
        decoding="async"
        loading={eager ? "eager" : "lazy"}
        onload={reveal}
        onerror={retry}
      />
    {/key}
  {:else}
    <span class="empty"><Icon name="disc" size={40} stroke={1.1} /></span>
  {/if}
</span>

<style>
  .art {
    position: relative;
    display: block;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.06);
    border-radius: inherit;
  }
  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    -webkit-user-drag: none;
  }
  .ph {
    filter: blur(14px) saturate(1.2);
    transform: scale(1.15);
  }
  .img {
    opacity: 0;
    transition:
      opacity 0.32s ease,
      transform 0.35s var(--ease);
  }
  .img.on {
    opacity: 1;
  }
  .img.still {
    transition: none;
  }
  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: rgba(255, 255, 255, 0.18);
  }
</style>
