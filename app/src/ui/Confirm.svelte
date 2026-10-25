<script lang="ts">
  import { haptic } from "../lib/haptics";
  import Icon from "./Icon.svelte";

  let {
    onconfirm,
    label,
    size = 18,
    variant = "icon",
  }: { onconfirm: () => void; label: string; size?: number; variant?: "icon" | "pill" } = $props();

  let armed = $state(false);
  let timer = 0;

  // 一度目は　赤く身構え　二度目で消す
  function go() {
    clearTimeout(timer);
    if (armed) {
      armed = false;
      haptic("medium");
      onconfirm();
      return;
    }
    armed = true;
    haptic();
    timer = window.setTimeout(() => (armed = false), 2600);
  }
</script>

<button class={variant} class:armed aria-label={label} onclick={go}>
  <Icon name="trash" {size} />
</button>

<style>
  button {
    display: grid;
    place-items: center;
    color: var(--fg-2);
    transition:
      color 0.2s var(--ease),
      background 0.2s var(--ease),
      transform 0.25s var(--spring);
  }
  .icon {
    width: 38px;
    height: 38px;
    border-radius: 50%;
  }
  .pill {
    width: 52px;
    height: 36px;
    border-radius: 999px;
    color: var(--fg);
    background: var(--glass-2);
  }
  @media (hover: hover) {
    .icon:hover {
      color: var(--fg);
      background: var(--glass);
    }
  }
  button:active {
    transform: scale(0.9);
  }
  .armed,
  .armed:hover {
    color: #fff;
    background: #ff453a;
  }
</style>
