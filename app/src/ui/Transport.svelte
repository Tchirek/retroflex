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
  <button class="side" aria-label="Previous" onclick={onprev} {disabled}><Icon name="prev" size={small ? 26 : 34} /></button>
  <button class="main" aria-label={playing ? "Pause" : "Play"} onclick={playing ? onpause : onplay} {disabled}>
    <Icon name={playing ? "pause" : "play"} size={small ? 34 : 46} />
  </button>
  <button class="side" aria-label="Next" onclick={onnext} {disabled}><Icon name="next" size={small ? 26 : 34} /></button>
</div>

<style>
  .transport {
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
      transform 0.3s var(--spring),
      background 0.2s var(--ease),
      opacity 0.2s;
  }
  .side {
    width: 58px;
    height: 58px;
  }
  .main {
    width: 76px;
    height: 76px;
  }
  .small .side {
    width: 46px;
    height: 46px;
  }
  .small .main {
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
