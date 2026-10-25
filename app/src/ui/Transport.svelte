<script lang="ts">
  import Icon from "./Icon.svelte";

  let {
    playing,
    disabled = false,
    small = false,
    onplay,
    onpause,
    onnext,
    onprev,
  }: {
    playing: boolean;
    disabled?: boolean;
    small?: boolean;
    onplay: () => void;
    onpause: () => void;
    onnext: () => void;
    onprev: () => void;
  } = $props();
</script>

<!-- 戻り・進み　真中に息の　釦あり -->
<div class="transport" class:small class:disabled>
  <button class="side" aria-label="Previous" onclick={onprev} {disabled}>
    <span class="glyph"><Icon name="prev" size={34} /></span>
  </button>
  <button class="main" aria-label={playing ? "Pause" : "Play"} onclick={playing ? onpause : onplay} {disabled}>
    <span class="glyph"><Icon name={playing ? "pause" : "play"} size={46} /></span>
  </button>
  <button class="side" aria-label="Next" onclick={onnext} {disabled}>
    <span class="glyph"><Icon name="next" size={34} /></span>
  </button>
</div>

<style>
  /* 大小を　真中が先に　脇が追う */
  .transport {
    --size: 0.52s;
    display: flex;
    align-items: center;
    justify-content: space-evenly;
    width: 100%;
  }
  button {
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.92);
    transition:
      width var(--size) var(--ease) var(--lag, 0s),
      height var(--size) var(--ease) var(--lag, 0s),
      transform 0.3s var(--spring),
      background 0.2s var(--ease),
      opacity 0.2s;
  }
  .glyph {
    display: grid;
    place-items: center;
    transform: scale(var(--k, 1));
    transition: transform var(--size) var(--spring) var(--lag, 0s);
    will-change: transform;
  }
  .side {
    --lag: 0.05s;
    width: 58px;
    height: 58px;
  }
  .main {
    width: 76px;
    height: 76px;
  }
  .small .side {
    --k: 0.765;
    width: 46px;
    height: 46px;
  }
  .small .main {
    --k: 0.74;
    width: 58px;
    height: 58px;
  }
  button:hover {
    background: var(--glass);
  }
  button:active {
    transform: scale(0.86);
    background: var(--glass-2);
  }
  .disabled button {
    opacity: 0.35;
    pointer-events: none;
  }
</style>
