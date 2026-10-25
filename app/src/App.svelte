<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { openRoom, openStations, Station, type Snap, type Track } from "./lib/api";
  import { blurs } from "./lib/art.svelte";
  import { derive, newSecret } from "./lib/crypto";
  import { frame, startFrames } from "./lib/frame.svelte";
  import { hash, parseTarget, roomLink, type Target } from "./lib/link";
  import { parseLrc, type Lyrics as LyricData } from "./lib/lrc";
  import { loadProfile } from "./lib/names";
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

  const initial = parseTarget(location.hash, homeHub());
  let target = $state<Target>(initial);
  let room = $state<Room | null>(null);
  let player = $state<Player | null>(null);
  let tracks = $state<Track[]>([]);
  let lyrics = $state<LyricData | null>(null);
  let panel = $state<Panel>(initial.kind === "station" ? "library" : "lyrics");
  let view = $state<"now" | Panel>(initial.kind === "station" ? "library" : "now");
  let probing = $state(initial.kind === "none");
  let wide = $state(true);
  let pending: { tracks: Snap[]; mode: Mode } | null = null;
  const lyricCache = new Map<string, LyricData | null>();

  const station = $derived.by(() => {
    if (target.kind === "station") return new Station(target.hub, target.station, target.key);
    if (target.kind === "room" && room?.station) return new Station(target.hub, room.station, room.pass);
    return null;
  });
  const current = $derived(room?.current);
  const pos = $derived(room ? room.position(frame.t + room.clock.offset) : 0);

  onMount(() => {
    startFrames();
    const mq = matchMedia("(min-width: 900px)");
    wide = mq.matches;
    mq.onchange = () => (wide = mq.matches);
    const onhash = () => {
      target = parseTarget(location.hash, homeHub());
      if (target.kind === "station") browse();
    };
    if (initial.kind === "none") probe();
    addEventListener("hashchange", onhash);
    addEventListener("keydown", keys);
    return () => {
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
    if (t.kind === "station") browse();
    go(t);
    return true;
  }

  // 初めの歌　選べば部屋が　生まれけり
  async function add(list: Snap[], mode: Mode) {
    if (mode === "now" && !wide) view = "now";
    if (room) return room.enqueue(list, mode);
    if (target.kind !== "station") return;
    const secret = newSecret();
    try {
      const id = await openRoom(target.hub, target.station, target.key, derive(secret).ticket);
      pending = { tracks: list, mode };
      go({ kind: "room", hub: target.hub, room: id, secret });
    } catch {}
  }

  // 合言葉　紙に写して　手渡しに
  async function share(): Promise<boolean> {
    if (!room || target.kind !== "room") return false;
    const link = roomLink(target.hub, room.id, room.secret);
    if (navigator.share && matchMedia("(pointer: coarse)").matches) {
      try {
        await navigator.share({ url: link });
        return true;
      } catch {
        return false;
      }
    }
    try {
      await navigator.clipboard.writeText(link);
      return true;
    } catch {
      const area = Object.assign(document.createElement("textarea"), { value: link });
      document.body.append(area);
      area.select();
      const ok = document.execCommand("copy");
      area.remove();
      return ok;
    }
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

  function tab(p: Panel) {
    if (wide) panel = p;
    else view = view === p ? "now" : p;
  }

  const shown = $derived<Panel>(wide ? panel : view === "now" ? "lyrics" : view);
  const transport = $derived({
    playing: room?.playing ?? false,
    disabled: !current,
    onplay: () => (player?.blocked ? player.unlock() : room?.play()),
    onpause: () => room?.pause(),
    onnext: () => room?.next(),
    onprev: () => room?.prev(),
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
  <main class="phone" class:full={view === "now"}>
    {#if view === "now"}
      <div class="art"><Cover {station} cover={current?.cover} playing={room?.playing ?? false} /></div>
      <div class="row">
        <div class="meta">
          <div class="title ellipsis">{current?.title ?? ""}</div>
          <div class="artist ellipsis">{current?.artist ?? ""}</div>
        </div>
        {#if room}<Members {room} onshare={share} />{/if}
      </div>
    {:else}
      <header class="mini">
        <span class="thumb"><Art {station} cover={current?.cover} /></span>
        <div class="meta">
          <div class="title ellipsis">{current?.title ?? ""}</div>
          <div class="artist ellipsis">{current?.artist ?? ""}</div>
        </div>
        {#if room}<Members {room} onshare={share} />{/if}
      </header>
      <section class="pane" in:fade={{ duration: 160 }}>
        {@render pane(view)}
      </section>
    {/if}
    {#if room && view !== "chat"}<div class="whispers"><Whisper {room} max={2} /></div>{/if}
    <Progress time={pos} duration={current?.duration ?? 0} onseek={(t) => room?.seek(t)} />
    <Transport {...transport} small={view !== "now"} />
    {@render tabs()}
  </main>
{/if}

{#snippet pane(which: Panel | "now")}
  {#if which === "library"}
    <div class="panel"><Library {tracks} {station} onadd={add} /></div>
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
  .phone .tabs {
    position: static;
    justify-content: space-around;
  }
  .phone .whispers {
    height: 72px;
    justify-content: flex-end;
    flex: none;
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
