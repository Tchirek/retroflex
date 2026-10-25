<script lang="ts">
  import { saveProfile } from "../lib/names";
  import type { Room } from "../lib/room.svelte";
  import Icon from "./Icon.svelte";

  let { room, onshare }: { room: Room; onshare: () => Promise<boolean> } = $props();

  let editing = $state(false);
  let draft = $state("");
  let copied = $state(false);

  const people = $derived(Object.values(room.members).sort((a, b) => (a.id === room.me ? 1 : b.id === room.me ? -1 : a.id - b.id)));

  // 呼ばれれば　すぐに指先　そこに置く
  function focus(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  function initial(nick: string | undefined): string {
    return nick ? [...nick][0] : "·";
  }

  // 名を替えて　机の傷も　新しく
  function commit() {
    const nick = draft.trim().slice(0, 24);
    if (nick) {
      const p = { nick, hue: room.profile.hue };
      saveProfile(p);
      room.rename(p);
    }
    editing = false;
  }

  async function share() {
    if (await onshare()) {
      copied = true;
      setTimeout(() => (copied = false), 1600);
    }
  }
</script>

<div class="members">
  <div class="faces">
    {#each people as m (m.id)}
      {@const mine = m.id === room.me}
      <button
        class="face"
        class:mine
        class:typing={!!m.draft}
        style:--hue={m.profile?.hue ?? 0}
        title={m.profile?.nick}
        aria-label={m.profile?.nick ?? "member"}
        onclick={() => {
          if (mine) {
            draft = room.profile.nick;
            editing = true;
          }
        }}
      >
        {initial(m.profile?.nick)}
      </button>
    {/each}
  </div>
  {#if editing}
    <input
      class="rename glass"
      bind:value={draft}
      maxlength="24"
      use:focus
      aria-label="Name"
      onkeydown={(e) => {
        if (e.key === "Enter") commit();
        if (e.key === "Escape") editing = false;
      }}
      onblur={commit}
    />
  {/if}
  <button class="icon-btn" class:on={copied} aria-label="Copy room link" onclick={share}>
    <Icon name={copied ? "check" : "link"} size={20} />
  </button>
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
    cursor: default;
    transition: transform 0.3s var(--spring);
  }
  .face:last-child {
    margin-left: 0;
  }
  .face.mine {
    cursor: pointer;
  }
  .face:hover {
    transform: translateY(-2px);
    z-index: 1;
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
      transform: scale(1);
      opacity: 1;
    }
    100% {
      transform: scale(0.78);
      opacity: 0;
    }
  }
  .rename {
    position: absolute;
    bottom: 40px;
    left: 0;
    width: 180px;
    padding: 8px 12px;
    border-radius: 12px;
    outline: none;
    background: rgba(30, 30, 38, 0.6);
    z-index: 5;
  }
</style>
