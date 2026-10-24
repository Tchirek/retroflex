// 一齣ごと　画面の針を　進めけり
export const frame = $state({ t: 0 });

let running = false;

export function startFrames(): void {
  if (running) return;
  running = true;
  const loop = () => {
    frame.t = performance.timeOrigin + performance.now();
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
}
