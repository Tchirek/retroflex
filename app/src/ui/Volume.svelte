<script lang="ts">
  import Icon from "./Icon.svelte";

  let { value, onchange }: { value: number; onchange: (v: number) => void } = $props();
  let last = 1;
</script>

<!-- 音の丈　ひそかに耳へ　寄せる指 -->
<div class="volume">
  <button
    class="icon-btn"
    aria-label="Mute"
    onclick={() => {
      if (value > 0) {
        last = value;
        onchange(0);
      } else onchange(last || 1);
    }}
  >
    <Icon name={value > 0 ? "volume" : "mute"} size={18} />
  </button>
  <input
    type="range"
    min="0"
    max="1"
    step="0.01"
    {value}
    style:--v={value}
    aria-label="Volume"
    oninput={(e) => onchange(Number(e.currentTarget.value))}
  />
</div>

<style>
  .volume {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
  }
  input {
    flex: 1;
    appearance: none;
    -webkit-appearance: none;
    height: 6px;
    border-radius: 6px;
    background: linear-gradient(90deg, rgba(255, 255, 255, 0.8) calc(var(--v) * 100%), rgba(255, 255, 255, 0.18) 0);
    outline: none;
    cursor: pointer;
  }
  input::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #fff;
    opacity: 0;
    transition: opacity 0.2s;
  }
  input:hover::-webkit-slider-thumb {
    opacity: 1;
  }
  input::-moz-range-thumb {
    width: 14px;
    height: 14px;
    border: 0;
    border-radius: 50%;
    background: #fff;
  }
</style>
