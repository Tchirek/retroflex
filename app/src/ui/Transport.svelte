<script lang="ts">
  import Icon from "./Icon.svelte";

  let {
    playing,
    disabled = false,
    small = false,
    open = false,
    muted = false,
    skips = 0,
    need = 1,
    voted = false,
    onplay,
    onpause,
    onnext,
    onprev,
    onmute,
  }: {
    playing: boolean;
    disabled?: boolean;
    small?: boolean;
    open?: boolean;
    muted?: boolean;
    skips?: number;
    need?: number;
    voted?: boolean;
    onplay: () => void;
    onpause: () => void;
    onnext: () => void;
    onprev: () => void;
    onmute?: () => void;
  } = $props();
</script>

<!-- 戻り・進み　真中に息の　釦あり -->
<div class="transport" class:small class:disabled>
  <button class="side" aria-label="Previous" onclick={onprev} disabled={disabled || open}>
    <span class="glyph"><Icon name="prev" size={34} /></span>
  </button>
  {#if open}
    <!-- 広間では　止めず己の　音を消す -->
    <button class="main" class:hush={muted} aria-label={muted ? "Unmute" : "Mute"} onclick={onmute} {disabled}>
      <span class="glyph"><Icon name={muted ? "mute" : "volume"} size={40} stroke={1.9} /></span>
    </button>
  {:else}
    <button class="main" aria-label={playing ? "Pause" : "Play"} onclick={playing ? onpause : onplay} {disabled}>
      <span class="glyph"><Icon name={playing ? "pause" : "play"} size={46} /></span>
    </button>
  {/if}
  <button class="side" class:voted aria-label="Next" onclick={onnext} {disabled}>
    <span class="glyph"><Icon name="next" size={34} /></span>
    {#if open && skips > 0}<span class="votes">{skips}/{need}</span>{/if}
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
    position: relative;
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
  button:disabled {
    opacity: 0.3;
    pointer-events: none;
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
  .hush {
    color: var(--fg-3);
  }
  .voted {
    background: var(--glass-2);
  }
  .votes {
    position: absolute;
    right: -4px;
    top: 2px;
    padding: 1px 6px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: #111;
    background: rgba(255, 255, 255, 0.9);
  }
  @media (hover: hover) {
    button:hover {
      background: var(--glass);
    }
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
