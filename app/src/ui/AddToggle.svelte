<script lang="ts">
  import { haptic } from "../lib/haptics";

  let {
    on,
    size = 18,
    variant = "plain",
    onclick,
  }: { on: boolean; size?: number; variant?: "plain" | "dot" | "pill"; onclick: () => void } = $props();

  let pop = $state(false);

  // 押すたびに　小さく跳ねて　気を知らす
  function press() {
    pop = false;
    requestAnimationFrame(() => (pop = true));
    haptic();
    onclick();
  }
</script>

<!-- 十字消え　鉤の印が　すっと書く -->
<button
  class="add {variant}"
  class:on
  class:pop
  aria-pressed={on}
  aria-label={on ? "Remove from queue" : "Add to queue"}
  onclick={press}
  onanimationend={() => (pop = false)}
>
  <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
    <path class="plus" d="M12 5.5v13M5.5 12h13" />
    <path class="check" d="M5.8 12.6l4.1 4.1 8.3-9" pathLength="1" />
  </svg>
</button>

<style>
  .add {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    color: var(--fg-2);
    border-radius: 50%;
    transition:
      background 0.22s var(--ease),
      color 0.22s var(--ease),
      transform 0.25s var(--ease),
      opacity 0.15s;
  }
  .plain {
    width: 38px;
    height: 38px;
  }
  .plain:hover {
    color: var(--fg);
    background: var(--glass);
  }
  .dot {
    width: 34px;
    height: 34px;
    color: #fff;
    background: rgba(20, 20, 24, 0.72);
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
  }
  .pill {
    width: 52px;
    height: 36px;
    border-radius: 999px;
    color: var(--fg);
    background: var(--glass-2);
  }
  .add.on {
    color: #111;
    background: #fff;
  }
  .add:active {
    transform: scale(0.88);
  }
  .add.pop {
    animation: pop 0.38s var(--ease);
  }
  @keyframes pop {
    35% {
      transform: scale(0.82);
    }
    100% {
      transform: scale(1);
    }
  }
  svg {
    display: block;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
    overflow: visible;
  }
  .plus {
    transform-origin: 12px 12px;
    transition:
      transform 0.3s var(--ease),
      opacity 0.18s;
  }
  .on .plus {
    transform: rotate(90deg) scale(0.2);
    opacity: 0;
  }
  .check {
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    stroke-width: 2.6;
    transition: stroke-dashoffset 0.16s ease-in;
  }
  .on .check {
    stroke-dashoffset: 0;
    transition: stroke-dashoffset 0.3s cubic-bezier(0.65, 0, 0.35, 1) 0.1s;
  }
</style>
