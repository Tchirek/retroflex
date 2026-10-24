<script lang="ts">
  import { activeLine, type LyricLine, type Lyrics } from "../lib/lrc";
  import Icon from "./Icon.svelte";

  let { lyrics, time, onseek }: { lyrics: Lyrics | null; time: number; onseek: (t: number) => void } = $props();

  let height = $state(0);
  let els: HTMLElement[] = $state([]);
  let tops = $state<number[]>([]);
  let manual = $state<number | null>(null);
  let idle = 0;

  // 前奏の　静けさにまた　点三つ
  const lines = $derived.by((): LyricLine[] => {
    if (!lyrics?.synced) return lyrics?.lines ?? [];
    const first = lyrics.lines[0];
    return first && first.t > 4 ? [{ t: 0, text: "" }, ...lyrics.lines] : lyrics.lines;
  });
  const active = $derived(lyrics?.synced ? activeLine(lines, time + 0.3) : -1);
  const shift = $derived(manual ?? height * 0.32 - (tops[Math.max(0, active)] ?? 0));

  // 行ごとの　高さを測り　位置を知る
  $effect(() => {
    void lines;
    void height;
    queueMicrotask(() => (tops = els.slice(0, lines.length).map((e) => e?.offsetTop ?? 0)));
  });

  // 指で繰る　しばしの後は　また歌へ
  function wheel(e: WheelEvent) {
    if (!lyrics?.synced) return;
    manual = (manual ?? shift) - e.deltaY;
    clearTimeout(idle);
    idle = window.setTimeout(() => (manual = null), 2600);
  }

  let touchY = 0;
  function touchstart(e: TouchEvent) {
    touchY = e.touches[0].clientY;
  }
  function touchmove(e: TouchEvent) {
    if (!lyrics?.synced) return;
    const y = e.touches[0].clientY;
    manual = (manual ?? shift) + (y - touchY);
    touchY = y;
    clearTimeout(idle);
    idle = window.setTimeout(() => (manual = null), 2600);
  }
</script>

<div
  class="lyrics"
  role="region"
  aria-label="Lyrics"
  class:plain={lyrics && !lyrics.synced}
  class:scroll={lyrics && !lyrics.synced}
  bind:clientHeight={height}
  onwheel={wheel}
  ontouchstart={touchstart}
  ontouchmove={touchmove}
>
  {#if !lyrics || lines.length === 0}
    <div class="none"><Icon name="hook" size={56} stroke={1.4} /></div>
  {:else}
    <div class="sheet" class:free={manual !== null}>
      {#each lines as line, i (i)}
        {@const d = i - Math.max(0, active)}
        <button
          bind:this={els[i]}
          class="line"
          class:on={i === active}
          class:past={lyrics.synced && i < active}
          class:gap={!line.text}
          style:transform="translateY({lyrics.synced ? shift : 0}px) scale({i === active || !lyrics.synced ? 1 : 0.96})"
          style:filter={lyrics.synced && manual === null && i !== active ? `blur(${Math.min(Math.abs(d), 6) * 0.55}px)` : "none"}
          style:transition-delay="{manual === null && d > 0 ? Math.min(d, 8) * 42 : 0}ms"
          onclick={() => lyrics.synced && onseek(line.t)}
        >
          {#if line.text}
            <span class="main">{line.text}</span>
            {#if line.sub}<span class="sub">{line.sub}</span>{/if}
          {:else}
            <span class="dots" class:live={i === active}><i></i><i></i><i></i></span>
          {/if}
        </button>
      {/each}
      <div class="tail"></div>
    </div>
  {/if}
</div>

<style>
  .lyrics {
    position: relative;
    height: 100%;
    overflow: hidden;
    mask-image: linear-gradient(180deg, transparent 0, #000 12%, #000 78%, transparent 100%);
    -webkit-mask-image: linear-gradient(180deg, transparent 0, #000 12%, #000 78%, transparent 100%);
  }
  .lyrics.plain {
    overflow-y: auto;
    padding: 18% 0 30%;
  }
  .sheet {
    display: flex;
    flex-direction: column;
    align-items: stretch;
  }
  .line {
    display: flex;
    flex-direction: column;
    gap: 4px;
    text-align: left;
    padding: 12px 18px;
    margin: 0 -18px;
    border-radius: 16px;
    font-size: var(--lyric-size, clamp(24px, 2.5vw, 40px));
    font-weight: 700;
    line-height: 1.22;
    letter-spacing: -0.01em;
    color: rgba(255, 255, 255, 0.28);
    transform-origin: 0 50%;
    transition:
      transform 0.75s var(--ease),
      color 0.45s var(--ease),
      filter 0.45s var(--ease),
      background 0.2s var(--ease);
    will-change: transform;
    text-wrap: pretty;
  }
  .free .line {
    transition:
      transform 0.12s linear,
      color 0.45s var(--ease),
      filter 0.3s var(--ease),
      background 0.2s var(--ease);
    transition-delay: 0ms !important;
  }
  .line:hover {
    background: rgba(255, 255, 255, 0.06);
  }
  .line.on {
    color: rgba(255, 255, 255, 0.96);
    text-shadow: 0 0 28px rgba(255, 255, 255, 0.18);
  }
  .line.past {
    color: rgba(255, 255, 255, 0.22);
  }
  .plain .line {
    color: rgba(255, 255, 255, 0.82);
    cursor: default;
  }
  .sub {
    font-size: 0.6em;
    font-weight: 600;
    opacity: 0.78;
  }
  .gap {
    padding-top: 8px;
    padding-bottom: 8px;
  }
  .dots {
    display: flex;
    gap: 9px;
    height: 0.6em;
    align-items: center;
  }
  .dots i {
    width: 0.32em;
    height: 0.32em;
    border-radius: 50%;
    background: currentColor;
    transform: scale(0.7);
  }
  .dots.live i {
    animation: breathe 2.4s var(--ease) infinite;
  }
  .dots.live i:nth-child(2) {
    animation-delay: 0.3s;
  }
  .dots.live i:nth-child(3) {
    animation-delay: 0.6s;
  }
  @keyframes breathe {
    0%,
    100% {
      transform: scale(0.7);
      opacity: 0.45;
    }
    50% {
      transform: scale(1.05);
      opacity: 1;
    }
  }
  .tail {
    height: 70vh;
  }
  .none {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--fg-4);
  }
</style>
