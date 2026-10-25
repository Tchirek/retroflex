<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";
  import type { Station } from "../lib/api";
  import { backable } from "../lib/back.svelte";
  import { haptic } from "../lib/haptics";
  import { clock } from "../lib/library";
  import { isApp } from "../lib/links";
  import { savePlaylist } from "../lib/playlists.svelte";
  import { press } from "../lib/press";
  import type { QItem, Room } from "../lib/room.svelte";
  import Art from "./Art.svelte";
  import Confirm from "./Confirm.svelte";
  import Icon from "./Icon.svelte";
  import Picks from "./Picks.svelte";
  import Tick from "./Tick.svelte";

  let { room, station }: { room: Room; station: Station | null } = $props();

  const at = $derived(room.queue.findIndex((i) => i.qid === room.cur));
  let saved = $state(false);

  // 今の列　手帳にそっと　書き写す
  function keep() {
    if (!station || saved) return;
    const tracks = room.queue.map(({ id, title, artist, album, duration, cover }) => ({ id, title, artist, album, duration, cover }));
    if (!savePlaylist(station.name, tracks)) return;
    haptic();
    saved = true;
    setTimeout(() => (saved = false), 1600);
  }

  // 広間では　己の歌と　籤ばかり
  const can = (item: QItem) => !room.public || ((item.by === room.me || !!item.auto) && item.qid !== room.cur);

  const picked = new SvelteSet<number>();
  let picking = $state(false);
  backable(() => picking, leave);

  function leave() {
    picking = false;
    picked.clear();
  }
  function choose(item: QItem) {
    if (!can(item)) return;
    haptic();
    if (!picking) {
      picking = true;
      picked.add(item.qid);
    } else if (picked.has(item.qid)) {
      picked.delete(item.qid);
      if (!picked.size) leave();
    } else picked.add(item.qid);
  }
  function hit(item: QItem) {
    if (picking) return choose(item);
    if (!room.public) room.play(item.qid);
  }
  function drop() {
    for (const q of picked) room.remove(q);
    leave();
  }
</script>

<!-- 次々と　誰かの選びし　歌並ぶ -->
<div class="wrap">
  <div class="queue scroll">
    <div class="head">
      {#if isApp}
        <button class="icon-btn" class:on={saved} aria-label="Save list" disabled={!room.queue.length} onclick={keep}>
          <Icon name={saved ? "check" : "save"} size={19} />
        </button>
      {/if}
    </div>
    {#each room.queue as item, i (item.qid)}
      {@const by = room.members[item.by]?.profile}
      {@const sel = picked.has(item.qid)}
      <div class="row" class:now={item.qid === room.cur} class:past={at >= 0 && i < at} class:sel class:fixed={room.public}>
        <button class="hit" use:press={can(item) ? () => choose(item) : undefined} onclick={() => hit(item)} aria-label={item.title}>
          <span class="art">
            <Art {station} cover={item.cover} />
            {#if picking && can(item)}<span class="tick"><Tick on={sel} /></span>{/if}
          </span>
          <span class="meta">
            <span class="title ellipsis">{item.title}</span>
            <span class="artist ellipsis">{item.artist}</span>
          </span>
          {#if item.auto}
            <span class="auto"><Icon name="shuffle" size={15} /></span>
          {:else if by}
            <span class="by" style:--hue={by.hue}></span>
          {/if}
          <span class="dur">{clock(item.duration)}</span>
        </button>
        {#if !picking && can(item)}
          <button class="icon-btn x" aria-label="Remove" onclick={() => room.remove(item.qid)}><Icon name="close" size={16} /></button>
        {/if}
      </div>
    {/each}
  </div>
  {#if picking}
    <Picks count={picked.size} onclose={leave}>
      <Confirm label="Remove" onconfirm={drop} size={19} />
    </Picks>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
    height: 100%;
  }
  .head {
    display: flex;
    justify-content: flex-end;
    min-height: 38px;
    padding: 0 2px 4px;
  }
  .head .icon-btn:disabled {
    opacity: 0.3;
  }
  .queue {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 0 40px;
  }
  .row {
    display: flex;
    align-items: center;
    border-radius: 12px;
    transition: background 0.2s var(--ease);
  }
  @media (hover: hover) {
    .row:hover {
      background: var(--glass);
    }
    .row:hover .x {
      opacity: 1;
    }
  }
  .row.now {
    background: var(--glass-2);
  }
  .row.sel {
    background: rgba(255, 255, 255, 0.11);
  }
  .row.past {
    opacity: 0.45;
  }
  .hit {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 7px 8px;
    text-align: left;
    -webkit-touch-callout: none;
  }
  .fixed .hit {
    cursor: default;
  }
  .art {
    position: relative;
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: 7px;
    overflow: hidden;
    background: var(--glass-2);
  }
  .tick {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.32);
  }
  .auto {
    color: var(--fg-3);
    flex: none;
  }
  .row.now .auto {
    color: var(--fg-2);
  }
  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title {
    font-weight: 600;
  }
  .artist {
    font-size: 13px;
    color: var(--fg-2);
  }
  .by {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: hsl(var(--hue) 70% 62%);
    flex: none;
  }
  .dur {
    font-size: 12.5px;
    color: var(--fg-3);
    font-variant-numeric: tabular-nums;
  }
  .x {
    width: 32px;
    height: 32px;
    margin-right: 4px;
    opacity: 0;
  }
  @media (hover: none) {
    .x {
      opacity: 0.6;
    }
  }
</style>
