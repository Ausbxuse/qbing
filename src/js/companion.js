(function() {
 'use strict';
 var last = 0;
 function update(state) {
  if (!state || state.updatedAt < last) return;
  last = state.updatedAt;
  var line = document.getElementById('companion-scramble');
  line.replaceChildren();
  if (state.segments && state.segments.length) state.segments.forEach(function(segment) { var span = document.createElement('span'); span.textContent = segment.text; span.className = ['done','current','correction'].indexOf(segment.kind) >= 0 ? segment.kind : ''; line.appendChild(span); });
  else line.textContent = state.scramble || 'Open csTimer to get a scramble';
  var dot = document.getElementById('companion-connection');
  dot.classList.toggle('connected', !!state.connected);
  dot.title = state.connected ? 'Connected: ' + (state.device || 'cube') : 'Open timer to connect cube';
  dot.setAttribute('aria-label', state.connected ? 'Cube connected' : 'Cube disconnected');
  document.getElementById('companion-time').textContent = state.time || '—';
  document.getElementById('companion-tps').textContent = typeof state.tps === 'number' ? state.tps.toFixed(2) : '—';
 }
 async function poll() { try { var response = await fetch('/api/companion/state', { cache: 'no-store' }); if (response.ok) update(await response.json()); } catch (_) {} }
 if (window.BroadcastChannel) { var channel = new BroadcastChannel('cstimer-companion'); channel.onmessage = function(e) { if (e.data.type === 'state') update(Object.assign({}, e.data.state, { online: true })); }; channel.postMessage({ type: 'request' }); }
 document.getElementById('companion-pin').onclick = function() { fetch('/api/companion/open', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: '{}' }); };
 document.getElementById('companion-connection').onclick = function() { window.open('/', 'cstimer-main'); };
 poll(); setInterval(poll, 500);
})();
