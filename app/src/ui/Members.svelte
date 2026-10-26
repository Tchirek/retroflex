<script lang="ts">
  import { fade } from "svelte/transition";
  import { backable } from "../lib/back.svelte";
  import { haptic } from "../lib/haptics";
  import { saveProfile } from "../lib/names";
  import type { Room } from "../lib/room.svelte";
  import Icon from "./Icon.svelte";
  import Invite from "./Invite.svelte";

  let { room, link }: { room: Room; link: string } = $props();

  const HUES = [350, 22, 45, 140, 178, 205, 250, 290];
  let open = $state<number | null>(null);
  let below = $state(false);
  let right = $state(false);
  let draft = $state("");
  let inviting = $state(false);
  let box: HTMLDivElement;

  backable(
    () => open !== null,
    () => (commit(), (open = null)),
  );
  backable(
    () => inviting,
    () => (inviting = false),
  );

  function place() {
    const r = box.getBoundingClientRect();
    below = r.top < 300;
    right = r.left + 220 > innerWidth - 12;
  }

  const people = $derived(Object.values(room.members).sort((a, b) => (a.id === room.me ? 1 : b.id === room.me ? -1 : a.id - b.id)));
  const shown = $derived(open === null ? null : room.members[open]);

  function initial(nick: string | undefined): string {
    return nick ? [...nick][0] : "·";
  }

  // 顔に触れ　名札を小さく　差し出しぬ
  function peek(id: number) {
    if (open === id) return (open = null);
    place();
    inviting = false;
    open = id;
    if (id === room.me) draft = room.profile.nick;
    haptic();
  }

  // 名を替えて　机の傷も　新しく
  function commit() {
    const nick = draft.trim().slice(0, 24);
    if (nick && nick !== room.profile.nick) {
      const p = { nick, hue: room.profile.hue };
      saveProfile(p);
      room.rename(p);
    }
  }

  function tint(hue: number) {
    const p = { nick: room.profile.nick, hue };
    saveProfile(p);
    room.rename(p);
    haptic();
  }

  // 札ひとつ　重なりの外へ　持ち上げる
  function lift(node: HTMLElement) {
    document.body.append(node);
    let raf = 0;
    const follow = () => {
      const r = box.getBoundingClientRect();
      node.style.left = right ? "auto" : `${r.left - 4}px`;
      node.style.right = right ? `${innerWidth - r.right - 4}px` : "auto";
      node.style.top = below ? `${r.bottom + 12}px` : "auto";
      node.style.bottom = below ? "auto" : `${innerHeight - r.top + 12}px`;
      raf = requestAnimationFrame(follow);
    };
    follow();
    return {
      destroy() {
        cancelAnimationFrame(raf);
        node.remove();
      },
    };
  }

  function away(e: PointerEvent) {
    if (box.contains(e.target as Node) || (e.target as Element).closest?.("[data-card]")) return;
    if (open !== null) {
      commit();
      open = null;
    }
    inviting = false;
  }

  // 招く札　開けば顔の　札は閉ず
  function invite() {
    haptic();
    if (inviting) return (inviting = false);
    place();
    open = null;
    inviting = true;
  }
</script>

<svelte:window onpointerdown={away} />

<div class="members" bind:this={box}>
  <div class="faces">
    {#each people as m (m.id)}
      <button
        class="face"
        class:mine={m.id === room.me}
        class:sel={open === m.id}
        class:typing={!!m.draft}
        style:--hue={m.profile?.hue ?? 0}
        aria-label={m.profile?.nick ?? "member"}
        onclick={() => peek(m.id)}
      >
        {initial(m.profile?.nick)}
      </button>
    {/each}
  </div>
  <button class="icon-btn" class:on={inviting} aria-label="Invite" onclick={invite}>
    <Icon name="link" size={20} />
  </button>

  {#if inviting}
    <div class="card invite" data-card use:lift transition:fade={{ duration: 140 }}>
      <Invite {room} {link} />
    </div>
  {/if}

  {#if shown}
    <!-- 名札には　名と色ばかり　声低く -->
    <div class="card" data-card use:lift transition:fade={{ duration: 140 }}>
      <span class="big" style:--hue={shown.profile?.hue ?? 0}>{initial(shown.id === room.me ? draft || room.profile.nick : shown.profile?.nick)}</span>
      {#if shown.id === room.me}
        <input
          class="name"
          bind:value={draft}
          maxlength="24"
          aria-label="Name"
          spellcheck="false"
          onkeydown={(e) => {
            if (e.key === "Enter") {
              commit();
              open = null;
            }
            if (e.key === "Escape") open = null;
          }}
          onblur={commit}
        />
        <div class="hues">
          {#each HUES as h (h)}
            <button class="swatch" class:on={h === room.profile.hue} style:--hue={h} aria-label="Color" onclick={() => tint(h)}></button>
          {/each}
        </div>
      {:else}
        <div class="name ro">{shown.profile?.nick ?? "·"}</div>
        {#if shown.draft}<div class="dots"><i></i><i></i><i></i></div>{/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .members {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .faces {
    display: flex;
    flex-direction: row-reverse;
    justify-content: flex-end;
  }
  .face {
    position: relative;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    margin-left: -6px;
    border-radius: 50%;
    font-size: 13px;
    font-weight: 700;
    color: #fff;
    background: hsl(var(--hue) 62% 52%);
    box-shadow: 0 0 0 2px rgba(20, 20, 26, 0.55);
    transition: box-shadow 0.18s var(--ease);
  }
  .face:last-child {
    margin-left: 0;
  }
  .face:hover,
  .face.sel {
    z-index: 1;
    box-shadow:
      0 0 0 2px rgba(20, 20, 26, 0.55),
      0 0 0 4px hsl(var(--hue) 75% 70% / 0.7);
  }
  .face.typing::after {
    content: "";
    position: absolute;
    inset: -4px;
    border-radius: 50%;
    border: 2px solid hsl(var(--hue) 80% 70%);
    animation: pulse 1.4s var(--ease) infinite;
  }
  @keyframes pulse {
    0% {
      transform: scale(0.9);
      opacity: 1;
    }
    100% {
      transform: scale(1.35);
      opacity: 0;
    }
  }

  /* 硝子越し　名札ひと枚　控えめに */
  .card {
    position: fixed;
    z-index: 50;
    width: 212px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: 14px;
    border-radius: 16px;
    background: rgba(24, 24, 30, 0.82);
    border: 1px solid rgba(255, 255, 255, 0.08);
    backdrop-filter: blur(30px) saturate(160%);
    -webkit-backdrop-filter: blur(30px) saturate(160%);
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.45);
  }
  .card.invite {
    width: auto;
    align-items: center;
    padding: 16px 16px 10px;
  }
  .big {
    display: grid;
    place-items: center;
    width: 42px;
    height: 42px;
    border-radius: 50%;
    font-size: 18px;
    font-weight: 700;
    color: #fff;
    background: hsl(var(--hue) 62% 52%);
    transition: background 0.25s var(--ease);
  }
  .name {
    width: 100%;
    font-size: 16px;
    font-weight: 700;
    letter-spacing: -0.005em;
    color: var(--fg);
  }
  input.name {
    padding: 4px 0 6px;
    border: 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.14);
    outline: none;
    background: none;
    transition: border-color 0.2s;
  }
  input.name:focus {
    border-bottom-color: rgba(255, 255, 255, 0.4);
  }
  .ro {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .hues {
    display: flex;
    gap: 7px;
  }
  .swatch {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: hsl(var(--hue) 62% 52%);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.12);
    transition: box-shadow 0.15s;
  }
  .swatch.on {
    box-shadow:
      0 0 0 2px rgba(24, 24, 30, 1),
      0 0 0 3.5px rgba(255, 255, 255, 0.85);
  }
  .dots {
    display: flex;
    gap: 5px;
  }
  .dots i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--fg-2);
    animation: blink 1.2s infinite;
  }
  .dots i:nth-child(2) {
    animation-delay: 0.2s;
  }
  .dots i:nth-child(3) {
    animation-delay: 0.4s;
  }
  @keyframes blink {
    50% {
      opacity: 0.25;
    }
  }
</style>
