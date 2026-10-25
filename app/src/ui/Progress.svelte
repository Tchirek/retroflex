<script lang="ts">
  import { clock } from "../lib/library";

  let {
    time,
    duration,
    onseek,
    locked = false,
  }: { time: number; duration: number; onseek: (t: number) => void; locked?: boolean } = $props();

  let bar: HTMLDivElement;
  let scrub = $state<number | null>(null);
  const shown = $derived(scrub ?? Math.min(time, duration || time));
  const ratio = $derived(duration > 0 ? Math.min(1, Math.max(0, shown / duration)) : 0);

  // 指を引く　時の帯をば　手繰り寄せ
  function at(e: PointerEvent): number {
    const r = bar.getBoundingClientRect();
    return Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) * duration;
  }
  function down(e: PointerEvent) {
    if (!duration || locked) return;
    bar.setPointerCapture(e.pointerId);
    scrub = at(e);
  }
  function move(e: PointerEvent) {
    if (scrub !== null) scrub = at(e);
  }
  function up(e: PointerEvent) {
    if (scrub === null) return;
    onseek(at(e));
    scrub = null;
  }
</script>

<div class="progress" class:active={scrub !== null}>
  <div
    class="bar"
    bind:this={bar}
    role="slider"
    tabindex="0"
    aria-label="Position"
    aria-valuemin={0}
    aria-valuemax={Math.round(duration)}
    aria-valuenow={Math.round(shown)}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={() => (scrub = null)}
  >
    <div class="track"><div class="fill" style:transform="scaleX({ratio})"></div></div>
  </div>
  <div class="times">
    <span>{clock(shown)}</span>
    <span>-{clock(Math.max(0, duration - shown))}</span>
  </div>
</div>

<style>
  .progress {
    width: 100%;
  }
  /* 太れども　器の丈は　変わらずに */
  .bar {
    height: 24px;
    display: flex;
    align-items: center;
    cursor: pointer;
    touch-action: none;
    outline: none;
  }
  .track {
    flex: 1;
    height: 6px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.18);
    overflow: hidden;
    transition:
      height 0.25s var(--spring),
      background 0.2s;
  }
  .bar:hover .track,
  .active .track {
    height: 10px;
    background: rgba(255, 255, 255, 0.24);
  }
  .fill {
    height: 100%;
    background: rgba(255, 255, 255, 0.82);
    transform-origin: 0 50%;
    will-change: transform;
  }
  .active .fill {
    background: #fff;
  }
  .times {
    display: flex;
    justify-content: space-between;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--fg-3);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.02em;
  }
</style>
