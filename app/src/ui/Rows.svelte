<script lang="ts">
  import { snap, type Snap, type Station, type Track } from "../lib/api";
  import { clock } from "../lib/library";
  import type { Mode } from "../lib/room.svelte";
  import AddToggle from "./AddToggle.svelte";
  import Art from "./Art.svelte";
  import Icon from "./Icon.svelte";

  let {
    tracks,
    station,
    numbered = false,
    art = true,
    onadd,
    queued,
    toggle,
  }: {
    tracks: Track[];
    station: Station | null;
    numbered?: boolean;
    art?: boolean;
    onadd: (t: Snap[], mode: Mode) => void;
    queued: (ids: string[]) => boolean;
    toggle: (t: Snap[]) => void;
  } = $props();
</script>

<!-- 一行に　今か次かか　末にかと -->
<div class="rows">
  {#each tracks as t, i (t.id)}
    {@const on = queued([t.id])}
    <div class="row" class:on>
      <button class="hit" onclick={() => onadd([snap(t)], "now")} aria-label={t.title}>
        {#if art}
          <span class="lead art"><Art {station} cover={t.cover} /><span class="go"><Icon name="play" size={16} /></span></span>
        {:else}
          <span class="lead no"><span class="n">{numbered ? (t.track ?? i + 1) : ""}</span><span class="go"><Icon name="play" size={15} /></span></span>
        {/if}
        <span class="meta">
          <span class="title ellipsis">{t.title}</span>
          <span class="sub ellipsis">{t.artist}{art ? ` · ${t.album}` : ""}</span>
        </span>
        <span class="dur">{clock(t.duration)}</span>
      </button>
      <span class="acts">
        <button class="icon-btn" aria-label="Play next" onclick={() => onadd([snap(t)], "next")}>
          <Icon name="playnext" size={18} />
        </button>
        <AddToggle {on} onclick={() => toggle([snap(t)])} />
      </span>
    </div>
  {/each}
</div>

<style>
  .rows {
    display: flex;
    flex-direction: column;
  }
  .row {
    display: flex;
    align-items: center;
    border-radius: 10px;
    content-visibility: auto;
    contain-intrinsic-size: auto 58px;
    transition: background 0.12s;
  }
  .row:hover {
    background: rgba(255, 255, 255, 0.08);
  }
  .hit {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 8px 10px;
    text-align: left;
  }
  .lead {
    position: relative;
    flex: none;
    display: grid;
    place-items: center;
  }
  .lead.art {
    width: 42px;
    height: 42px;
    border-radius: 6px;
    overflow: hidden;
  }
  .lead.no {
    width: 24px;
    height: 24px;
    color: var(--fg-3);
    font-size: 13.5px;
    font-variant-numeric: tabular-nums;
  }
  .go {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    opacity: 0;
    color: #fff;
    transition: opacity 0.12s;
  }
  .art .go {
    background: rgba(0, 0, 0, 0.45);
  }
  .row:hover .go {
    opacity: 1;
  }
  .row:hover .n {
    opacity: 0;
  }
  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title {
    font-weight: 600;
    font-size: 14.5px;
  }
  .sub {
    font-size: 13px;
    color: var(--fg-2);
  }
  .dur {
    color: var(--fg-3);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  .acts {
    display: flex;
    gap: 2px;
    padding-right: 6px;
    opacity: 0;
    transition: opacity 0.12s;
  }
  .row:hover .acts,
  .row.on .acts,
  .acts:focus-within {
    opacity: 1;
  }
  @media (hover: none) {
    .acts {
      opacity: 1;
    }
    .acts .icon-btn:first-child {
      display: none;
    }
  }
</style>
