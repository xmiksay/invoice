/** Full page load to another host (the base hub, another space). A module so tests can stub it. */
export function leaveTo(url: string): void {
  window.location.assign(url);
}
