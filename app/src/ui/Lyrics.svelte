<script lang="ts">
  import { activeLine, type LyricLine, type Lyrics } from "../lib/lrc";
  import Mark from "./Mark.svelte";

  let { lyrics, time, onseek }: { lyrics: Lyrics | null; time: number; onseek: (t: number) => void } = $props();

  let height = $state(0);
  let els: HTMLElement[] = $state([]);
  let tops = $state<number[]>([]);
  let manual = $state<number | null>(null);
  let hover = $state(false);
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

  function hold() {
    clearTimeout(idle);
    idle = window.setTimeout(() => (manual = null), 2600);
  }

  // 指で繰る　しばしの後は　また歌へ
  function wheel(e: WheelEvent) {
    if (!lyrics?.synced) return;
    manual = (manual ?? shift) - e.deltaY;
    hold();
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
    hold();
  }

  // 指差せば　その一行へ　針が跳ぶ
  function pick(line: LyricLine) {
    if (!lyrics?.synced) return;
    clearTimeout(idle);
    manual = null;
    onseek(line.t);
  }
</script>

<div
  class="lyrics"
  role="region"
  aria-label="Lyrics"
  class:plain={lyrics && !lyrics.synced}
  class:scroll={lyrics && !lyrics.synced}
  class:hover
  bind:clientHeight={height}
  onwheel={wheel}
  ontouchstart={touchstart}
  ontouchmove={touchmove}
  onpointerenter={(e) => e.pointerType === "mouse" && (hover = true)}
  onpointerleave={() => (hover = false)}
>
  {#if !lyrics || lines.length === 0}
    <div class="none"><Mark size={72} /></div>
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
          style:--y="{lyrics.synced ? shift : 0}px"
          style:--s={i === active || !lyrics.synced ? 1 : 0.965}
          style:--blur="{lyrics.synced && i !== active ? Math.min(Math.abs(d), 6) * 0.55 : 0}px"
          style:--d="{d > 0 ? Math.min(d, 8) * 40 : 0}ms"
          onclick={() => pick(line)}
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
    padding: 0 6px;
    mask-image: linear-gradient(180deg, transparent 0, #000 12%, #000 78%, transparent 100%);
    -webkit-mask-image: linear-gradient(180deg, transparent 0, #000 12%, #000 78%, transparent 100%);
  }
  .lyrics.plain {
    overflow-y: auto;
    padding: 18% 6px 30%;
  }
  .sheet {
    display: flex;
    flex-direction: column;
    align-items: stretch;
  }

  /* 鉤の行　動きは遅れ　触れは即 */
  .line {
    display: flex;
    flex-direction: column;
    gap: 4px;
    text-align: left;
    padding: 12px 16px;
    border-radius: 16px;
    font-size: var(--lyric-size, clamp(24px, 2.5vw, 40px));
    font-weight: 700;
    line-height: 1.22;
    letter-spacing: -0.01em;
    color: rgba(255, 255, 255, 0.28);
    transform: translateY(var(--y)) scale(var(--s));
    transform-origin: 0 50%;
    filter: blur(var(--blur));
    transition:
      transform 0.75s var(--ease) var(--d),
      color 0.4s var(--ease),
      filter 0.4s var(--ease),
      background-color 0.08s linear;
    will-change: transform;
    text-wrap: pretty;
    cursor: pointer;
  }
  .hover .line,
  .free .line {
    filter: none;
  }
  .free .line {
    transition:
      transform 0.1s ease-out,
      color 0.4s var(--ease),
      filter 0.2s,
      background-color 0.08s linear;
  }
  .line:hover {
    background-color: rgba(255, 255, 255, 0.09);
  }
  .line:active {
    background-color: rgba(255, 255, 255, 0.14);
  }
  .line.on {
    color: rgba(255, 255, 255, 0.96);
    text-shadow: 0 0 28px rgba(255, 255, 255, 0.18);
  }
  .line.past {
    color: rgba(255, 255, 255, 0.22);
  }
  .hover .line:not(.on) {
    color: rgba(255, 255, 255, 0.42);
  }
  .plain .line {
    color: rgba(255, 255, 255, 0.82);
    cursor: default;
  }
  .plain .line:hover {
    background-color: transparent;
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
    color: rgba(255, 255, 255, 0.12);
  }
</style>
