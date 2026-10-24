<script lang="ts">
  import type { Station } from "../lib/api";
  import { clock } from "../lib/library";
  import type { Room } from "../lib/room.svelte";
  import Icon from "./Icon.svelte";

  let { room, station }: { room: Room; station: Station | null } = $props();

  const at = $derived(room.queue.findIndex((i) => i.qid === room.cur));
</script>

<!-- 次々と　誰かの選びし　歌並ぶ -->
<div class="queue scroll">
  {#each room.queue as item, i (item.qid)}
    {@const by = room.members[item.by]?.profile}
    <div class="row" class:now={item.qid === room.cur} class:past={at >= 0 && i < at}>
      <button class="hit" onclick={() => room.play(item.qid)} aria-label={item.title}>
        <span class="art">
          {#if station?.cover(item.cover)}<img src={station.cover(item.cover)} alt="" loading="lazy" />{/if}
        </span>
        <span class="meta">
          <span class="title ellipsis">{item.title}</span>
          <span class="artist ellipsis">{item.artist}</span>
        </span>
        {#if by}<span class="by" style:--hue={by.hue} title={by.nick}></span>{/if}
        <span class="dur">{clock(item.duration)}</span>
      </button>
      <button class="icon-btn x" aria-label="Remove" onclick={() => room.remove(item.qid)}><Icon name="close" size={16} /></button>
    </div>
  {/each}
</div>

<style>
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
  .row:hover {
    background: var(--glass);
  }
  .row.now {
    background: var(--glass-2);
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
  }
  .art {
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: 7px;
    overflow: hidden;
    background: var(--glass-2);
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
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
  .row:hover .x {
    opacity: 1;
  }
  @media (hover: none) {
    .x {
      opacity: 0.6;
    }
  }
</style>
