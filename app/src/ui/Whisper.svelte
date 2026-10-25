<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import type { Room } from "../lib/room.svelte";
  import { frame } from "../lib/frame.svelte";

  let { room, max = 3 }: { room: Room; max?: number } = $props();

  const LIFE = 7000;
  // 耳打ちは　ほんの数秒　浮かび消ゆ
  const typing = $derived(Object.values(room.members).filter((m) => m.id !== room.me && m.draft).slice(0, max));
  const recent = $derived.by(() => {
    const left = max - typing.length;
    if (left <= 0) return [];
    return room.lines.filter((l) => !l.mine && frame.t + room.clock.offset - l.at < LIFE).slice(-left);
  });
</script>

<div class="whisper" aria-live="polite">
  {#each recent as line (line.key)}
    <div class="bubble" in:fly={{ y: 12, duration: 320 }} out:fade={{ duration: 600 }}>
      <span class="nick" style:--hue={line.hue}>{line.nick}</span>{line.text}
    </div>
  {/each}
  {#each typing as m (m.id)}
    <div class="bubble ghost" in:fade={{ duration: 200 }} out:fade={{ duration: 400 }}>
      <span class="nick" style:--hue={m.profile?.hue ?? 0}>{m.profile?.nick ?? "·"}</span>{m.draft}<i class="caret"></i>
    </div>
  {/each}
</div>

<style>
  .whisper {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    pointer-events: none;
  }
  .bubble {
    max-width: min(440px, 80vw);
    padding: 8px 14px;
    border-radius: 18px;
    font-size: 14.5px;
    line-height: 1.4;
    color: rgba(255, 255, 255, 0.92);
    background: rgba(255, 255, 255, 0.1);
    backdrop-filter: blur(24px) saturate(160%);
    -webkit-backdrop-filter: blur(24px) saturate(160%);
    word-break: break-word;
  }
  .ghost {
    color: rgba(255, 255, 255, 0.55);
    font-style: italic;
  }
  .nick {
    font-weight: 700;
    font-style: normal;
    margin-right: 7px;
    color: hsl(var(--hue) 85% 74%);
  }
  .caret {
    display: inline-block;
    width: 2px;
    height: 1em;
    margin-left: 2px;
    vertical-align: -0.15em;
    background: currentColor;
    animation: blink 1s steps(2) infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
</style>
