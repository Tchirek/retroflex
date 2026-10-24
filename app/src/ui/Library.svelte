<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { snap, type Snap, type Station, type Track } from "../lib/api";
  import { albums, clock, search, type Album } from "../lib/library";
  import type { Mode } from "../lib/room.svelte";
  import Icon from "./Icon.svelte";

  let {
    tracks,
    station,
    onadd,
    onclose,
  }: { tracks: Track[]; station: Station | null; onadd: (t: Snap[], mode: Mode) => void; onclose: () => void } = $props();

  let query = $state("");
  let open = $state<Album | null>(null);
  const shelf = $derived(albums(tracks));
  const found = $derived(search(tracks, query));

  // 一枚を　選べば針は　そこへ落つ
  function take(list: Track[], mode: Mode) {
    onadd(list.map(snap), mode);
  }
</script>

<div class="sheet" transition:fly={{ y: 40, duration: 380, opacity: 0 }}>
  <div class="head">
    {#if open}
      <button class="icon-btn" aria-label="Back" onclick={() => (open = null)}><Icon name="back" /></button>
    {/if}
    <label class="find glass">
      <Icon name="search" size={18} />
      <input bind:value={query} aria-label="Search" spellcheck="false" />
      {#if query}<button class="clear" aria-label="Clear" onclick={() => (query = "")}><Icon name="close" size={14} /></button>{/if}
    </label>
    <button class="icon-btn" aria-label="Close" onclick={onclose}><Icon name="close" /></button>
  </div>

  <div class="body scroll">
    {#if query}
      <!-- 探す名に　触れたる歌を　連ね見す -->
      <div class="rows" in:fade={{ duration: 150 }}>
        {#each found as t (t.id)}
          <div class="row">
            <button class="hit" onclick={() => take([t], "now")}>
              <span class="mini">{#if station?.cover(t.cover)}<img src={station.cover(t.cover)} alt="" loading="lazy" />{/if}</span>
              <span class="meta">
                <span class="title ellipsis">{t.title}</span>
                <span class="sub ellipsis">{t.artist} · {t.album}</span>
              </span>
              <span class="dur">{clock(t.duration)}</span>
            </button>
            <button class="icon-btn" aria-label="Add to queue" onclick={() => take([t], "end")}><Icon name="plus" size={18} /></button>
          </div>
        {/each}
      </div>
    {:else if open}
      <div class="album" in:fly={{ x: 30, duration: 300 }}>
        <div class="hero">
          <div class="cover big">{#if station?.cover(open.cover)}<img src={station.cover(open.cover)} alt="" />{/if}</div>
          <div class="info">
            <div class="name">{open.title}</div>
            <div class="by">{open.artist}</div>
            {#if open.year}<div class="year">{open.year}</div>{/if}
            <div class="acts">
              <button class="pill" aria-label="Play album" onclick={() => take(open!.tracks, "now")}><Icon name="play" size={18} /></button>
              <button class="pill" aria-label="Add album to queue" onclick={() => take(open!.tracks, "end")}><Icon name="plus" size={18} /></button>
            </div>
          </div>
        </div>
        <div class="rows">
          {#each open.tracks as t, i (t.id)}
            <div class="row">
              <button class="hit" onclick={() => take([t], "now")}>
                <span class="no">{t.track ?? i + 1}</span>
                <span class="meta">
                  <span class="title ellipsis">{t.title}</span>
                  {#if t.artist !== open.artist}<span class="sub ellipsis">{t.artist}</span>{/if}
                </span>
                <span class="dur">{clock(t.duration)}</span>
              </button>
              <button class="icon-btn" aria-label="Add to queue" onclick={() => take([t], "end")}><Icon name="plus" size={18} /></button>
            </div>
          {/each}
        </div>
      </div>
    {:else}
      <!-- 棚一面　盤の背表紙　ならびけり -->
      <div class="grid">
        {#each shelf as a (a.key)}
          <button class="card" onclick={() => (open = a)}>
            <span class="cover">
              {#if station?.cover(a.cover)}<img src={station.cover(a.cover)} alt="" loading="lazy" />{:else}<Icon name="disc" size={40} stroke={1.2} />{/if}
            </span>
            <span class="title ellipsis">{a.title}</span>
            <span class="sub ellipsis">{a.artist}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .sheet {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    padding: calc(var(--top) + var(--titlebar, 0px) + 14px) 0 0;
    background: rgba(14, 14, 20, 0.55);
    backdrop-filter: blur(40px) saturate(160%);
    -webkit-backdrop-filter: blur(40px) saturate(160%);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 clamp(16px, 4vw, 48px) 14px;
  }
  .find {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 520px;
    margin: 0 auto;
    height: 40px;
    padding: 0 14px;
    border-radius: 12px;
    color: var(--fg-3);
  }
  .find input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: none;
    color: var(--fg);
  }
  .clear {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--glass-2);
  }
  .body {
    flex: 1;
    min-height: 0;
    padding: 6px clamp(16px, 4vw, 48px) calc(var(--bottom) + 40px);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(clamp(140px, 16vw, 200px), 1fr));
    gap: 26px 22px;
  }
  .card {
    display: flex;
    flex-direction: column;
    min-width: 0;
    text-align: left;
    gap: 2px;
  }
  .cover {
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    width: 100%;
    margin-bottom: 8px;
    border-radius: 10px;
    overflow: hidden;
    color: var(--fg-4);
    background: var(--glass);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.25);
    transition: transform 0.35s var(--spring);
  }
  .card:hover .cover {
    transform: translateY(-3px) scale(1.02);
  }
  .card:active .cover {
    transform: scale(0.97);
  }
  .cover img,
  .mini img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .title {
    font-weight: 600;
    font-size: 14.5px;
  }
  .sub {
    font-size: 13px;
    color: var(--fg-2);
  }
  .album {
    max-width: 920px;
    margin: 0 auto;
  }
  .hero {
    display: flex;
    gap: clamp(18px, 4vw, 40px);
    align-items: flex-end;
    margin: 8px 0 26px;
  }
  .cover.big {
    width: clamp(140px, 28vw, 260px);
    flex: none;
    margin: 0;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.4);
  }
  .info {
    min-width: 0;
  }
  .name {
    font-size: clamp(22px, 3vw, 34px);
    font-weight: 800;
    line-height: 1.15;
    letter-spacing: -0.01em;
  }
  .by {
    font-size: clamp(16px, 2vw, 20px);
    color: var(--fg-2);
    margin-top: 4px;
  }
  .year {
    font-size: 13px;
    color: var(--fg-3);
    margin-top: 2px;
  }
  .acts {
    display: flex;
    gap: 10px;
    margin-top: 16px;
  }
  .pill {
    display: grid;
    place-items: center;
    width: 64px;
    height: 38px;
    border-radius: 999px;
    background: var(--glass-2);
    transition:
      transform 0.3s var(--spring),
      background 0.2s;
  }
  .pill:first-child {
    background: rgba(255, 255, 255, 0.92);
    color: #111;
  }
  .pill:active {
    transform: scale(0.92);
  }
  .rows {
    display: flex;
    flex-direction: column;
  }
  .row {
    display: flex;
    align-items: center;
    border-radius: 10px;
  }
  .row:hover {
    background: var(--glass);
  }
  .hit {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 10px;
    text-align: left;
  }
  .no {
    width: 22px;
    text-align: right;
    color: var(--fg-3);
    font-variant-numeric: tabular-nums;
    font-size: 13.5px;
  }
  .mini {
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: 6px;
    overflow: hidden;
    background: var(--glass-2);
  }
  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .dur {
    color: var(--fg-3);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
</style>
