import { haptic } from "./haptics";

type Choice = {
  onstart?: () => void;
  onend?: () => void;
  onpick?: (el: HTMLElement, index: number) => void;
  hold?: number;
};

const HOLD = 300;

// 長押しの　のちに滑らせ　選び取る
export function scrub(node: HTMLElement, options: Choice) {
  let opts = options;
  let timer = 0;
  let clickTimer = 0;
  let active = false;
  let fired = false;
  let id = -1;
  let sx = 0;
  let sy = 0;
  let last = -1;
  let boxes: { el: HTMLElement; x: number }[] = [];

  const collect = () => {
    boxes = Array.from(node.querySelectorAll<HTMLElement>("[data-choice]"))
      .filter((el) => !(el as HTMLButtonElement).disabled)
      .map((el) => {
        const r = el.getBoundingClientRect();
        return { el, x: r.left + r.width / 2 };
      });
    last = -1;
  };

  const pick = (x: number) => {
    if (!boxes.length) return;
    let best = 0;
    let dist = Infinity;
    for (let i = 0; i < boxes.length; i++) {
      const d = Math.abs(x - boxes[i].x);
      if (d < dist) {
        dist = d;
        best = i;
      }
    }
    if (best === last) return;
    last = best;
    haptic();
    opts.onpick?.(boxes[best].el, best);
  };

  const down = (e: PointerEvent) => {
    if (e.pointerType === "mouse" && e.button !== 0) return;
    fired = false;
    active = false;
    id = e.pointerId;
    sx = e.clientX;
    sy = e.clientY;
    clearTimeout(timer);
    timer = window.setTimeout(() => {
      const start = document.elementFromPoint(sx, sy)?.closest<HTMLElement>("[data-choice]");
      if (start && (start as HTMLButtonElement).disabled) return;
      collect();
      if (!boxes.length) return;
      active = true;
      fired = true;
      node.classList.add("scrubbing");
      opts.onstart?.();
      try {
        node.setPointerCapture(id);
      } catch {}
      pick(sx);
    }, opts.hold ?? HOLD);
  };

  const move = (e: PointerEvent) => {
    if (e.pointerId !== id) return;
    if (!active) {
      if (Math.hypot(e.clientX - sx, e.clientY - sy) > 8) clearTimeout(timer);
      return;
    }
    if (e.cancelable) e.preventDefault();
    pick(e.clientX);
  };

  const up = (e: PointerEvent) => {
    if (e.pointerId !== id) return;
    clearTimeout(timer);
    if (!active) return;
    active = false;
    node.classList.remove("scrubbing");
    opts.onend?.();
    clearTimeout(clickTimer);
    clickTimer = window.setTimeout(() => (fired = false), 60);
  };

  // 途絶えても　指の印を　畳みけり
  const cancel = () => {
    clearTimeout(timer);
    if (!active) return;
    active = false;
    node.classList.remove("scrubbing");
    opts.onend?.();
  };

  const click = (e: MouseEvent) => {
    if (!fired) return;
    e.preventDefault();
    e.stopPropagation();
    fired = false;
  };

  const menu = (e: Event) => {
    if (!fired) return;
    e.preventDefault();
  };

  const leave = () => {
    if (!active) clearTimeout(timer);
  };

  node.addEventListener("pointerdown", down);
  node.addEventListener("pointermove", move);
  node.addEventListener("pointerup", up);
  node.addEventListener("pointercancel", cancel);
  node.addEventListener("pointerleave", leave);
  node.addEventListener("click", click, true);
  node.addEventListener("contextmenu", menu);

  return {
    update(next: Choice) {
      opts = next;
    },
    destroy() {
      clearTimeout(timer);
      clearTimeout(clickTimer);
      node.classList.remove("scrubbing");
      node.removeEventListener("pointerdown", down);
      node.removeEventListener("pointermove", move);
      node.removeEventListener("pointerup", up);
      node.removeEventListener("pointercancel", cancel);
      node.removeEventListener("pointerleave", leave);
      node.removeEventListener("click", click, true);
      node.removeEventListener("contextmenu", menu);
    },
  };
}
