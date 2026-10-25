<script lang="ts">
  let { on }: { on: boolean } = $props();
</script>

<!-- 選ぶ印　輪と勾のみの　控えめさ -->
<span class="tick" class:on aria-hidden="true">
  <svg width="12" height="12" viewBox="0 0 24 24"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
</span>

<style>
  .tick {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    box-shadow:
      inset 0 0 0 1.5px rgba(255, 255, 255, 0.75),
      0 2px 8px rgba(0, 0, 0, 0.35);
    background: rgba(0, 0, 0, 0.18);
    transition:
      background 0.18s var(--ease),
      box-shadow 0.18s var(--ease),
      transform 0.25s var(--spring);
  }
  .tick.on {
    background: #fff;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.35);
    transform: scale(0.96);
  }
  svg {
    fill: none;
    stroke: #111;
    stroke-width: 3.2;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-dasharray: 24;
    stroke-dashoffset: 24;
    transition: stroke-dashoffset 0.22s var(--ease);
  }
  .on svg {
    stroke-dashoffset: 0;
  }
</style>
