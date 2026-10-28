const { test } = require('node:test');
const assert = require('node:assert/strict');
const http = require('node:http');
async function fixture(t) {
 const { createCompanionBridge } = await import('../scripts/companion-bridge.mjs');
 let now = 10000, opens = 0;
 const bridge = createCompanionBridge({ now: () => now, openNative: async () => { opens++; } });
 const server = http.createServer((req, res) => bridge.handle(req, res, new URL(req.url, 'http://localhost').pathname));
 await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
 t.after(() => new Promise(resolve => { server.closeAllConnections(); server.close(resolve); }));
 const url = 'http://127.0.0.1:' + server.address().port;
 return { url, get: async () => (await fetch(url + '/api/companion/state')).json(),
  post: (data, origin = url, endpoint = 'state') => fetch(url + '/api/companion/' + endpoint, { method: 'POST', headers: { 'Content-Type': 'application/json', Origin: origin }, body: JSON.stringify(data) }),
  advance: ms => { now += ms; }, opens: () => opens };
}
const snapshot = { owner: 'timer-one', connected: true, device: 'Test cube', scramble: 'R2 U F', time: '12.44', tps: 4.18, moves: 52, segments: [{ text: 'R2 ', kind: 'done' }, { text: 'U ', kind: 'current' }] };
test('publishes actual live fields, retaining last solve while stale connection becomes offline', async t => {
 const f = await fixture(t);
 assert.equal((await f.get()).online, false);
 assert.equal((await f.post(snapshot)).status, 200);
 let state = await f.get();
 assert.equal(state.connected, true); assert.equal(state.time, '12.44'); assert.equal(state.tps, 4.18);
 assert.deepEqual(state.segments, snapshot.segments);
 f.advance(5001); state = await f.get();
 assert.equal(state.online, false); assert.equal(state.connected, false); assert.equal(state.time, '12.44');
});
test('requires a same-origin browser request before accepting state or opening native window', async t => {
 const f = await fixture(t);
 assert.equal((await f.post(snapshot, 'https://unrelated.example')).status, 403);
 assert.equal((await f.post({}, 'https://unrelated.example', 'open')).status, 403); assert.equal(f.opens(), 0);
 assert.equal((await f.post({}, f.url, 'open')).status, 200); assert.equal(f.opens(), 1);
 assert.equal((await f.get()).online, false);
});
test('one active owner prevents competing timer windows from replacing live cube state', async t => {
 const f = await fixture(t);
 await f.post(snapshot);
 assert.equal((await f.post({ ...snapshot, owner: 'other-timer', time: '99.99' })).status, 409);
 assert.equal((await f.get()).time, '12.44');
 f.advance(3001);
 assert.equal((await f.post({ ...snapshot, owner: 'other-timer' })).status, 200);
});
test('invalid samples cannot create numeric TPS or arbitrary segment styles', async t => {
 const f = await fixture(t);
 assert.equal((await f.post({})).status, 400);
 assert.equal((await f.post({ ...snapshot, tps: -1, moves: null, segments: [{ text: 'R', kind: 'untrusted' }] })).status, 200);
 const state = await f.get(); assert.equal(state.tps, null); assert.equal(state.moves, null); assert.equal(state.segments[0].kind, '');
});
