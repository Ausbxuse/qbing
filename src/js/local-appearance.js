/* Optional typography for the local fork. csTimer owns layout and colors. */
(function () {
	'use strict';

	function applyAppearance() {
		$('body').toggleClass('local-appearance', kernel.getProp('localAppearance') === true)
			.toggleClass('local-mono-scramble', kernel.getProp('scrMono') === true);
		var size = Number(kernel.getProp('timerSize')) || 12;
		var scrambleSize = Number(kernel.getProp('scrSize')) || 15;
		var zoom = Number(kernel.getProp('zoom')) || 1;
		var style = document.body.style;
		style.setProperty('--local-clock-size', size * 8 * zoom + 'px');
		style.setProperty('--local-clock-phone-size', size * 4 * zoom + 'px');
		style.setProperty('--local-scramble-size', scrambleSize * 2 * zoom + 'px');
		style.setProperty('--local-scramble-phone-size', scrambleSize * 1.6 * zoom + 'px');
		style.setProperty('--local-average-size', 16 * zoom + 'px');
		style.setProperty('--local-text-size', 14 * zoom + 'px');
	}

	$(function () {
		kernel.regListener('localAppearance', 'property', applyAppearance, /^(?:localAppearance|timerSize|scrSize|scrMono|zoom)$/);
		kernel.regProp('ui', 'localTypographyVersion', ~5, 'Local typography version', [0]);
		kernel.regProp('ui', 'localAppearance', 0, 'Compact typography', [true]);
		applyAppearance();
	});
})();
