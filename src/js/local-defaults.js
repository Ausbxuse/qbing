/* First-run preferences, plus a one-time default for unspecified typography.
   Explicit existing settings and sessions win. Runs before csTimer loads. */
(function () {
  'use strict';
  try {
    var saved = localStorage.getItem('properties');
    if (saved) {
      var properties = JSON.parse(saved);
      if (properties && typeof properties === 'object' && !Array.isArray(properties) &&
          !(properties.localTypographyVersion >= 2)) {
        if (properties.localAppearance !== false &&
            !Object.prototype.hasOwnProperty.call(properties, 'smallADP')) {
          properties.smallADP = false;
        }
        properties.localTypographyVersion = 2;
        localStorage.setItem('properties', JSON.stringify(properties));
      }
      return;
    }
    localStorage.setItem('properties', JSON.stringify({
      color: 'u',
      'col-font': '#deddec', 'col-back': '#1e2233',
      'col-board': '#222233', 'col-button': '#343a52',
      'col-link': '#b3addf', 'col-logo': '#deddec',
      'col-logoback': '#1e2233', 'col-pbs': '#a8c891',
      font: 'Arial', timerSize: 12, uidesign: 'mtns', localAppearance: true, localMetrics: true,
      smallADP: false, localTypographyVersion: 2,
      input: 't', showAvg: true,
      giiVRC: 'v', vrcMP: 'cfop', giiSD: 's', giiSK: true,
      giiSM: 'n', giiMode: 'n', giiRST: 'p', giiAED: false,
      NTools: 2, toolPos: 'f',
      toolsfunc: JSON.stringify(['giikerutil', 'recons', 'casestat', 'trainstat']),
      tools: true, stats: true, scramble: true
    }));
  } catch (error) {
    console.warn('Could not apply local csTimer defaults:', error);
  }
})();
