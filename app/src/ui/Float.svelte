<script lang="ts">
  import { onMount } from "svelte";
  import type { Station } from "../lib/api";
  import Art from "./Art.svelte";

  let {
    station,
    cover,
    at,
    rest = false,
    slotName = "big",
    follow = false,
  }: { station: Station | null; cover?: string; at: string; rest?: boolean; slotName?: string; follow?: boolean } = $props();

  type Box = { x: number; y: number; w: number; h: number; r: number };
  const DUR = 520;

  let el: HTMLDivElement;
  let kind = $state("big");
  let box: Box | null = null;
  let flight: { from: Box; t0: number } | null = null;
  let radii = new WeakMap<Element, number>();
  let target: HTMLElement | null = null;

  // 曲線の　緩みを解きて　時に問う
  function bezier(x1: number, y1: number, x2: number, y2: number) {
    const c = (a: number, b: number, t: number) => 3 * a * t * (1 - t) ** 2 + 3 * b * t * t * (1 - t) + t ** 3;
    return (x: number) => {
      let lo = 0;
      let hi = 1;
      for (let i = 0; i < 22; i++) {
        const mid = (lo + hi) / 2;
        if (c(x1, x2, mid) < x) lo = mid;
        else hi = mid;
      }
      return c(y1, y2, (lo + hi) / 2);
    };
  }
  const ease = bezier(0.2, 0.8, 0.2, 1);

  // 今の座を　指の行先に　結びつけ
  function slotEl(): HTMLElement | null {
    if (target?.isConnected && target.dataset.slot === slotName) return target;
    target = document.querySelector<HTMLElement>(`.phone [data-slot="${slotName}"]`);
    return target;
  }

  function read(s: HTMLElement): Box {
    const r = s.getBoundingClientRect();
    let radius = radii.get(s);
    if (radius === undefined) radii.set(s, (radius = parseFloat(getComputedStyle(s).borderTopLeftRadius) || 0));
    return { x: r.left, y: r.top, w: r.width, h: r.height, r: radius };
  }

  const same = (a: Box | null, b: Box) => !!a && a.x === b.x && a.y === b.y && a.w === b.w && a.h === b.h && a.r === b.r;

  // 行く先の　今の姿を　毎拍追う
  function frame(now: number) {
    if (!el) return;
    const s = slotEl();
    if (!s) return;
    if (slotName !== kind) kind = slotName;
    const to = read(s);
    let next = to;
    if (flight) {
      const p = ease(Math.min(1, Math.max(0, now - flight.t0) / DUR));
      const f = flight.from;
      const mix = (a: number, b: number) => a + (b - a) * p;
      next = { x: mix(f.x, to.x), y: mix(f.y, to.y), w: mix(f.w, to.w), h: mix(f.h, to.h), r: mix(f.r, to.r) };
      if (p >= 1) flight = null;
    }
    if (same(box, next)) return;
    box = next;
    el.style.transform = `translate3d(${next.x}px, ${next.y}px, 0)`;
    el.style.width = `${next.w}px`;
    el.style.height = `${next.h}px`;
    el.style.borderRadius = `${next.r}px`;
  }

  let raf = 0;
  let until = 0;

  // 針を止め　動くときのみ　影を追う
  function wake(ms = 700) {
    until = Math.max(until, performance.now() + ms);
    if (!raf && !document.hidden) raf = requestAnimationFrame(loop);
  }

  function loop(now: number) {
    raf = 0;
    frame(now);
    if (!document.hidden && (flight || follow || now < until)) raf = requestAnimationFrame(loop);
  }

  $effect(() => {
    void at;
    void slotName;
    target = null;
    if (box) flight = { from: box, t0: performance.now() };
    wake();
  });

  // 頁を繰る間は　影もまた　足を止めず
  $effect(() => {
    if (follow) wake(1200);
  });

  onMount(() => {
    // 窓移り　器の丈を　測りけり
    const measure = () => {
      radii = new WeakMap();
      wake(500);
    };
    const ro = new ResizeObserver(measure);
    ro.observe(document.documentElement);
    const phone = document.querySelector(".phone");
    if (phone) ro.observe(phone);
    const view = () => wake(400);
    visualViewport?.addEventListener("resize", view);
    visualViewport?.addEventListener("scroll", view);
    addEventListener("resize", view);
    addEventListener("orientationchange", view);
    // 翳りし間　戻りし時に　また測る
    const show = () => {
      if (!document.hidden) measure();
    };
    document.addEventListener("visibilitychange", show);
    wake(900);
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      visualViewport?.removeEventListener("resize", view);
      visualViewport?.removeEventListener("scroll", view);
      removeEventListener("resize", view);
      removeEventListener("orientationchange", view);
      document.removeEventListener("visibilitychange", show);
    };
  });
</script>

<!-- 盤ひとつ　小窓大窓　渡り鳥 -->
<div class="float {kind}" bind:this={el} aria-hidden="true">
  <div class="inner" class:rest={rest && kind === "big"}>
    <Art {station} {cover} full eager />
  </div>
</div>

<style>
  .float {
    position: fixed;
    left: 0;
    top: 0;
    z-index: 3;
    width: 0;
    height: 0;
    pointer-events: none;
    will-change: transform;
  }
  .inner {
    width: 100%;
    height: 100%;
    border-radius: inherit;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.06);
    transition:
      transform 0.7s var(--spring),
      box-shadow 0.5s var(--ease);
  }
  .big .inner {
    box-shadow:
      0 30px 70px rgba(0, 0, 0, 0.42),
      0 0 0 0.5px rgba(255, 255, 255, 0.12);
  }
  .thumb .inner {
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.3);
  }
  .dock .inner {
    box-shadow: none;
  }
  .inner.rest {
    transform: scale(0.84);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.3);
  }
</style>
