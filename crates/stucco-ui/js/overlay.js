// stucco overlay behaviour: fills the gaps in native dialogs and popovers.
// Everything is delegated from the document, so overlays inserted later by
// fragments work too.

document.documentElement.setAttribute("data-st-script", "");

const hasCommands = "command" in HTMLButtonElement.prototype;
const hasClosedBy = "closedBy" in HTMLDialogElement.prototype;

function dialogFor(id) {
  const dialog = id && document.getElementById(id);
  return dialog instanceof HTMLDialogElement ? dialog : null;
}

/** Whether a click on `link` is one the page may take over. */
function plainClick(event, link) {
  if (event.defaultPrevented || event.button !== 0) return false;
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return false;
  const target = link.getAttribute("target");
  return !target || target === "_self";
}

document.addEventListener("click", (event) => {
  const target = event.target instanceof Element ? event.target : null;
  if (!target) return;

  // A link opener opens its dialog in place instead of navigating, but only
  // for a plain primary click: a modified click (new tab or window, download)
  // or one another handler already took still follows the link.
  const link = target.closest("a[data-st-opens]");
  if (link) {
    if (!plainClick(event, link)) return;
    const dialog = dialogFor(link.getAttribute("data-st-opens"));
    if (dialog) {
      event.preventDefault();
      if (!dialog.open) dialog.showModal();
    }
    return;
  }

  // A toast's Dismiss button.
  const dismiss = target.closest("[data-st-dismiss]");
  if (dismiss) {
    dismiss.closest(".st-toast")?.remove();
    return;
  }

  // Invoker commands, for browsers without them.
  if (!hasCommands) {
    const button = target.closest("button[commandfor]");
    const dialog = button && dialogFor(button.getAttribute("commandfor"));
    if (dialog) {
      const command = button.getAttribute("command");
      if (command === "show-modal" && !dialog.open) dialog.showModal();
      else if (command === "close" || command === "request-close") dialog.close();
      return;
    }
  }

  // closedby="any" (a click outside closes), for browsers without it.
  if (!hasClosedBy && target instanceof HTMLDialogElement && target.open
      && target.getAttribute("closedby") === "any") {
    const r = target.getBoundingClientRect();
    const outside = event.clientX < r.left || event.clientX > r.right
      || event.clientY < r.top || event.clientY > r.bottom;
    if (outside) target.close();
  }
});

// Menus open under their button, flipping above it near the bottom of the
// screen, and stay inside the viewport.
function place(popover) {
  const invoker = document.querySelector(`[popovertarget="${CSS.escape(popover.id)}"]`);
  if (!invoker) return;
  const margin = 8;
  const gap = 4;
  popover.style.inset = "auto";
  popover.style.margin = "0";
  const anchor = invoker.getBoundingClientRect();
  const box = popover.getBoundingClientRect();
  const rtl = getComputedStyle(invoker).direction === "rtl";
  let left = rtl ? anchor.right - box.width : anchor.left;
  left = Math.max(margin, Math.min(left, innerWidth - box.width - margin));
  let top = anchor.bottom + gap;
  if (top + box.height > innerHeight - margin && anchor.top - gap - box.height > margin) {
    top = anchor.top - gap - box.height;
  }
  popover.style.top = `${top}px`;
  popover.style.left = `${left}px`;
}

const isMenu = (target) => target instanceof HTMLElement && target.matches(".st-menu-popover");

// "toggle" fires after the opened menu is first painted, where the browser
// centres popovers, so the menu would flash in the middle of the screen. An
// animation frame requested as it opens runs after it is shown and before
// that paint, so it appears in place.
document.addEventListener("beforetoggle", (event) => {
  if (isMenu(event.target) && event.newState === "open") {
    requestAnimationFrame(() => {
      if (event.target.matches(":popover-open")) place(event.target);
    });
  }
}, true);
document.addEventListener("toggle", (event) => {
  if (isMenu(event.target) && event.newState === "open") place(event.target);
}, true);

function placeOpenMenus() {
  for (const popover of document.querySelectorAll(".st-menu-popover")) {
    if (popover.matches(":popover-open")) place(popover);
  }
}
addEventListener("resize", placeOpenMenus);
addEventListener("scroll", placeOpenMenus, { passive: true, capture: true });

// Escape hides a tooltip until the pointer or focus leaves its control.
document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  for (const anchor of document.querySelectorAll(".st-tooltip-anchor")) {
    if (anchor.matches(":hover, :focus-within")) anchor.setAttribute("data-dismissed", "");
  }
});
for (const type of ["pointerout", "focusout"]) {
  document.addEventListener(type, (event) => {
    const anchor = event.target instanceof Element ? event.target.closest(".st-tooltip-anchor") : null;
    if (anchor && !anchor.contains(event.relatedTarget)) anchor.removeAttribute("data-dismissed");
  });
}
