<script lang="ts">
  import { fade, fly } from "svelte/transition";
  import { snap, type Snap, type Station, type Track } from "../lib/api";
  import { albums, search, type Album } from "../lib/library";
  import { haptic } from "../lib/haptics";
  import { isApp, openExternal } from "../lib/links";
  import { dropPlaylist, playlists, type Playlist } from "../lib/playlists.svelte";
  import type { Mode } from "../lib/room.svelte";
  import AddToggle from "./AddToggle.svelte";
  import Art from "./Art.svelte";
  import Icon from "./Icon.svelte";
  import Keep from "./Keep.svelte";
  import Rows from "./Rows.svelte";

  let {
    tracks,
    station,
    onadd,
    queued,
    toggle,
    nowId,
    playing,
    onplaypause,
  }: {
    tracks: Track[];
    station: Station | null;
    onadd: (t: Snap[], mode: Mode) => void;
    queued: (ids: string[]) => boolean;
    toggle: (t: Snap[]) => void;
    nowId?: string;
    playing: boolean;
    onplaypause: () => void;
  } = $props();

  const ids = (list: Track[]) => list.map((t) => t.id);
  const live = (list: Track[]) => !!nowId && list.some((t) => t.id === nowId);

  // 鳴る盤は　止めも出来るぞ　同じ釦
  function start(list: Track[]) {
    if (live(list)) onplaypause();
    else take(list, "now");
  }

  type Shelf = "albums" | "songs" | "lists";
  const SHELVES: Shelf[] = ["albums", "songs", "lists"];
  let query = $state("");
  let open = $state<Album | null>(null);
  let list = $state<Playlist | null>(null);
  let mode = $state<Shelf>("albums");
  const shelf = $derived(albums(tracks));
  const found = $derived(search(tracks, query));
  const byId = $derived(new Map(tracks.map((t) => [t.id, t])));
  const mine = $derived(playlists.filter((p) => !station || p.station === station.name));

  // 札だけの　歌も棚から　身を借りる
  function full(s: Snap[]): Track[] {
    return s.map((t) => byId.get(t.id) ?? { ...t, mime: "", size: 0, lyrics: false });
  }

  function shelve(m: Shelf) {
    if (m !== mode) haptic();
    mode = m;
    open = null;
    list = null;
  }

  function stamp(at: number): string {
    const d = new Date(at);
    return `${d.getMonth() + 1}-${String(d.getDate()).padStart(2, "0")}`;
  }

  // 外の札　この窓からは　開かずに
  function out(e: MouseEvent) {
    e.preventDefault();
    openExternal((e.currentTarget as HTMLAnchorElement).href);
  }

  // 盤ひとつ　まるごと今か　末尾にか
  function take(list: Track[], mode: Mode) {
    onadd(list.map(snap), mode);
  }
</script>

<div class="library">
  <div class="head">
    {#if (open || list) && !query}
      <button class="icon-btn" aria-label="Back" onclick={() => ((open = null), (list = null))}><Icon name="back" /></button>
    {/if}
    <label class="find">
      <Icon name="search" size={17} />
      <input bind:value={query} aria-label="Search" spellcheck="false" />
      {#if query}<button class="clear" aria-label="Clear" onclick={() => (query = "")}><Icon name="close" size={13} /></button>{/if}
    </label>
    <div class="seg" style:--i={SHELVES.indexOf(mode)}>
      <span class="knob"></span>
      <button class:on={mode === "albums"} aria-label="Albums" onclick={() => shelve("albums")}><Icon name="library" size={18} /></button>
      <button class:on={mode === "songs"} aria-label="Songs" onclick={() => shelve("songs")}><Icon name="songs" size={18} /></button>
      <button class:on={mode === "lists"} aria-label="Saved lists" onclick={() => shelve("lists")}><Icon name="lists" size={18} /></button>
    </div>
  </div>

  <div class="body scroll">
    {#if query}
      <div in:fade={{ duration: 120 }}><Rows tracks={found} {station} {onadd} {queued} {toggle} {nowId} {playing} {onplaypause} /></div>
    {:else if open}
      {@const busy = live(open.tracks) && playing}
      <!-- 盤を開き　針を落とすも　積むも良し -->
      <div class="album" in:fly={{ x: 24, duration: 260 }}>
        <div class="hero">
          <span class="cover big"><Art {station} cover={open.cover} full eager /></span>
          <div class="info">
            <div class="name">{open.title}</div>
            <div class="by">{open.artist}</div>
            {#if open.year}<div class="year">{open.year}</div>{/if}
            <div class="acts">
              <button class="pill solid" aria-label={busy ? "Pause" : "Play album"} onclick={() => start(open!.tracks)}>
                <Icon name={busy ? "pause" : "play"} size={18} />
              </button>
              <button class="pill" aria-label="Play album next" onclick={() => take(open!.tracks, "next")}><Icon name="playnext" size={18} /></button>
              <AddToggle variant="pill" on={queued(ids(open.tracks))} onclick={() => toggle(open!.tracks.map(snap))} />
              {#if isApp && station}<Keep {station} tracks={open.tracks} variant="pill" />{/if}
            </div>
          </div>
        </div>
        <Rows tracks={open.tracks} {station} art={false} numbered {onadd} {queued} {toggle} {nowId} {playing} {onplaypause} />
      </div>
    {:else if list}
      {@const lt = full(list.tracks)}
      {@const busy = live(lt) && playing}
      <!-- 手帳から　選びし束を　また鳴らす -->
      <div class="album" in:fly={{ x: 24, duration: 260 }}>
        <div class="hero">
          <span class="cover big" class:mosaic={lt.length >= 4}>
            {#each lt.slice(0, lt.length >= 4 ? 4 : 1) as t (t.id)}<span><Art {station} cover={t.cover} /></span>{/each}
          </span>
          <div class="info">
            <div class="name">{lt[0]?.title ?? ""}</div>
            <div class="by">{lt.length} · {stamp(list.at)}</div>
            <div class="acts">
              <button class="pill solid" aria-label={busy ? "Pause" : "Play list"} onclick={() => start(lt)}>
                <Icon name={busy ? "pause" : "play"} size={18} />
              </button>
              <AddToggle variant="pill" on={queued(ids(lt))} onclick={() => toggle(lt.map(snap))} />
              {#if isApp && station}<Keep {station} tracks={lt} variant="pill" />{/if}
              <button
                class="pill"
                aria-label="Delete list"
                onclick={() => {
                  dropPlaylist(list!.id);
                  list = null;
                }}><Icon name="trash" size={18} /></button
              >
            </div>
          </div>
        </div>
        <Rows tracks={lt} {station} {onadd} {queued} {toggle} {nowId} {playing} {onplaypause} />
      </div>
    {:else if mode === "songs"}
      <div in:fade={{ duration: 120 }}><Rows {tracks} {station} {onadd} {queued} {toggle} {nowId} {playing} {onplaypause} /></div>
    {:else if mode === "lists"}
      <div class="grid" in:fade={{ duration: 120 }}>
        {#each mine as p (p.id)}
          {@const lt = full(p.tracks)}
          <div class="card">
            <button class="face" onclick={() => (list = p)} aria-label={lt[0]?.title}>
              <span class="cover" class:mosaic={lt.length >= 4}>
                {#each lt.slice(0, lt.length >= 4 ? 4 : 1) as t (t.id)}<span><Art {station} cover={t.cover} /></span>{/each}
              </span>
              <span class="title ellipsis">{lt[0]?.title ?? ""}</span>
              <span class="sub ellipsis">{lt.length} · {stamp(p.at)}</span>
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <!-- 背表紙に　触れれば今を　鳴らす釦 -->
      <div class="grid" in:fade={{ duration: 120 }}>
        {#each shelf as a (a.key)}
          {@const on = queued(ids(a.tracks))}
          {@const busy = live(a.tracks) && playing}
          <div class="card">
            <button class="face" onclick={() => (open = a)} aria-label={a.title}>
              <span class="cover"><Art {station} cover={a.cover} /></span>
              <span class="title ellipsis">{a.title}</span>
              <span class="sub ellipsis">{a.artist}</span>
            </button>
            <span class="over">
              <span class="quick" class:on>
                <span class="reveal"><AddToggle variant="dot" size={16} {on} onclick={() => toggle(a.tracks.map(snap))} /></span>
                <button class="dot play" aria-label={busy ? "Pause" : "Play album"} onclick={() => start(a.tracks)}>
                  <Icon name={busy ? "pause" : "play"} size={18} />
                </button>
              </span>
            </span>
          </div>
        {/each}
      </div>
    {/if}
    <!-- 名を記し　権は手放す　と小さく -->
    <footer class="legal">
      Copyleft 🄯 2026
      <a href="https://github.com/Tchirek" target="_blank" rel="noopener" onclick={out}>tchirek afra<Icon name="out" size={11} stroke={2} /></a>, under
      <a href="https://www.gnu.org/licenses/agpl-3.0.html" target="_blank" rel="noopener" onclick={out}>AGPL-3.0<Icon name="out" size={11} stroke={2} /></a>
      (Code). All rights reversed.
    </footer>
  </div>
</div>

<style>
  .library {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-bottom: 14px;
  }
  .find {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 12px;
    border-radius: 11px;
    color: var(--fg-3);
    background: rgba(255, 255, 255, 0.08);
    transition: background 0.15s;
  }
  .find:focus-within {
    background: rgba(255, 255, 255, 0.13);
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
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--glass-2);
  }
  /* 小さき珠　滑りて今の　棚を指す */
  .seg {
    position: relative;
    flex: none;
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.06);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: var(--glass-2);
    transform: translateX(calc(var(--i) * 36px));
    transition: transform 0.32s var(--ease);
  }
  .seg button {
    position: relative;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    color: var(--fg-3);
    transition: color 0.2s;
  }
  .seg button.on {
    color: var(--fg);
  }
  .mosaic {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
  }
  .mosaic span,
  .cover > span {
    display: block;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  .legal {
    padding: 36px 4px 8px;
    font-size: 10.5px;
    line-height: 1.6;
    color: var(--fg-3);
    text-align: center;
    letter-spacing: 0.01em;
    user-select: text;
    -webkit-user-select: text;
  }
  .legal a {
    color: var(--fg-2);
    text-decoration: none;
    white-space: nowrap;
  }
  .legal a :global(svg) {
    display: inline-block;
    vertical-align: -1px;
    margin-left: 1px;
  }
  .legal a:hover {
    color: var(--fg);
  }
  .body {
    flex: 1;
    min-height: 0;
    padding-bottom: 40px;
    mask-image: linear-gradient(180deg, #000 calc(100% - 40px), transparent);
    -webkit-mask-image: linear-gradient(180deg, #000 calc(100% - 40px), transparent);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(140px, calc(50% - 8px)), 1fr));
    gap: 20px 16px;
  }
  .card {
    position: relative;
    min-width: 0;
  }
  .face {
    display: flex;
    flex-direction: column;
    width: 100%;
    min-width: 0;
    gap: 2px;
    text-align: left;
  }
  .cover {
    display: block;
    aspect-ratio: 1;
    width: 100%;
    margin-bottom: 8px;
    border-radius: 9px;
    overflow: hidden;
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.3);
    transition: transform 0.3s var(--spring);
  }
  .card:hover .cover {
    transform: translateY(-2px);
  }
  .face:active .cover {
    transform: scale(0.97);
  }
  .title {
    font-weight: 600;
    font-size: 14px;
  }
  .sub {
    font-size: 12.5px;
    color: var(--fg-2);
  }
  .over {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    aspect-ratio: 1;
    pointer-events: none;
  }
  /* 勾ひとつ　手を離せばこそ　釦の座へ */
  .quick {
    position: absolute;
    right: 9px;
    bottom: 9px;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .dot {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    color: #fff;
    background: rgba(20, 20, 24, 0.72);
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    opacity: 0;
    transform: translateY(6px);
    transition:
      opacity 0.15s,
      transform 0.25s var(--spring),
      background 0.15s;
    pointer-events: none;
  }
  .dot.play {
    width: 44px;
    height: 44px;
    color: #111;
    background: #fff;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
  }
  .reveal {
    display: grid;
    pointer-events: auto;
    opacity: 0;
    transform: translateY(6px);
    transition:
      opacity 0.15s,
      transform 0.32s var(--ease);
  }
  .quick.on .reveal {
    opacity: 1;
    transform: translateX(45px);
  }
  .card:hover .dot,
  .dot:focus-visible {
    opacity: 1;
    transform: none;
    pointer-events: auto;
  }
  .card:hover .reveal,
  .reveal:focus-within {
    opacity: 1;
    transform: none;
  }
  .dot:active {
    transform: scale(0.9);
  }
  @media (hover: none) {
    .reveal,
    .quick.on .reveal {
      opacity: 1;
      transform: none;
    }
    .dot.play {
      display: none;
    }
  }
  .album {
    padding-top: 4px;
  }
  .hero {
    display: flex;
    gap: 24px;
    align-items: flex-end;
    margin-bottom: 18px;
  }
  .cover.big {
    width: clamp(120px, 34%, 220px);
    flex: none;
    margin: 0;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.4);
  }
  .info {
    min-width: 0;
  }
  .name {
    font-size: clamp(20px, 2.4vw, 30px);
    font-weight: 800;
    line-height: 1.15;
  }
  .by {
    font-size: 16px;
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
    gap: 8px;
    margin-top: 14px;
  }
  .pill {
    display: grid;
    place-items: center;
    width: 52px;
    height: 36px;
    border-radius: 999px;
    background: var(--glass-2);
    transition:
      transform 0.25s var(--spring),
      background 0.15s;
  }
  .pill:hover {
    background: var(--glass-3);
  }
  .pill.solid {
    background: #fff;
    color: #111;
  }
  .pill:active {
    transform: scale(0.92);
  }
</style>
