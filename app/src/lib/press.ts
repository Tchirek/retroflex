import { haptic } from "./haptics";

const HOLD = 420;

// 長押しの　間合いを計り　選び取る
export function press(node: HTMLElement, onlong: (() => void) | undefined) {
  let fn = onlong;
  let timer = 0;
  let fired = false;
  let x = 0;
  let y = 0;
  const fire = () => {
    fired = true;
    haptic("medium");
    fn?.();
  };
  const down = (e: PointerEvent) => {
    if (e.button !== 0 || !fn) return;
    fired = false;
    x = e.clientX;
    y = e.clientY;
    clearTimeout(timer);
    timer = window.setTimeout(fire, HOLD);
  };
  const move = (e: PointerEvent) => {
    if (Math.hypot(e.clientX - x, e.clientY - y) > 9) clearTimeout(timer);
  };
  const stop = () => clearTimeout(timer);
  const click = (e: MouseEvent) => {
    if (!fired) return;
    fired = false;
    e.preventDefault();
    e.stopPropagation();
  };
  // 右押しも　長押し同じ　入り口に
  const menu = (e: MouseEvent) => {
    if (!fn) return;
    e.preventDefault();
    if (fired) return;
    clearTimeout(timer);
    fire();
  };
  node.addEventListener("pointerdown", down);
  node.addEventListener("pointermove", move);
  node.addEventListener("pointerup", stop);
  node.addEventListener("pointercancel", stop);
  node.addEventListener("pointerleave", stop);
  node.addEventListener("click", click, true);
  node.addEventListener("contextmenu", menu);
  return {
    update(next: (() => void) | undefined) {
      fn = next;
    },
    destroy() {
      clearTimeout(timer);
      node.removeEventListener("pointerdown", down);
      node.removeEventListener("pointermove", move);
      node.removeEventListener("pointerup", stop);
      node.removeEventListener("pointercancel", stop);
      node.removeEventListener("pointerleave", stop);
      node.removeEventListener("click", click, true);
      node.removeEventListener("contextmenu", menu);
    },
  };
}
