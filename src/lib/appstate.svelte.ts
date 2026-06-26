/** 跨组件共享的运行时应用状态（不持久化）。 */
export const appstate = $state({
  blockRemoteImages: true,
  autoLockMins: 30,
});
