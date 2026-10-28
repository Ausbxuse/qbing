/* Additional sensor metrics for csTimer's time-ranked average windows. */
var localAverageMetrics = (function() {
	'use strict';

	function finite(value) {
		return typeof value == 'number' && isFinite(value);
	}

	// Keep time ranking, tie breaking and the user's trim settings in TimeStat.
	// The currently selected history metric must not change these samples.
	function calculateWindow(size, source) {
		if (source.length < size) {
			return null;
		}
		var start = source.length - size;
		var timeStats = new source.TimeStat([], source.length, source.timeAt);
		var windows = timeStats.runAvgMean(start, size);
		if (!windows || !Array.isArray(windows[0])) {
			return null;
		}
		var window = windows[0];
		var trimmed = timeStats.getTrimList(start, size, window[2], window[3]);
		var indices = [];
		var countSum = 0, countSamples = 0, tpsSum = 0, tpsSamples = 0;
		for (var i = 0; i < size; i++) {
			if (trimmed.indexOf(i) != -1) {
				continue;
			}
			var index = start + i;
			indices.push(index);
			var score = source.timeAt(index);
			if (!finite(score) || score < 0) {
				continue;
			}
			var record = source.recordAt(index);
			// A manual time must never acquire a synthetic turn count.
			if (!record || !record[4] || !record[4][0]) {
				continue;
			}
			var count = source.moveCount(index);
			if (!finite(count) || count < 0) {
				continue;
			}
			countSum += count;
			countSamples++;
			var duration = record[0] && record[0][1];
			if (finite(duration) && duration > 0) {
				tpsSum += count * 1000 / duration;
				tpsSamples++;
			}
		}
		return {
			size: size,
			time: window[0],
			indices: indices,
			retained: indices.length,
			tps: tpsSamples ? tpsSum / tpsSamples : null,
			moves: countSamples ? countSum / countSamples : null,
			tpsSamples: tpsSamples,
			moveSamples: countSamples
		};
	}

	function install() {
		var panel = $('<div id="local-average-metrics" aria-label="Average turn metrics">').appendTo('#avgstr');
		var updateFrame;
		function scheduleUpdate() {
			if (!updateFrame) updateFrame = requestAnimationFrame(function() { updateFrame = null; update(); });
		}
		function update() {
			panel.empty();
			if (document.body.classList.contains('local-interface')) { panel.hide(); return; }
			if (!kernel.getProp('localMetrics', true)) {
				panel.hide();
				return;
			}
			var source = {
				TimeStat: TimeStat,
				length: stats.getTimesStatsList().timesLen,
				timeAt: stats.timeAt,
				recordAt: stats.timesAt,
				moveCount: function(index) { return stats.getExtraInfo('mvcnt_htm', index); }
			};
			[5, 12].forEach(function(size) {
				var average = calculateWindow(size, source);
				if (!average) {
					return;
				}
				var speedText = (average.tps == null ? '—' : average.tps.toFixed(2)) + ' tps';
				var movesText = (average.moves == null ? '—' : average.moves.toFixed(1)) + ' moves';
				var description = 'ao' + size + ': ' + speedText + ', ' + movesText + '. Time-ranked retained solves. ' +
					'TPS from ' + average.tpsSamples + '/' + average.retained + '; ' +
					'moves from ' + average.moveSamples + '/' + average.retained + '. ' +
					'Counts use csTimer HTM reconstruction. +2 affects ranking, not execution TPS.';
				$('<div class="local-average-row">')
					.attr('title', description)
					.attr('aria-label', description)
					.append($('<span class="local-average-name">').text('ao' + size),
						$('<span>').text(speedText),
						$('<span>').text(movesText))
					.appendTo(panel);
			});
			panel.toggle(panel.children().length > 0);
		}
		kernel.regProp('stats', 'localMetrics', 0, 'Average TPS and moves', [true]);
		stats.regUtil('localMetrics', scheduleUpdate);
		kernel.regListener('localMetrics', 'property', scheduleUpdate, /^(localMetrics|localInterface|trim|trimr)$/);
		scheduleUpdate();
	}

	var api = { calculateWindow: calculateWindow };
	if (typeof module != 'undefined' && module.exports) {
		module.exports = api;
	} else {
		$(install);
	}
	return api;
})();
