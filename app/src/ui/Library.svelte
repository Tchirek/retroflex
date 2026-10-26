<script lang="ts">
  import { untrack, type Snippet } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { fade, fly } from "svelte/transition";
  import { snap, type Snap, type Station, type Track } from "../lib/api";
  import { backable } from "../lib/back.svelte";
  import { haptic } from "../lib/haptics";
  import { albums, search, type Album, type Look } from "../lib/library";
  import { listLink } from "../lib/link";
  import { isApp, openExternal } from "../lib/links";
  import { keep } from "../lib/offline.svelte";
  import { press } from "../lib/press";
  import { dropPlaylists, playlists } from "../lib/playlists.svelte";
  import type { Mode } from "../lib/room.svelte";
  import { scrub } from "../lib/scrub";
  import { sendLink } from "../lib/share";
  import AddToggle from "./AddToggle.svelte";
  import Art from "./Art.svelte";
  import Confirm from "./Confirm.svelte";
  import Icon from "./Icon.svelte";
  import Keep from "./Keep.svelte";
  import Picks from "./Picks.svelte";
  import Rows from "./Rows.svelte";
  import Tick from "./Tick.svelte";

  type Sheet = { id?: string; at: number; tracks: Snap[] };

  let {
    tracks,
    station,
    onadd,
    queued,
    toggle,
    nowId,
    playing,
    onplaypause,
    focus = null,
    look = null,
    onlook,
    lounge,
  }: {
    tracks: Track[];
    station: Station | null;
    onadd: (t: Snap[], mode: Mode) => void;
    queued: (ids: string[]) => boolean;
    toggle: (t: Snap[]) => void;
    nowId?: string;
    playing: boolean;
    onplaypause: () => void;
    focus?: Sheet | null;
    look?: Look | null;
    onlook?: () => void;
    lounge?: Snippet;
  } = $props();

  const ids = (list: Track[]) => list.map((t) => t.id);
  const live = (list: Track[]) => !!nowId && list.some((t) => t.id === nowId);

  // 鳴る盤は　止めも出来るぞ　同じ釦
  function start(list: Track[]) {
    if (live(list)) onplaypause();
    else take(list, "now");
  }

  type Shelf = "albums" | "songs" | "lists";
  const SHELVES: Shelf[] = isApp ? ["albums", "songs", "lists"] : ["albums", "songs"];
  let query = $state("");
  let open = $state<Album | null>(null);
  let list = $state<Sheet | null>(null);
  let artist = $state<string | null>(null);
  let mode = $state<Shelf>("albums");
  let shared = $state(false);
  let segScrub = $state(false);
  const shelf = $derived(albums(tracks));
  const found = $derived(search(tracks, query));
  const byId = $derived(new Map(tracks.map((t) => [t.id, t])));
  const mine = $derived(playlists.filter((p) => !station || p.station === station.name));
  const artistAlbums = $derived(
    artist ? shelf.filter((a) => a.artist === artist || a.tracks.some((t) => t.artist === artist || t.album_artist === artist)) : [],
  );
  const artistTracks = $derived(artist ? tracks.filter((t) => t.artist === artist || t.album_artist === artist) : []);

  const FIRST = 48;
  const STEP = 24;
  let cards = $state(FIRST);
  const shelfShown = $derived(shelf.slice(0, cards));
  const mineShown = $derived(mine.slice(0, cards));

  // 棚の先　少し開きて　歌を待つ
  function grow(node: HTMLElement) {
    if (!("IntersectionObserver" in window)) {
      cards = Math.max(shelf.length, mine.length);
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
          const total = mode === "lists" ? mine.length : shelf.length;
          if (cards < total) cards = Math.min(total, cards + STEP);
          requestAnimationFrame(() => (io.unobserve(node), io.observe(node)));
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

  // 棚を替え　窓の先から　また数う
  $effect(() => {
    void mode;
    cards = FIRST;
  });

  // 届いた束　そのまま開いて　見せにゆく
  $effect(() => {
    const f = focus;
    if (!f) return;
    untrack(() => {
      query = "";
      open = null;
      artist = null;
      if (f.id) mode = "lists";
      list = f;
    });
  });

  // 指の名は　窓の行先　開きけり
  $effect(() => {
    const want = look;
    if (!want) return;
    if (want.kind === "artist") {
      const known = tracks.some((t) => t.artist === want.name || t.album_artist === want.name);
      if (!known) {
        if (tracks.length) onlook?.();
        return;
      }
      untrack(() => {
        query = "";
        open = null;
        list = null;
        artist = want.name;
        mode = "albums";
        onlook?.();
      });
      return;
    }
    const album = shelf.find((a) => a.tracks.some((t) => t.id === want.id));
    if (!album) {
      if (tracks.length) onlook?.();
      return;
    }
    untrack(() => {
      query = "";
      list = null;
      artist = null;
      open = album;
      mode = "albums";
      onlook?.();
    });
  });

  // 表紙には　重なる歌を　一度だけ
  function faces(lt: Track[]): Track[] {
    const one = [...new Map(lt.map((t) => [t.id, t])).values()];
    return one.slice(0, one.length >= 4 ? 4 : 1);
  }

  // 札だけの　歌も棚から　身を借りる
  function full(s: Snap[]): Track[] {
    return s.map((t) => byId.get(t.id) ?? { ...t, mime: "", size: 0, lyrics: false });
  }

  function shelve(m: Shelf) {
    if (m !== mode) haptic();
    mode = m;
    open = null;
    list = null;
    artist = null;
  }

  // 窓ひとつ　戻る手順を　指に聞く
  function backOne() {
    if (open) open = null;
    else if (list) list = null;
    else artist = null;
  }

  // 盤の名を　同じ名の棚　繋ぎけり
  function seeArtist() {
    if (!open) return;
    artist = open.artist;
    open = null;
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

  async function share(lt: Track[]) {
    if (!station || !(await sendLink(listLink(station.hub, station.name, ids(lt))))) return;
    haptic();
    shared = true;
    setTimeout(() => (shared = false), 1600);
  }

  // 長押しで　選ぶ印を　ひとつずつ
  const picked = new SvelteSet<string>();
  let picking = $state(false);
  const scope = $derived(query ? "found" : open ? "album" : list ? "list" : artist ? "artist" : mode);

  function begin(key: string) {
    picking = true;
    picked.add(key);
  }
  function flip(key: string) {
    haptic();
    if (picked.has(key)) picked.delete(key);
    else picked.add(key);
    if (!picked.size) leave();
  }
  function leave() {
    picking = false;
    picked.clear();
  }
  $effect(() => {
    void scope;
    void open;
    void list;
    void artist;
    untrack(leave);
  });

  backable(() => picking, leave);
  backable(() => !!artist, () => (artist = null));
  backable(() => !!list, () => (list = null));
  backable(() => !!open, () => (open = null));

  const pool = $derived.by((): Track[] => {
    if (scope === "found") return found;
    if (scope === "album") return open?.tracks ?? [];
    if (scope === "list") return full(list?.tracks ?? []);
    if (scope === "artist") return artistTracks;
    return tracks;
  });
  const chosen = $derived.by((): Track[] => {
    if (!picking) return [];
    if (scope === "albums") return shelf.filter((a) => picked.has(a.key)).flatMap((a) => a.tracks);
    if (scope === "lists") return mine.filter((p) => picked.has(p.id)).flatMap((p) => full(p.tracks));
    return pool.filter((t) => picked.has(t.id));
  });

  function queueChosen() {
    take(chosen, "end");
    haptic();
    leave();
  }
  function keepChosen() {
    if (station) keep(station, chosen);
    haptic();
    leave();
  }
  function dropChosen() {
    dropPlaylists(picked);
    leave();
  }

  const rowsPick = $derived(picking ? picked : null);
  const pick = (id: string) => (picking ? flip(id) : begin(id));
</script>

<div class="library">
  <div class="head">
    {#if (open || list || artist) && !query}
      <button class="icon-btn" aria-label="Back" onclick={backOne}><Icon name="back" /></button>
    {/if}
    <label class="find">
      <Icon name="search" size={17} />
      <input bind:value={query} aria-label="Search" spellcheck="false" />
      {#if query}<button class="clear" aria-label="Clear" onclick={() => (query = "")}><Icon name="close" size={13} /></button>{/if}
    </label>
    {@render lounge?.()}
    <div
      class="seg"
      class:scrubbing={segScrub}
      style:--i={SHELVES.indexOf(mode)}
      use:scrub={{
        onstart: () => (segScrub = true),
        onend: () => (segScrub = false),
        onpick: (el) => {
          const next = el.dataset.mode as Shelf | undefined;
          if (next && next !== mode) shelve(next);
        },
      }}
    >
      <span class="knob"></span>
      <button class:on={mode === "albums"} aria-label="Albums" data-choice data-mode="albums" onclick={() => shelve("albums")}>
        <Icon name="library" size={18} />
      </button>
      <button class:on={mode === "songs"} aria-label="Songs" data-choice data-mode="songs" onclick={() => shelve("songs")}>
        <Icon name="songs" size={18} />
      </button>
      {#if isApp}<button class:on={mode === "lists"} aria-label="Saved lists" data-choice data-mode="lists" onclick={() => shelve("lists")}><Icon name="lists" size={18} /></button>{/if}
    </div>
  </div>

  <div class="viewport">
    <div class="body scroll">
    {#if query}
      <div in:fade={{ duration: 120 }}>
        <Rows tracks={found} {station} {onadd} {queued} {toggle} {nowId} {playing} {onplaypause} picked={rowsPick} onpick={pick} />
      </div>
    {:else if open}
      {@const busy = live(open.tracks) && playing}
      <!-- 盤を開き　針を落とすも　積むも良し -->
      <div class="album" in:fly={{ x: 24, duration: 260 }}>
        <div class="hero">
          <span class="cover big"><Art {station} cover={open.cover} full eager /></span>
          <div class="info">
            <div class="name">{open.title}</div>
            <button class="by link" onclick={seeArtist}>{open.artist}</button>
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
        <Rows tracks={open.tracks} {station} art={false} numbered {onadd} {queued} {toggle} {nowId} {playing} {onplaypause} picked={rowsPick} onpick={pick} />
      </div>
    {:else if artist}
      {@const at = artistTracks}
      {@const busy = live(at) && playing}
      <!-- 同じ名の　歌をひと棚　集めけり -->
      <div class="album artist-view" in:fly={{ x: 24, duration: 260 }}>
        <div class="artist-head">
          <span class="artist-face"><Art {station} cover={artistAlbums[0]?.cover ?? at[0]?.cover} full eager /></span>
          <div class="info">
            <div class="name">{artist}</div>
            <div class="by">{artistAlbums.length} · {at.length}</div>
            <div class="acts">
              <button class="pill solid" aria-label={busy ? "Pause" : "Play artist"} onclick={() => start(at)}>
                <Icon name={busy ? "pause" : "play"} size={18} />
              </button>
              <button class="pill" aria-label="Play artist next" onclick={() => take(at, "next")}><Icon name="playnext" size={18} /></button>
              <AddToggle variant="pill" on={queued(ids(at))} onclick={() => toggle(at.map(snap))} />
            </div>
          </div>
        </div>
        {#if artistAlbums.length}
          <div class="grid artist-shelf">
            {#each artistAlbums as a (a.key)}
              <div class="card">
                <button class="face" onclick={() => (open = a)} aria-label={a.title}>
                  <span class="cover"><Art {station} cover={a.cover} /></span>
                  <span class="title ellipsis">{a.title}</span>
                  <span class="sub ellipsis">{a.year ?? a.artist}</span>
                </button>
              </div>
            {/each}
          </div>
        {/if}
        <Rows tracks={at} {station} {onadd} {queued} {toggle} {nowId} {playing} {onplaypause} picked={rowsPick} onpick={pick} />
      </div>
    {:else if list}
      {@const lt = full(list.tracks)}
      {@const busy = live(lt) && playing}
      {@const saved = list.id}
      <!-- 手帳から　選びし束を　また鳴らす -->
      <div class="album" in:fly={{ x: 24, duration: 260 }}>
        <div class="hero">
          <span class="cover big" class:mosaic={faces(lt).length >= 4}>
            {#each faces(lt) as t (t.id)}<span><Art {station} cover={t.cover} /></span>{/each}
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
              <button class="pill" aria-label="Share list" onclick={() => share(lt)}>
                <Icon name={shared ? "check" : "share"} size={18} />
              </button>
              {#if isApp && saved}
                <Confirm
                  variant="pill"
                  label="Delete list"
                  onconfirm={() => {
                    dropPlaylists([saved]);
                    list = null;
                  }}
                />
              {/if}
            </div>
          </div>
        </div>
        <Rows tracks={lt} {station} {onadd} {queued} {toggle} {nowId} {playing} {onplaypause} picked={rowsPick} onpick={pick} />
      </div>
    {:else if mode === "songs"}
      <div in:fade={{ duration: 120 }}>
        <Rows {tracks} {station} {onadd} {queued} {toggle} {nowId} {playing} {onplaypause} picked={rowsPick} onpick={pick} />
      </div>
    {:else if mode === "lists"}
      <div class="grid" in:fade={{ duration: 120 }}>
        {#each mineShown as p (p.id)}
          {@const lt = full(p.tracks)}
          <div class="card" class:sel={picked.has(p.id)}>
            <button class="face" use:press={() => begin(p.id)} onclick={() => (picking ? flip(p.id) : (list = p))} aria-label={lt[0]?.title}>
              <span class="cover" class:mosaic={faces(lt).length >= 4}>
                {#each faces(lt) as t (t.id)}<span><Art {station} cover={t.cover} /></span>{/each}
              </span>
              <span class="title ellipsis">{lt[0]?.title ?? ""}</span>
              <span class="sub ellipsis">{lt.length} · {stamp(p.at)}</span>
            </button>
            {#if picking}<span class="mark"><Tick on={picked.has(p.id)} /></span>{/if}
          </div>
        {/each}
      </div>
      {#if mine.length > cards}<div class="more" use:grow aria-hidden="true"></div>{/if}
    {:else}
      <!-- 背表紙に　触れれば今を　鳴らす釦 -->
      <div class="grid" in:fade={{ duration: 120 }}>
        {#each shelfShown as a (a.key)}
          {@const on = queued(ids(a.tracks))}
          {@const busy = live(a.tracks) && playing}
          <div class="card" class:sel={picked.has(a.key)}>
            <button class="face" use:press={() => begin(a.key)} onclick={() => (picking ? flip(a.key) : (open = a))} aria-label={a.title}>
              <span class="cover"><Art {station} cover={a.cover} /></span>
              <span class="title ellipsis">{a.title}</span>
              <span class="sub ellipsis">{a.artist}</span>
            </button>
            {#if picking}
              <span class="mark"><Tick on={picked.has(a.key)} /></span>
            {:else}
              <span class="over">
                <span class="quick" class:on>
                  <span class="reveal"><AddToggle variant="dot" size={16} {on} onclick={() => toggle(a.tracks.map(snap))} /></span>
                  <button class="dot play" aria-label={busy ? "Pause" : "Play album"} onclick={() => start(a.tracks)}>
                    <Icon name={busy ? "pause" : "play"} size={18} />
                  </button>
                </span>
              </span>
            {/if}
          </div>
        {/each}
      </div>
      {#if shelf.length > cards}<div class="more" use:grow aria-hidden="true"></div>{/if}
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

  {#if picking}
    <Picks count={picked.size} onclose={leave}>
      <button class="icon-btn" aria-label="Add to queue" onclick={queueChosen}><Icon name="plus" size={19} /></button>
      {#if isApp && station}<button class="icon-btn" aria-label="Download" onclick={keepChosen}><Icon name="download" size={19} /></button>{/if}
      {#if scope === "lists"}<Confirm label="Delete lists" onconfirm={dropChosen} size={19} />{/if}
    </Picks>
  {/if}
</div>

<style>
  .library {
    position: relative;
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
    touch-action: none;
  }
  .seg.scrubbing .knob {
    transform: translateX(calc(var(--i) * 36px)) scale(1.12);
    box-shadow: 0 0 18px rgba(255, 255, 255, 0.16);
  }
  .seg.scrubbing button {
    color: var(--fg-2);
    transform: scale(0.94);
  }
  .seg.scrubbing button.on {
    color: var(--fg);
    transform: scale(1.06);
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
  /* 深き棚　霞は窓の　縁に置く */
  .viewport {
    position: relative;
    flex: 1;
    min-height: 0;
    mask-image: linear-gradient(180deg, #000 calc(100% - 40px), transparent);
    -webkit-mask-image: linear-gradient(180deg, #000 calc(100% - 40px), transparent);
  }
  .body {
    height: 100%;
    padding-bottom: 40px;
  }
  .more {
    height: 1px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(140px, calc(50% - 8px)), 1fr));
    gap: 20px 16px;
  }
  /* 見えぬ棚　絵をば描かず　息を継ぐ */
  .card {
    position: relative;
    min-width: 0;
    content-visibility: auto;
    contain-intrinsic-size: auto 230px;
  }
  .face {
    display: flex;
    flex-direction: column;
    width: 100%;
    min-width: 0;
    gap: 2px;
    text-align: left;
    -webkit-touch-callout: none;
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
  @media (hover: hover) {
    .card:hover .cover {
      transform: translateY(-2px);
    }
  }
  .face:active .cover {
    transform: scale(0.97);
  }
  /* 選ばれし　盤はわずかに　身を引きて */
  .card.sel .cover {
    transform: scale(0.94);
  }
  .mark {
    position: absolute;
    top: 8px;
    left: 8px;
    pointer-events: none;
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
  .by.link {
    display: block;
    text-align: left;
    cursor: pointer;
    transition: color 0.18s var(--ease);
  }
  .by.link:hover {
    color: var(--fg);
  }
  .artist-shelf {
    margin-bottom: 20px;
  }
  /* 丸き窓　同じ歌い手の　顔となる */
  .artist-head {
    display: flex;
    align-items: center;
    gap: 16px;
    margin: 2px 0 20px;
  }
  .artist-face {
    width: 76px;
    height: 76px;
    flex: none;
    border-radius: 50%;
    overflow: hidden;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.35);
  }
  .artist-head .info {
    flex: 1;
  }
  .artist-head .acts {
    margin-top: 10px;
  }
  .year {
    font-size: 13px;
    color: var(--fg-3);
    margin-top: 2px;
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
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
  @media (hover: hover) {
    .pill:hover {
      background: var(--glass-3);
    }
  }
  .pill.solid {
    background: #fff;
    color: #111;
  }
  .pill:active {
    transform: scale(0.92);
  }
</style>
