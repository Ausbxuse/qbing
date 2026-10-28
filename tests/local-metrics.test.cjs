'use strict';
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const test = require('node:test');
const { calculateWindow } = require('../src/js/local-metrics.js');

function upstreamTimeStat(properties = {}) {
	const context = vm.createContext({
		execMain: fn => fn(),
		kernel: { getProp: (name, fallback) => properties[name] ?? fallback }
	});
	for (const file of ['src/js/lib/sbtree.js', 'src/js/stats/timestat.js']) {
		vm.runInContext(readFileSync(path.join(__dirname, '..', file), 'utf8'), context);
	}
	return context.TimeStat;
}

function recorded(duration, count, penalty = 0) {
	return { record: [[penalty, duration], 'R', '', 0, ['R\'@0', '333']], count };
}
function source(rows, properties) {
	return {
		TimeStat: upstreamTimeStat(properties),
		length: rows.length,
		timeAt: i => rows[i].record[0][0] < 0 ? -1 : rows[i].record[0][0] + rows[i].record[0][1],
		recordAt: i => rows[i].record,
		moveCount: i => rows[i].count
	};
}

test('uses actual upstream time trimming and only the newest window', () => {
	const rows = [recorded(99999, 999), ...[1000, 2000, 3000, 4000, 5000].map((ms, i) => recorded(ms, 10 * (i + 1)))];
	const average = calculateWindow(5, source(rows));
	assert.deepEqual(average.indices, [2, 3, 4]);
	assert.equal(average.time, 3000);
	assert.equal(average.moves, 30);
	assert.equal(average.tps, 10);
	assert.equal(average.retained, 3);
});

test('plus two changes retained time ranks but not execution TPS', () => {
	const rows = [recorded(1000, 10), recorded(1500, 15, 2000), recorded(2000, 20), recorded(3000, 30), recorded(4000, 40)];
	const average = calculateWindow(5, source(rows));
	assert.deepEqual(average.indices, [1, 2, 3]);
	assert.equal(average.tps, 10);
	assert.equal(average.moves, 65 / 3);
});

test('manual and unavailable reconstruction counts are excluded independently', () => {
	const rows = [1000, 2000, 3000, 4000, 5000].map(ms => recorded(ms, ms / 100));
	rows[1].record.splice(4); // A manual solve cannot gain counts from a metric cache.
	rows[2].count = -1;
	const average = calculateWindow(5, source(rows));
	assert.equal(average.retained, 3);
	assert.equal(average.moveSamples, 1);
	assert.equal(average.tpsSamples, 1);
	assert.equal(average.moves, 40);
	assert.equal(average.tps, 10);
});

test('valid counts survive unusable execution duration and retain independent coverage', () => {
	const rows = [recorded(1000, 10), recorded(2000, 20), recorded(0, 30, 3000), recorded(4000, 40), recorded(5000, 50)];
	const average = calculateWindow(5, source(rows));
	assert.equal(average.moveSamples, 3);
	assert.equal(average.tpsSamples, 2);
	assert.equal(average.moves, 30);
	assert.equal(average.tps, 10);
});

test('DNF trimming agrees with upstream and retained DNFs do not enter sensor means', () => {
	const rows = [recorded(1000, 10), recorded(2000, 20), recorded(3000, 30), recorded(4000, 400, -1), recorded(5000, 500, -1)];
	const average = calculateWindow(5, source(rows));
	assert.equal(average.time, -1);
	assert.equal(average.retained, 3);
	assert.equal(average.moveSamples, 2);
	assert.equal(average.tpsSamples, 2);
	assert.equal(average.moves, 25);
	assert.equal(average.tps, 10);
});

test('uses time ranks even when history sorts another metric; honors custom trimming', () => {
	const rows = Array.from({ length: 12 }, (_, i) => recorded((i + 1) * 1000, 120 - i * 10));
	const api = source(rows, { trim: '2', trimr: 'a', statsrc: 'mmvcnt_htm' });
	api.getTimesStatsTable = () => { throw new Error('selected metric must not be consulted'); };
	const average = calculateWindow(12, api);
	assert.deepEqual(average.indices, [2, 3, 4, 5, 6, 7, 8, 9]);
	assert.equal(average.moves, 65);
	assert.equal(average.time, 6500);
});

test('insufficient windows stay absent and empty metric samples stay null', () => {
	assert.equal(calculateWindow(5, source([recorded(1000, 10)])), null);
	const rows = [1000, 2000, 3000, 4000, 5000].map(ms => recorded(ms, -1));
	const average = calculateWindow(5, source(rows));
	assert.equal(average.moves, null);
	assert.equal(average.tps, null);
	assert.equal(average.moveSamples, 0);
});
