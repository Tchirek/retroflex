<script lang="ts">
  import qrcode from "qrcode-generator";
  import { onMount } from "svelte";
  import { makeInvite } from "../lib/api";
  import { haptic } from "../lib/haptics";
  import type { Room } from "../lib/room.svelte";
  import { copyText, sendLink } from "../lib/share";
  import Icon from "./Icon.svelte";

  let { room, link }: { room: Room; link: string } = $props();

  let code = $state("");
  let copied = $state<"" | "code" | "link">("");
  const canShare = typeof navigator !== "undefined" && !!navigator.share;

  // 道の札　白黒の升に　描き写す
  const art = $derived.by(() => {
    const qr = qrcode(0, "M");
    qr.addData(link);
    qr.make();
    const n = qr.getModuleCount();
    let d = "";
    for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) if (qr.isDark(r, c)) d += `M${c + 2},${r + 2}h1v1h-1z`;
    return { d, size: n + 4 };
  });

  onMount(() => {
    makeInvite(room.hub, room.id, room.keys.ticket, room.secret).then((c) => (code = c ?? ""));
  });

  async function copy(what: "code" | "link") {
    if (!(await copyText(what === "code" ? code : link))) return;
    haptic();
    copied = what;
    setTimeout(() => (copied = ""), 1500);
  }
</script>

<!-- 招く札　目で読む升と　口の字と -->
<div class="invite">
  <svg class="qr" viewBox="0 0 {art.size} {art.size}" shape-rendering="crispEdges" aria-label="QR">
    <rect width={art.size} height={art.size} fill="#fff" />
    <path d={art.d} fill="#0b0b0f" />
  </svg>
  <button class="code" class:done={copied === "code"} aria-label="Copy code" disabled={!code} onclick={() => copy("code")}>
    {#if code}{code.slice(0, 3)}<i></i>{code.slice(3)}{:else}<span class="wait">······</span>{/if}
  </button>
  <div class="acts">
    <button class="icon-btn" class:on={copied === "link"} aria-label="Copy link" onclick={() => copy("link")}>
      <Icon name={copied === "link" ? "check" : "link"} size={19} />
    </button>
    {#if canShare}
      <button class="icon-btn" aria-label="Share" onclick={() => sendLink(link)}><Icon name="share" size={19} /></button>
    {/if}
  </div>
</div>

<style>
  .invite {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }
  .qr {
    width: 168px;
    height: 168px;
    border-radius: 12px;
    display: block;
  }
  .code {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 14px;
    border-radius: 12px;
    font-family: ui-monospace, "SF Mono", "Cascadia Mono", Menlo, monospace;
    font-size: 22px;
    font-weight: 700;
    letter-spacing: 0.14em;
    color: var(--fg);
    transition:
      background 0.2s var(--ease),
      transform 0.25s var(--spring);
  }
  .code i {
    width: 6px;
  }
  .code:active {
    transform: scale(0.95);
  }
  .code.done {
    background: var(--glass-2);
  }
  .wait {
    color: var(--fg-3);
  }
  .acts {
    display: flex;
    gap: 6px;
  }
</style>
