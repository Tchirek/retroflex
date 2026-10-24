<script lang="ts">
  import { tick } from "svelte";
  import type { Room } from "../lib/room.svelte";
  import Icon from "./Icon.svelte";

  let { room }: { room: Room } = $props();

  let text = $state("");
  let live = $state(readLive());
  let list: HTMLDivElement;
  let last = 0;
  let pending = 0;

  function readLive(): boolean {
    try {
      return localStorage.getItem("retroflex.live") !== "0";
    } catch {
      return true;
    }
  }

  const typing = $derived(Object.values(room.members).filter((m) => m.id !== room.me && m.draft));

  // 一文字の　ためらいまでも　風に乗す
  function stream() {
    if (!live) return;
    clearTimeout(pending);
    const wait = Math.max(0, 45 - (performance.now() - last));
    pending = window.setTimeout(() => {
      last = performance.now();
      room.draft(text);
    }, wait);
  }

  function toggle() {
    live = !live;
    try {
      localStorage.setItem("retroflex.live", live ? "1" : "0");
    } catch {}
    if (!live) room.draft("");
    else stream();
  }

  function send() {
    const body = text.trim();
    if (!body) return;
    clearTimeout(pending);
    room.say(body);
    text = "";
  }

  // 新しき　言葉の方へ　目を落とす
  $effect(() => {
    void room.lines.length;
    void typing.map((m) => m.draft).join();
    tick().then(() => list && list.scrollTo({ top: list.scrollHeight, behavior: "smooth" }));
  });
</script>

<div class="chat">
  <div class="list scroll" bind:this={list}>
    {#each room.lines as line (line.key)}
      <div class="line" class:mine={line.mine}>
        <span class="nick" style:--hue={line.hue}>{line.nick}</span>
        <span class="text">{line.text}</span>
      </div>
    {/each}
    {#each typing as m (m.id)}
      <div class="line ghost">
        <span class="nick" style:--hue={m.profile?.hue ?? 0}>{m.profile?.nick ?? "·"}</span>
        <span class="text">{m.draft}<i class="caret"></i></span>
      </div>
    {/each}
  </div>
  <div class="bar glass">
    <button class="icon-btn" class:on={live} aria-label="Live typing" aria-pressed={live} onclick={toggle}>
      <Icon name={live ? "eye" : "eyeoff"} size={19} />
    </button>
    <input
      bind:value={text}
      maxlength="2000"
      aria-label="Message"
      enterkeyhint="send"
      oninput={stream}
      onkeydown={(e) => {
        if (e.key === "Enter" && !e.isComposing) {
          e.preventDefault();
          send();
        }
      }}
    />
    <button class="icon-btn send" class:on={!!text.trim()} aria-label="Send" onclick={send}>
      <Icon name="send" size={19} stroke={2.2} />
    </button>
  </div>
</div>

<style>
  .chat {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    gap: 12px;
  }
  .list {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 30% 4px 6px;
    mask-image: linear-gradient(180deg, transparent 0, #000 24%);
    -webkit-mask-image: linear-gradient(180deg, transparent 0, #000 24%);
  }
  .line {
    font-size: 16px;
    line-height: 1.45;
    word-break: break-word;
    user-select: text;
    -webkit-user-select: text;
    animation: rise 0.35s var(--ease);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  .nick {
    font-weight: 700;
    margin-right: 8px;
    color: hsl(var(--hue) 85% 74%);
  }
  .text {
    color: rgba(255, 255, 255, 0.9);
  }
  .ghost .text {
    color: rgba(255, 255, 255, 0.5);
    font-style: italic;
  }
  .caret {
    display: inline-block;
    width: 2px;
    height: 1.05em;
    margin-left: 2px;
    vertical-align: -0.18em;
    background: currentColor;
    animation: blink 1s steps(2) infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 5px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.08);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 36px;
    padding: 0 6px;
    border: 0;
    outline: none;
    background: transparent;
    font-size: 15.5px;
  }
  .send.on {
    color: #111;
    background: rgba(255, 255, 255, 0.92);
  }
</style>
