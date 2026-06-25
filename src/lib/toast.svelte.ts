/** 极简全局提示，避免引入额外依赖 */
type ToastKind = "ok" | "error" | "info";

interface ToastState {
  msg: string;
  kind: ToastKind;
  visible: boolean;
}

export const toast = $state<ToastState>({ msg: "", kind: "info", visible: false });

let timer: ReturnType<typeof setTimeout> | null = null;

export function showToast(msg: string, kind: ToastKind = "info") {
  toast.msg = msg;
  toast.kind = kind;
  toast.visible = true;
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => {
    toast.visible = false;
  }, 3200);
}
