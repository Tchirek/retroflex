<script lang="ts">
  import { snap, type Snap, type Station, type Track } from "../lib/api";
  import { clock } from "../lib/library";
  import { isApp } from "../lib/links";
  import { press } from "../lib/press";
  import type { Mode } from "../lib/room.svelte";
  import AddToggle from "./AddToggle.svelte";
  import Art from "./Art.svelte";
  import Icon from "./Icon.svelte";
  import Keep from "./Keep.svelte";
  import Tick from "./Tick.svelte";

  let {
    tracks,
    station,
    numbered = false,
    art = true,
    onadd,
    queued,
    toggle,
    nowId,
    playing = false,
    onplaypause,
    picked,
    onpick,
  }: {
    tracks: Track[];
    station: Station | null;
    numbered?: boolean;
    art?: boolean;
    onadd: (t: Snap[], mode: Mode) => void;
    queued: (ids: string[]) => boolean;
    toggle: (t: Snap[]) => void;
    nowId?: string;
    playing?: boolean;
    onplaypause?: () => void;
    picked?: Set<string> | null;
    onpick?: (id: string) => void;
  } = $props();

  function hit(t: Track) {
    if (picked) return onpick?.(t.id);
    if (t.id === nowId && onplaypause) return onplaypause();
    onadd([snap(t)], "now");
  }

  const FIRST = 48;
  const BATCH = 32;
  let cap = $state(FIRST);
  const shown = $derived(tracks.slice(0, cap));

  // 行の群れ　少しずつ解き　声を待つ
  function grow(node: HTMLElement) {
    if (!("IntersectionObserver" in window)) {
      cap = tracks.length;
      return;
    }
    const root = node.closest<HTMLElement>(".scroll");
    let timer = 0;
    const io = new IntersectionObserver(
      (entries) => {
        if (!entries.some((e) => e.isIntersecting)) {
          clearTimeout(timer);
          return;
        }
        clearTimeout(timer);
        timer = window.setTimeout(() => {
          if (cap < tracks.length) cap = Math.min(tracks.length, cap + BATCH);
        }, 140);
      },
      { root, rootMargin: "800px 0px" },
    );
    io.observe(node);
    return {
      destroy: () => {
        clearTimeout(timer);
        io.disconnect();
      },
    };
  }

  // 束新し　行の数を　数えけり
  $effect(() => {
    void tracks;
    cap = FIRST;
  });
</script>

<!-- 一行に　今か次かか　末にかと -->
<div class="rows" class:picking={!!picked}>
  {#each shown as t, i (t.id)}
    {@const on = queued([t.id])}
    {@const here = t.id === nowId}
    {@const sel = !!picked?.has(t.id)}
    <div class="row" class:on class:here class:sel>
      <button class="hit" use:press={onpick && (() => onpick(t.id))} onclick={() => hit(t)} aria-label={t.title}>
        {#if art}
          <span class="lead art">
            <Art {station} cover={t.cover} />
            <span class="go"><Icon name={here && playing ? "pause" : "play"} size={16} /></span>
            {#if picked}<span class="tick"><Tick on={sel} /></span>{/if}
          </span>
        {:else}
          <span class="lead no">
            {#if picked}<Tick on={sel} />{:else}<span class="n">{numbered ? (t.track ?? i + 1) : ""}</span><span class="go"><Icon name={here && playing ? "pause" : "play"} size={15} /></span>{/if}
          </span>
        {/if}
        <span class="meta">
          <span class="title">{t.title}</span>
          <span class="sub ellipsis">{t.artist}{art ? ` · ${t.album}` : ""}<span class="tail"> · {clock(t.duration)}</span></span>
        </span>
        <span class="dur">{clock(t.duration)}</span>
      </button>
      {#if !picked}
        <span class="acts">
          {#if isApp && station}<Keep {station} tracks={[t]} />{/if}
          <button class="icon-btn nextup" aria-label="Play next" onclick={() => onadd([snap(t)], "next")}>
            <Icon name="playnext" size={18} />
          </button>
          <AddToggle {on} onclick={() => toggle([snap(t)])} />
        </span>
      {/if}
    </div>
  {/each}
  {#if shown.length < tracks.length}<div class="more" use:grow aria-hidden="true"></div>{/if}
</div>

<style>
  .rows {
    display: flex;
    flex-direction: column;
    container-type: inline-size;
  }
  .more {
    height: 1px;
  }
  .row {
    display: flex;
    align-items: center;
    min-height: 58px;
    border-radius: 10px;
    content-visibility: auto;
    contain-intrinsic-size: auto 58px;
    transition: background 0.12s;
  }
  @media (hover: hover) {
    .row:hover {
      background: rgba(255, 255, 255, 0.08);
    }
    .row:hover .go {
      opacity: 1;
    }
    .row:hover .n {
      opacity: 0;
    }
    .row:hover .acts {
      opacity: 1;
    }
  }
  .row.sel {
    background: rgba(255, 255, 255, 0.1);
  }
  .hit {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 8px 10px;
    text-align: left;
    -webkit-touch-callout: none;
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
  .picking .go {
    display: none;
  }
  .tick {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.32);
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
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .here .title {
    color: #fff;
    text-shadow: 0 0 18px rgba(255, 255, 255, 0.35);
  }
  .here .sub {
    color: var(--fg);
  }
  .sub {
    font-size: 13px;
    color: var(--fg-2);
  }
  .tail {
    display: none;
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
  .row.on .acts,
  .acts:focus-within {
    opacity: 1;
  }
  @media (hover: none) {
    .acts {
      opacity: 1;
    }
    .acts .nextup {
      display: none;
    }
  }
  /* 狭き窓　名は二行まで　長さは下へ */
  @container (max-width: 520px) {
    .hit {
      gap: 12px;
      padding: 7px 4px 7px 8px;
    }
    .title {
      white-space: normal;
      display: -webkit-box;
      -webkit-box-orient: vertical;
      -webkit-line-clamp: 2;
      line-clamp: 2;
      line-height: 1.25;
      font-size: 14px;
      overflow-wrap: anywhere;
    }
    .sub {
      font-size: 12.5px;
      margin-top: 1px;
    }
    .tail {
      display: inline;
    }
    .dur {
      display: none;
    }
    .acts {
      gap: 0;
      padding-right: 2px;
    }
  }
</style>
