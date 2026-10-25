type Layer = { id: number; close: () => void };

const stack: Layer[] = [];
let next = 0;
let skip = 0;
let home = () => location.href;

// 層を剥ぎ　道標だけは　今のまま
function restore() {
  const url = new URL(home(), location.href).href;
  if (url !== location.href) history.replaceState(history.state, "", url);
}

// 戻る手に　重ねし層を　一枚ずつ
if (typeof window !== "undefined") {
  addEventListener("popstate", () => {
    if (skip > 0) {
      skip--;
      return restore();
    }
    const top = stack.pop();
    if (!top) return;
    restore();
    top.close();
  });
}

export function anchor(url: () => string): void {
  home = url;
}

function push(close: () => void): () => void {
  const id = ++next;
  stack.push({ id, close });
  history.pushState({ rf: id }, "", location.href);
  return () => {
    const i = stack.findIndex((l) => l.id === id);
    if (i < 0) return;
    stack.splice(i, 1);
    skip++;
    history.back();
  };
}

// 開く間は　戻る一押し　閉じる役
export function backable(open: () => boolean, close: () => void): void {
  const shown = $derived(open());
  $effect(() => {
    if (!shown) return;
    return push(close);
  });
}
