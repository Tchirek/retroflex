<script lang="ts" module>
  // 書きかけは　部屋ごと棚に　しまいおく
  const drafts = new Map<string, string>();
</script>

<script lang="ts">
  import { onMount } from "svelte";
  import { chars } from "../lib/crypto";
  import { frame } from "../lib/frame.svelte";
  import type { Line, Room } from "../lib/room.svelte";
  import Icon from "./Icon.svelte";

  let { room }: { room: Room } = $props();

  const LIMIT = 450;

  let text = $state("");
  let bound = "";
  let input: HTMLInputElement;
  let last = 0;
  let pending = 0;

  const typing = $derived(Object.values(room.members).filter((m) => m.id !== room.me && m.draft));
  const wait = $derived(Math.max(0, Math.ceil((room.hush - frame.t) / 1000)));

  $effect.pre(() => {
    if (room.id === bound) return;
    bound = room.id;
    text = drafts.get(room.id) ?? "";
  });

  $effect(() => {
    drafts.set(room.id, text);
  });

  // 戻されし　言葉は手元の　箱に戻す
  $effect(() => {
    const back = room.bounced;
    if (!back) return;
    if (!text.trim()) text = back;
    room.bounced = "";
  });

  // 一文字の　ためらいまでも　風に乗す
  function stream() {
    if (chars(text) > LIMIT) text = Array.from(text).slice(0, LIMIT).join("");
    clearTimeout(pending);
    const gap = Math.max(0, 45 - (performance.now() - last));
    pending = window.setTimeout(() => {
      last = performance.now();
      room.draft(text);
    }, gap);
  }

  function send() {
    const body = text.trim();
    if (!body || wait > 0) return;
    clearTimeout(pending);
    room.say(body);
    text = "";
  }

  // 刻の札　半時ごとに　間に立つ
  type Entry = { kind: "time"; key: string; label: string } | { kind: "line"; key: string; line: Line };
  const entries = $derived.by(() => {
    const out: Entry[] = [];
    const today = new Date().toDateString();
    let slot = "";
    for (const line of room.lines) {
      const d = new Date(line.at);
      d.setMinutes(d.getMinutes() < 30 ? 0 : 30, 0, 0);
      const key = String(d.getTime());
      if (key !== slot) {
        slot = key;
        const time = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
        const day = d.toDateString() === today ? "" : `${d.toLocaleDateString([], { month: "numeric", day: "numeric" })} `;
        out.push({ kind: "time", key: `t${key}`, label: day + time });
      }
      out.push({ kind: "line", key: line.key, line });
    }
    return out;
  });

  onMount(() => {
    // 鍵盤を　替えに出た手が　戻る時
    let held = false;
    const away = () => (held = document.activeElement === input);
    const back = () => {
      if (held) input?.focus({ preventScroll: true });
      held = false;
    };
    addEventListener("blur", away);
    addEventListener("focus", back);
    return () => {
      removeEventListener("blur", away);
      removeEventListener("focus", back);
      clearTimeout(pending);
      // 席を立つ　書きかけの影は　皆に消す
      if (text) room.draft("");
    };
  });

</script>

<div class="chat">
  <div class="list scroll">
    <div class="inner">
      {#each entries as e (e.key)}
        {#if e.kind === "time"}
          <div class="time">{e.label}</div>
        {:else}
          <div class="line" class:mine={e.line.mine}>
            <span class="nick" style:--hue={e.line.hue}>{e.line.nick}</span>
            <span class="text">{e.line.text}</span>
          </div>
        {/if}
      {/each}
      {#each typing as m (m.id)}
        <div class="line ghost">
          <span class="nick" style:--hue={m.profile?.hue ?? 0}>{m.profile?.nick ?? "·"}</span>
          <span class="text">{m.draft}<i class="caret"></i></span>
        </div>
      {/each}
    </div>
  </div>
  <div class="bar glass">
    <input
      bind:this={input}
      bind:value={text}
      maxlength={LIMIT * 2}
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
    <button class="icon-btn send" class:on={!!text.trim() && !wait} class:hold={wait > 0} aria-label="Send" onclick={send}>
      {#if wait > 0}<span class="wait">{wait}</span>{:else}<Icon name="send" size={19} stroke={2.2} />{/if}
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
  /* 底に錨　新しき声は　下から積む */
  .list {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column-reverse;
    mask-image: linear-gradient(180deg, transparent 0, #000 24%);
    -webkit-mask-image: linear-gradient(180deg, transparent 0, #000 24%);
  }
  .inner {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 30vh 4px 6px;
  }
  .time {
    align-self: center;
    margin: 10px 0 4px;
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--fg-3);
    font-variant-numeric: tabular-nums;
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
    padding: 0 6px 0 12px;
    border: 0;
    outline: none;
    background: transparent;
    font-size: 15.5px;
  }
  .send.on {
    color: #111;
    background: rgba(255, 255, 255, 0.92);
  }
  .send.hold {
    cursor: default;
  }
  .wait {
    font-size: 13px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--fg-3);
  }
</style>
