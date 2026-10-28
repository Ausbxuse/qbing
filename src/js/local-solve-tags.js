/* Record-attached labels; comments and upstream scores are never rewritten. */
var LocalSolveTags = (function() {
 'use strict';
 var colors = ['', 'blue', 'mint', 'amber', 'rose'];
 var groups = ['', 'review', 'practice', 'milestone'];
 function read(record) {
  var value = record && record[5] && record[5].localReview || {};
  return { favorite: value.favorite === true, color: colors.indexOf(value.color) >= 0 ? value.color : '', group: groups.indexOf(value.group) >= 0 ? value.group : '' };
 }
 function words(record) { var t = read(record); return [t.favorite ? 'favorite' : '', t.color, t.group].join(' '); }
 var api = { read: read, words: words, colors: colors, groups: groups };
 if (typeof module !== 'undefined' && module.exports) module.exports = api;
 return api;
})();
