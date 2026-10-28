/* Anchored action menu shared by the app and its isolated interface study. */
var LocalActionMenu = (function() {
 'use strict';
 function create(menu, triggers, beforeOpen) {
  var anchor = null;
  menu.setAttribute('role', 'menu');
  var items = function() { return Array.prototype.filter.call(menu.querySelectorAll('button'), function(button) { return !button.disabled && !button.hidden; }); };
  menu.querySelectorAll('button').forEach(function(button) { button.setAttribute('role', 'menuitem'); button.tabIndex = -1; });
  function place() {
   if (menu.hidden || !anchor) return;
   menu.style.maxHeight = Math.max(80, window.innerHeight - 16) + 'px';
   menu.style.overflowY = 'auto';
   var rect = anchor.getBoundingClientRect(), box = menu.getBoundingClientRect();
   menu.style.left = Math.max(8, Math.min(window.innerWidth - box.width - 8, rect.right - box.width)) + 'px';
   menu.style.right = 'auto';
   var below = rect.bottom + 6;
   menu.style.top = (below + box.height <= window.innerHeight - 8 ? below : Math.max(8, rect.top - box.height - 6)) + 'px';
  }
  function close(restoreFocus) {
   if (menu.hidden) return;
   menu.hidden = true;
   if (anchor) { anchor.setAttribute('aria-expanded', 'false'); if (restoreFocus) anchor.focus(); }
  }
  function open(trigger, last) {
   if (beforeOpen && beforeOpen(trigger) === false) return;
   if (anchor) anchor.setAttribute('aria-expanded', 'false');
   anchor = trigger; menu.hidden = false; anchor.setAttribute('aria-expanded', 'true'); place();
   var enabled = items(); if (enabled.length) enabled[last ? enabled.length - 1 : 0].focus({ preventScroll: true });
  }
  function toggle(trigger) { if (!menu.hidden && anchor === trigger) close(true); else open(trigger); }
  triggers.forEach(function(trigger) {
   trigger.setAttribute('aria-haspopup', 'menu'); trigger.setAttribute('aria-expanded', 'false'); trigger.setAttribute('aria-controls', menu.id);
   trigger.addEventListener('keydown', function(event) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') { event.preventDefault(); event.stopPropagation(); open(trigger, event.key === 'ArrowUp'); }
   });
  });
  menu.addEventListener('keydown', function(event) {
   event.stopPropagation();
   var enabled = items(), index = enabled.indexOf(document.activeElement), next;
   if (event.key === 'Tab') { close(true); return; }
   if (event.key === 'ArrowDown') next = (index + 1) % enabled.length;
   else if (event.key === 'ArrowUp') next = (index + enabled.length - 1) % enabled.length;
   else if (event.key === 'Home') next = 0;
   else if (event.key === 'End') next = enabled.length - 1;
   if (next != null && enabled[next]) { event.preventDefault(); enabled[next].focus({ preventScroll: true }); }
  });
  menu.addEventListener('keyup', function(event) { event.stopPropagation(); });
  menu.addEventListener('click', function(event) { if (event.target.closest('button:not(:disabled)')) close(false); });
  document.addEventListener('keydown', function(event) {
   if (!menu.hidden && event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); close(true); }
  }, true);
  document.addEventListener('click', function(event) {
   if (!menu.hidden && !menu.contains(event.target) && !triggers.some(function(trigger) { return trigger.contains(event.target); })) close(false);
  });
  window.addEventListener('resize', place);
  window.addEventListener('scroll', function(event) { if (!menu.hidden && !(event.target instanceof Node && menu.contains(event.target))) close(false); }, true);
  return { open: open, close: close, toggle: toggle };
 }
 return { create: create };
})();
