<script lang="ts">
  import { flushSync, onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { anchor, backable } from "./lib/back.svelte";
  import { haptic } from "./lib/haptics";
  import { listenDeepLinks, offerApp } from "./lib/deeplink";
  import { isApp, openExternal } from "./lib/links";
  import { loadKept } from "./lib/offline.svelte";
  import { checkUpdate, type Update } from "./lib/update";
  import { openRoom, openStations, snap, Station, type Snap, type Track } from "./lib/api";
  import { blurs } from "./lib/art.svelte";
  import { derive, newSecret } from "./lib/crypto";
  import { frame, startFrames } from "./lib/frame.svelte";
  import { hash, parseTarget, roomLink, type Target } from "./lib/link";
  import { parseLrc, tongue, type Lyrics as LyricData } from "./lib/lrc";
  import { loadProfile } from "./lib/names";
  import { adoptPlaylist } from "./lib/playlists.svelte";
  import { sendLink } from "./lib/share";
  import { Player } from "./lib/player.svelte";
  import { Room, type Mode } from "./lib/room.svelte";
  import Art from "./ui/Art.svelte";
  import Backdrop from "./ui/Backdrop.svelte";
  import Chat from "./ui/Chat.svelte";
  import Cover from "./ui/Cover.svelte";
  import Gate from "./ui/Gate.svelte";
  import Icon from "./ui/Icon.svelte";
  import Library from "./ui/Library.svelte";
  import Lyrics from "./ui/Lyrics.svelte";
  import Members from "./ui/Members.svelte";
  import Progress from "./ui/Progress.svelte";
  import Queue from "./ui/Queue.svelte";
  import Transport from "./ui/Transport.svelte";
  import Volume from "./ui/Volume.svelte";
  import Whisper from "./ui/Whisper.svelte";

  type Panel = "lyrics" | "chat" | "queue" | "library";
  const DEFAULT_HUB = "https://retroflex.tchirek.top";

  const tauri = "__TAURI_INTERNALS__" in window;
  const mac = tauri && /Mac OS X/.test(navigator.userAgent) && navigator.maxTouchPoints === 0;
  const profile = loadProfile();

  // 中枢は　この頁の主か　覚えし名か
  function homeHub(): string {
    if (!tauri) return location.origin;
    try {
      return localStorage.getItem("retroflex.hub") ?? DEFAULT_HUB;
    } catch {
      return DEFAULT_HUB;
    }
  }

  // 歌の束　届けば駅の　棚で開く
  const first = parseTarget(location.hash, homeHub());
  const initial: Target = first.kind === "list" ? { kind: "station", hub: first.hub, station: first.station, key: "" } : first;
  let incoming = $state<{ station: string; ids: string[] } | null>(first.kind === "list" ? { station: first.station, ids: first.ids } : null);
  let focus = $state<{ id?: string; at: number; tracks: Snap[] } | null>(null);
  let target = $state<Target>(initial);
  let room = $state<Room | null>(null);
  let player = $state<Player | null>(null);
  let tracks = $state<Track[]>([]);
  let lyrics = $state<LyricData | null>(null);
  let panel = $state<Panel>(initial.kind === "station" ? "library" : "lyrics");
  let view = $state<"now" | Panel>(initial.kind === "station" ? "library" : "now");
  let probing = $state(initial.kind === "none");
  let update = $state<Update | null>(null);
  let wide = $state(true);
  let pending: { tracks: Snap[]; mode: Mode } | null = null;
  const lyricCache = new Map<string, LyricData | null>();

  // 一度得し　棚の鍵こそ　使い続けん
  let browsing = $state<{ hub: string; station: string; key: string } | null>(null);
  const made = new Map<string, Station>();
  function stationFor(hub: string, name: string, pass: string): Station {
    const id = `${hub}|${name}|${pass}`;
    let st = made.get(id);
    if (!st) made.set(id, (st = new Station(hub, name, pass)));
    return st;
  }
  $effect(() => {
    if (target.kind === "station") browsing = { hub: target.hub, station: target.station, key: target.key };
  });
  const station = $derived.by(() => {
    if (target.kind === "station") return stationFor(target.hub, target.station, target.key);
    if (target.kind === "room") {
      const b = browsing;
      const name = room?.station;
      if (b && b.hub === target.hub && (!name || b.station === name)) return stationFor(b.hub, b.station, b.key);
      if (name && room) return stationFor(target.hub, name, room.pass);
    }
    return null;
  });
  const current = $derived(room?.current);
  const pos = $derived(room ? room.position(frame.t + room.clock.offset) : 0);

  onMount(() => {
    startFrames();
    // 鍵盤の　出入りに合わせ　丈を変う
    const vv = window.visualViewport;
    const fit = () => {
      if (!vv) return;
      document.documentElement.style.setProperty("--vvh", `${Math.round(vv.height)}px`);
      if (vv.offsetTop > 0 || window.scrollY > 0) window.scrollTo(0, 0);
    };
    vv?.addEventListener("resize", fit);
    vv?.addEventListener("scroll", fit);
    fit();
    const mq = matchMedia("(min-width: 900px)");
    wide = mq.matches;
    mq.onchange = () => (wide = mq.matches);
    const onhash = () => {
      const t = parseTarget(location.hash, homeHub());
      if (hash(t) !== hash(target)) land(t);
    };
    if (initial.kind === "none") probe();
    if (isApp) {
      loadKept();
      listenDeepLinks((text) => onlink(text));
      checkUpdate(__APP_VERSION__).then((u) => (update = u));
    } else offerApp(first);
    addEventListener("hashchange", onhash);
    addEventListener("keydown", keys);
    return () => {
      vv?.removeEventListener("resize", fit);
      vv?.removeEventListener("scroll", fit);
      removeEventListener("hashchange", onhash);
      removeEventListener("keydown", keys);
    };
  });

  // 戸口にて　開いた駅が　あるか問う
  async function probe() {
    const hub = homeHub();
    const names = hub ? await openStations(hub) : [];
    if (names[0] && target.kind === "none") {
      target = { kind: "station", hub, station: names[0], key: "" };
      browse();
    }
    probing = false;
  }

  function browse() {
    panel = "library";
    view = "library";
  }

  // 部屋の札　変われば糸を　張り替える
  $effect(() => {
    const t = target;
    if (t.kind !== "room") return;
    const r = new Room(t.hub, t.room, t.secret, profile);
    const p = new Player(r, () => station);
    room = r;
    player = p;
    if (import.meta.env.DEV) Object.assign(window, { __rf: { room: r, player: p } });
    if (pending) {
      r.enqueue(pending.tracks, pending.mode);
      pending = null;
    }
    r.connect();
    return () => {
      p.destroy();
      r.close();
      room = null;
      player = null;
    };
  });

  // 駅が知れ　棚の目録　借りにゆく
  $effect(() => {
    const st = station;
    if (!st) return;
    let alive = true;
    st.library().then(
      (lib) => {
        if (!alive) return;
        Object.assign(blurs, lib.blurs);
        tracks = lib.tracks;
      },
      () => {},
    );
    return () => {
      alive = false;
    };
  });

  // 歌かわり　詞の紙をも　取り替える
  $effect(() => {
    const id = current?.id;
    const st = station;
    if (!id || !st) {
      lyrics = null;
      return;
    }
    if (lyricCache.has(id)) {
      lyrics = lyricCache.get(id) ?? null;
      return;
    }
    lyrics = null;
    let alive = true;
    st.lyrics(id).then((text) => {
      const parsed = text ? parseLrc(text) : null;
      lyricCache.set(id, parsed);
      if (alive) lyrics = parsed;
      // 異国の詞　母の言葉を　下に添う
      if (!parsed?.synced || parsed.lines.some((l) => l.sub)) return;
      const said = await st.translate(id, tongue());
      if (!said) return;
      const lines = parsed.lines.map((l) => (said[l.text] ? { ...l, sub: said[l.text] } : l));
      const done = { ...parsed, lines };
      lyricCache.set(id, done);
      if (alive) lyrics = done;
    });
    return () => {
      alive = false;
    };
  });

  // 散りし部屋　跡には戸口　ひとつだけ
  $effect(() => {
    if (room?.status === "gone") go({ kind: "none" });
  });

  $effect(() => {
    document.title = current ? `${current.title} · ${current.artist}` : "Retroflex";
  });

  function go(t: Target) {
    history.replaceState(null, "", hash(t) || location.pathname);
    target = t;
  }

  function onlink(text: string): boolean {
    const t = parseTarget(text, homeHub());
    if (t.kind === "none") return false;
    if (tauri) {
      try {
        localStorage.setItem("retroflex.hub", t.hub);
      } catch {}
    }
    land(t);
    return true;
  }

  // 行き先が　束なら棚を　開くだけ
  function land(t: Target) {
    if (t.kind === "list") {
      incoming = { station: t.station, ids: t.ids };
      browse();
      if (room && target.kind === "room" && target.hub === t.hub && room.station === t.station) return;
      return go({ kind: "station", hub: t.hub, station: t.station, key: "" });
    }
    if (t.kind === "station") browse();
    if (t.kind !== "none" || target.kind !== "none") go(t);
  }

  $effect(() => {
    const want = incoming;
    const st = station;
    if (!want || !st || st.name !== want.station || !tracks.length) return;
    const byId = new Map(tracks.map((t) => [t.id, t]));
    const list = want.ids.flatMap((id) => (byId.has(id) ? [snap(byId.get(id)!)] : []));
    incoming = null;
    if (!list.length) return;
    focus = isApp ? adoptPlaylist(st.name, list) : { at: Date.now(), tracks: list };
  });

  // 初めの歌　選べば部屋が　生まれけり
  async function add(list: Snap[], mode: Mode) {
    if (room) return room.enqueue(list, mode);
    if (target.kind !== "station") return;
    const secret = newSecret();
    try {
      const id = await openRoom(target.hub, target.station, target.key, derive(secret).ticket);
      pending = { tracks: list, mode };
      go({ kind: "room", hub: target.hub, room: id, secret });
    } catch {}
  }

  // 今鳴るも　次に控えも　列のうち
  const lineup = $derived.by(() => {
    if (!room) return new Set<string>();
    const at = Math.max(0, room.queue.findIndex((q) => q.qid === room!.cur));
    return new Set(room.queue.slice(room.cur == null ? room.queue.length : at).map((q) => q.id));
  });
  // 押した手の　思いが先に　形とる
  let guess = $state<{ key: string; on: boolean; at: number } | null>(null);

  function queued(ids: string[]): boolean {
    if (guess && guess.key === ids.join() && performance.now() - guess.at < 6000) return guess.on;
    return ids.length > 0 && ids.every((id) => lineup.has(id));
  }

  // 積むも良し　気が変わらば　また降ろす
  function toggle(list: Snap[]) {
    const ids = list.map((t) => t.id);
    const on = queued(ids);
    guess = { key: ids.join(), on: !on, at: performance.now() };
    if (!on) return void add(list, "end");
    if (!room) return;
    const drop = new Set(ids);
    const at = Math.max(0, room.queue.findIndex((q) => q.qid === room!.cur));
    for (const q of room.queue.slice(at)) if (drop.has(q.id)) room.remove(q.qid);
  }

  // 合言葉　紙に写して　手渡しに
  async function share(): Promise<boolean> {
    if (!room || target.kind !== "room") return false;
    return sendLink(roomLink(target.hub, room.id, room.secret));
  }

  function keys(e: KeyboardEvent) {
    const el = e.target as HTMLElement;
    if (!room || el.closest("input, textarea")) return;
    if (e.code === "Space") {
      e.preventDefault();
      if (player?.blocked) player.unlock();
      else if (room.playing) room.pause();
      else room.play();
    } else if (e.code === "ArrowRight" && e.shiftKey) room.next();
    else if (e.code === "ArrowLeft" && e.shiftKey) room.prev();
  }

  // 頁の順　左へ右へと　指が繰る
  type View = "now" | Panel;
  const ORDER: View[] = ["now", "lyrics", "chat", "queue", "library"];
  let slide = $state(0);

  function usable(v: View): boolean {
    if (v === "chat" || v === "queue") return !!room;
    if (v === "library") return !!station;
    return true;
  }

  function turn(next: View) {
    if (next === view || !usable(next)) return;
    slide = Math.sign(ORDER.indexOf(next) - ORDER.indexOf(view));
    haptic();
    const morph = next === "now" || view === "now";
    const vt = (document as Document & { startViewTransition?: (cb: () => void) => unknown }).startViewTransition;
    if (morph && vt) {
      vt.call(document, () => {
        view = next;
        flushSync();
      });
    } else view = next;
  }

  // 戻る手は　まず大きなる　盤の前へ
  anchor(() => hash(target) || location.pathname);

  backable(
    () => !wide && !!room && view !== "now",
    () => turn("now"),
  );

  function tab(p: Panel) {
    if (wide) {
      if (panel !== p) haptic();
      panel = p;
    } else turn(view === p ? "now" : p);
  }

  let touch: { x: number; y: number; t: number } | null = null;
  function swipeStart(e: TouchEvent) {
    const el = e.target as HTMLElement;
    touch = el.closest("input, textarea, .progress, .bar, .seg") ? null : { x: e.touches[0].clientX, y: e.touches[0].clientY, t: performance.now() };
  }
  function swipeEnd(e: TouchEvent) {
    if (!touch) return;
    const dx = e.changedTouches[0].clientX - touch.x;
    const dy = e.changedTouches[0].clientY - touch.y;
    const fast = performance.now() - touch.t < 700;
    touch = null;
    if (!fast || Math.abs(dx) < 60 || Math.abs(dx) < Math.abs(dy) * 1.6) return;
    const step = dx < 0 ? 1 : -1;
    let i = ORDER.indexOf(view) + step;
    while (i >= 0 && i < ORDER.length && !usable(ORDER[i])) i += step;
    if (i >= 0 && i < ORDER.length) turn(ORDER[i]);
  }

  const shown = $derived<Panel>(wide ? panel : view === "now" ? "lyrics" : view);
  const transport = $derived({
    playing: room?.playing ?? false,
    disabled: !current,
    onplay: () => (haptic(), player?.blocked ? player.unlock() : room?.play()),
    onpause: () => (haptic(), room?.pause()),
    onnext: () => (haptic(), room?.next()),
    onprev: () => (haptic(), room?.prev()),
  });
</script>

<Backdrop src={station?.cover(current?.cover)} />
{#if mac}<div class="drag" data-tauri-drag-region></div>{/if}

{#if target.kind === "none"}
  {#if !probing}<Gate {onlink} />{/if}
{:else if wide}
  <main class="wide" class:mac>
    <section class="stage">
      <div class="art"><Cover {station} cover={current?.cover} playing={room?.playing ?? false} /></div>
      <div class="meta">
        <div class="title ellipsis">{current?.title ?? ""}</div>
        <div class="artist ellipsis">{current?.artist ?? ""}</div>
      </div>
      <Progress time={pos} duration={current?.duration ?? 0} onseek={(t) => room?.seek(t)} />
      <Transport {...transport} />
      <div class="foot">
        {#if room}<Members {room} onshare={share} />{:else}<span></span>{/if}
        {#if player}<div class="vol"><Volume value={player.volume} onchange={(v) => player?.setVolume(v)} /></div>{/if}
      </div>
      {#if room}<div class="whispers">{#if shown !== "chat"}<Whisper {room} />{/if}</div>{/if}
    </section>
    <section class="side">
      {@render pane(shown)}
    </section>
    {@render tabs()}
  </main>
{:else}
  <main class="phone" class:full={view === "now"} ontouchstart={swipeStart} ontouchend={swipeEnd}>
    {#if view === "now"}
      <div class="art"><Cover {station} cover={current?.cover} playing={room?.playing ?? false} /></div>
      <div class="row">
        <div class="meta">
          <div class="title ellipsis">{current?.title ?? ""}</div>
          <div class="artist ellipsis">{current?.artist ?? ""}</div>
        </div>
        {#if room}<Members {room} onshare={share} />{/if}
      </div>
    {:else if view === "lyrics"}
      <header class="mini">
        <button class="thumb" aria-label="Now playing" onclick={() => turn("now")}><Art {station} cover={current?.cover} full eager /></button>
        <div class="meta">
          <div class="title ellipsis">{current?.title ?? ""}</div>
          <div class="artist ellipsis">{current?.artist ?? ""}</div>
        </div>
        {#if room}<Members {room} onshare={share} />{/if}
      </header>
      <section class="pane" in:fly={{ x: slide * 32, duration: 260, opacity: 0 }}>
        {@render pane(view)}
      </section>
    {:else}
      {#if view === "chat"}{@render dock()}{/if}
      <section class="pane tall" class:talk={view === "chat"} in:fly={{ x: slide * 32, duration: 260, opacity: 0 }}>
        {@render pane(view)}
      </section>
    {/if}
    {#if view === "now" || view === "lyrics"}
      {#if room}<div class="whispers"><Whisper {room} max={2} /></div>{/if}
      <Progress time={pos} duration={current?.duration ?? 0} onseek={(t) => room?.seek(t)} />
      <Transport {...transport} small={view !== "now"} />
    {:else if view !== "chat"}
      {@render dock()}
    {/if}
    {@render tabs()}
  </main>
{/if}

{#snippet dock()}
  <!-- 小さき帯　今鳴る歌を　手のひらに -->
  <div class="dock">
    <button class="who" aria-label="Now playing" onclick={() => turn("now")}>
      <span class="thumb small"><Art {station} cover={current?.cover} full eager /></span>
      <span class="meta">
        <span class="title ellipsis">{current?.title ?? ""}</span>
        <span class="artist ellipsis">{current?.artist ?? ""}</span>
      </span>
    </button>
    <button
      class="icon-btn"
      aria-label={transport.playing ? "Pause" : "Play"}
      disabled={transport.disabled}
      onclick={transport.playing ? transport.onpause : transport.onplay}
    >
      <Icon name={transport.playing ? "pause" : "play"} size={24} />
    </button>
    <button class="icon-btn" aria-label="Next" disabled={transport.disabled} onclick={transport.onnext}>
      <Icon name="next" size={22} />
    </button>
    <span class="line" style:transform="scaleX({current?.duration ? Math.min(1, pos / current.duration) : 0})"></span>
  </div>
{/snippet}

{#snippet pane(which: Panel | "now")}
  {#if which === "library"}
    <div class="panel">
      <Library
        {tracks}
        {station}
        onadd={add}
        {queued}
        {toggle}
        nowId={current?.id}
        playing={room?.playing ?? false}
        onplaypause={() => (room?.playing ? room.pause() : transport.onplay())}
        {focus}
      />
    </div>
  {:else if which === "chat" && room}
    <div class="panel"><Chat {room} /></div>
  {:else if which === "queue" && room}
    <div class="panel"><Queue {room} {station} /></div>
  {:else}
    <Lyrics {lyrics} time={pos} onseek={(t) => room?.seek(t)} />
  {/if}
{/snippet}

{#snippet tabs()}
  <nav class="tabs">
    <button class="icon-btn" class:on={shown === "lyrics" && (wide || view === "lyrics")} aria-label="Lyrics" onclick={() => tab("lyrics")}>
      <Icon name="lyrics" size={20} />
    </button>
    <button class="icon-btn" class:on={shown === "chat"} aria-label="Chat" onclick={() => tab("chat")} disabled={!room}>
      <Icon name="chat" size={20} />
    </button>
    <button class="icon-btn" class:on={shown === "queue"} aria-label="Queue" onclick={() => tab("queue")} disabled={!room}>
      <Icon name="queue" size={20} />
    </button>
    <button class="icon-btn" class:on={shown === "library"} aria-label="Library" onclick={() => tab("library")} disabled={!station}>
      <Icon name="library" size={20} />
    </button>
  </nav>
{/snippet}

{#if update}
  <!-- 新しき　版あり釦　ひとつだけ -->
  <button
    class="update"
    aria-label="Update"
    transition:fade={{ duration: 200 }}
    onclick={() => {
      openExternal(update!.url);
      update = null;
    }}><Icon name="download" size={15} stroke={2} />{update.version}</button
  >
{/if}

{#if player?.blocked}
  <button class="unlock" aria-label="Listen" onclick={() => player?.unlock()} transition:fade={{ duration: 250 }}>
    <span class="ring"><Icon name="headphones" size={56} stroke={1.6} /></span>
  </button>
{/if}

<style>
  .drag {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 30px;
    z-index: 50;
  }
  main {
    position: relative;
    z-index: 1;
    height: 100%;
  }

  /* 左に盤　右に詞の流れ　見開きに */
  .wide {
    display: grid;
    grid-template-columns: minmax(320px, 0.82fr) 1.18fr;
    gap: clamp(32px, 5vw, 96px);
    padding: calc(var(--top) + 40px) clamp(32px, 6vw, 110px) calc(var(--bottom) + 36px);
  }
  .wide.mac {
    padding-top: 56px;
  }
  .stage {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 18px;
    min-height: 0;
    max-width: 520px;
    width: 100%;
    justify-self: end;
  }
  .art {
    width: 100%;
    max-width: min(100%, 56vh);
    align-self: center;
  }
  .meta {
    min-width: 0;
  }
  .title {
    font-size: clamp(19px, 1.7vw, 24px);
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .artist {
    font-size: clamp(15px, 1.3vw, 19px);
    color: var(--fg-2);
    margin-top: 2px;
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-top: 4px;
  }
  .vol {
    width: 150px;
  }
  .side {
    min-height: 0;
    min-width: 0;
    padding-top: 6vh;
  }
  .panel {
    height: 100%;
    max-width: 720px;
  }
  .tabs {
    position: absolute;
    top: calc(var(--top) + 16px);
    right: clamp(18px, 3vw, 40px);
    display: flex;
    gap: 6px;
  }
  .wide.mac .tabs {
    top: 12px;
  }
  .tabs button:disabled {
    opacity: 0.3;
    pointer-events: none;
  }
  /* 耳打ちの　席を空けおき　揺らぎなし */
  .whispers {
    height: 112px;
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
    overflow: hidden;
  }

  /* 掌に　縦一列の　小舞台 */
  .phone {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: calc(var(--top) + 18px) 22px calc(var(--bottom) + 10px);
    --lyric-size: 26px;
  }
  .phone .art {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    max-width: none;
  }
  .phone .art :global(.cover) {
    width: min(100%, 52vh);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .row .meta,
  .mini .meta {
    flex: 1;
  }
  .mini {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .thumb {
    width: 54px;
    height: 54px;
    flex: none;
    border-radius: 8px;
    overflow: hidden;
    background: var(--glass);
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.3);
  }
  .mini .title {
    font-size: 16px;
  }
  .mini .artist {
    font-size: 14px;
  }
  .phone .pane {
    flex: 1;
    min-height: 0;
  }
  .phone .panel {
    max-width: none;
  }
  /* 盤ひとつ　小さき窓から　大きな窓へ */
  .phone .art :global(.cover),
  .phone .thumb {
    view-transition-name: cover;
  }
  .mini .thumb {
    padding: 0;
    cursor: pointer;
  }
  .phone .pane.talk {
    padding-bottom: 10px;
  }
  .dock {
    position: relative;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 7px 6px 7px 7px;
    border-radius: 14px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.09);
    flex: none;
  }
  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 11px;
    text-align: left;
  }
  .thumb.small {
    width: 44px;
    height: 44px;
    border-radius: 7px;
    box-shadow: none;
  }
  .who .meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .who .title {
    font-size: 14.5px;
    font-weight: 600;
  }
  .who .artist {
    font-size: 12.5px;
    margin-top: 0;
  }
  .dock .icon-btn {
    width: 42px;
    height: 42px;
    color: var(--fg);
  }
  .dock .icon-btn:disabled {
    opacity: 0.35;
  }
  .line {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: rgba(255, 255, 255, 0.7);
    transform-origin: 0 50%;
  }
  .phone .tabs {
    position: static;
    justify-content: space-around;
  }
  .phone .whispers {
    height: 72px;
    justify-content: flex-end;
    flex: none;
  }

  .update {
    position: fixed;
    top: calc(var(--top) + 10px);
    left: 50%;
    z-index: 45;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px 0 10px;
    transform: translateX(-50%);
    border-radius: 999px;
    font-size: 12.5px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: #111;
    background: rgba(255, 255, 255, 0.92);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  }

  .unlock {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    background: rgba(8, 8, 12, 0.45);
    backdrop-filter: blur(14px);
    -webkit-backdrop-filter: blur(14px);
  }
  .ring {
    display: grid;
    place-items: center;
    width: 132px;
    height: 132px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.12);
    box-shadow: 0 0 0 0 rgba(255, 255, 255, 0.35);
    animation: call 2s var(--ease) infinite;
  }
  @keyframes call {
    70% {
      box-shadow: 0 0 0 28px rgba(255, 255, 255, 0);
    }
    100% {
      box-shadow: 0 0 0 0 rgba(255, 255, 255, 0);
    }
  }
</style>
