<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import type { Room } from "../lib/room.svelte";
  import { frame } from "../lib/frame.svelte";

  let { room, max = 3 }: { room: Room; max?: number } = $props();

  const LIFE = 7000;

  // 書きかけは　末の数文字　だけ見せる
  const tail = (s: string) => (s.length > 32 ? "…" + s.slice(-32) : s);
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
    {@const age = frame.t + room.clock.offset - line.at}
    <div class="bubble" style:opacity={Math.min(1, (LIFE - age) / 700)} in:fly={{ y: 12, duration: 320 }}>
      <span class="nick" style:--hue={line.hue}>{line.nick}</span>{line.text}
    </div>
  {/each}
  {#each typing as m (m.id)}
    <div class="bubble ghost" in:fade={{ duration: 200 }}>
      <span class="nick" style:--hue={m.profile?.hue ?? 0}>{m.profile?.nick ?? "·"}</span>{tail(m.draft)}<i class="caret"></i>
    </div>
  {/each}
</div>

<style>
  .whisper {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    max-width: 100%;
    pointer-events: none;
  }
  /* 一行に　収めて溢れは　点で閉ず */
  .bubble {
    max-width: min(340px, 100%);
    height: 31px;
    padding: 6px 13px;
    border-radius: 16px;
    font-size: 13.5px;
    line-height: 19px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: none;
    color: rgba(255, 255, 255, 0.92);
    background: rgba(255, 255, 255, 0.1);
    backdrop-filter: blur(24px) saturate(160%);
    -webkit-backdrop-filter: blur(24px) saturate(160%);
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
