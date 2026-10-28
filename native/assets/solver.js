if(!Array.prototype.at)Object.defineProperty(Array.prototype,"at",{value:function(i){i=Math.trunc(i)||0;return this[i<0?this.length+i:i];}});var DEBUG=false, ISCSTIMER=false; var $={isArray:Array.isArray,noop:function(){}}; var console={log:function(){}};
/* ----------------------------------------------------------------------
 * Copyright (c) 2012 Yves-Marie K. Rinquin
 *
 * Permission is hereby granted, free of charge, to any person obtaining
 * a copy of this software and associated documentation files (the
 * "Software"), to deal in the Software without restriction, including
 * without limitation the rights to use, copy, modify, merge, publish,
 * distribute, sublicense, and/or sell copies of the Software, and to
 * permit persons to whom the Software is furnished to do so, subject to
 * the following conditions:
 *
 * The above copyright notice and this permission notice shall be
 * included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
 * MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
 * NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
 * LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
 * OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
 * WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 *
 * ----------------------------------------------------------------------
 *
 * ISAAC is a cryptographically secure pseudo-random number generator
 * (or CSPRNG for short) designed by Robert J. Jenkins Jr. in 1996 and
 * based on RC4. It is designed for speed and security.
 *
 * ISAAC's informations & analysis:
 *   http://burtleburtle.net/bob/rand/isaac.html
 * ISAAC's implementation details:
 *   http://burtleburtle.net/bob/rand/isaacafa.html
 *
 * ISAAC succesfully passed TestU01
 *
 * ----------------------------------------------------------------------
 *
 * Usage:
 *   <script src="isaac.js"></script>
 *   var random_number = isaac.random();
 *
 * Output: [ 0x00000000; 0xffffffff]
 *         [-2147483648; 2147483647]
 *
 */

/* isaac module pattern */
var isaac = (function(){

	/* private: internal states */
	var m = Array(256), // internal memory
		acc = 0,        // accumulator
		brs = 0,        // last result
		cnt = 0,        // counter
		r = Array(256), // result array
		gnt = 0;        // generation counter

	seed(Math.random() * 0xffffffff);

	/* private: 32-bit integer safe adder */
	function add(x, y) {
		var lsb = (x & 0xffff) + (y & 0xffff);
		var msb = (x >>>   16) + (y >>>   16) + (lsb >>> 16);
		return (msb << 16) | (lsb & 0xffff);
	}

	/* public: initialisation */
	function reset() {
		acc = brs = cnt = 0;
		for(var i = 0; i < 256; ++i)
			m[i] = r[i] = 0;
		gnt = 0;
	}

	/* public: seeding function */
	function seed(s) {
		var a, b, c, d, e, f, g, h, i;

		/* seeding the seeds of love */
		a = b = c = d =
		e = f = g = h = 0x9e3779b9; /* the golden ratio */

		if(s && typeof(s) === 'number') {
			s = [s];
		}

		if(s instanceof Array) {
			reset();
			for(i = 0; i < s.length; i++)
				r[i & 0xff] += (typeof(s[i]) === 'number') ? s[i] : 0;
		}

		/* private: seed mixer */
		function seed_mix() {
			a ^= b <<  11; d = add(d, a); b = add(b, c);
			b ^= c >>>  2; e = add(e, b); c = add(c, d);
			c ^= d <<   8; f = add(f, c); d = add(d, e);
			d ^= e >>> 16; g = add(g, d); e = add(e, f);
			e ^= f <<  10; h = add(h, e); f = add(f, g);
			f ^= g >>>  4; a = add(a, f); g = add(g, h);
			g ^= h <<   8; b = add(b, g); h = add(h, a);
			h ^= a >>>  9; c = add(c, h); a = add(a, b);
		}

		for(i = 0; i < 4; i++) /* scramble it */
			seed_mix();

		for(i = 0; i < 256; i += 8) {
			if(s) { /* use all the information in the seed */
				a = add(a, r[i + 0]); b = add(b, r[i + 1]);
				c = add(c, r[i + 2]); d = add(d, r[i + 3]);
				e = add(e, r[i + 4]); f = add(f, r[i + 5]);
				g = add(g, r[i + 6]); h = add(h, r[i + 7]);
			}
			seed_mix();
			/* fill in m[] with messy stuff */
			m[i + 0] = a; m[i + 1] = b; m[i + 2] = c; m[i + 3] = d;
			m[i + 4] = e; m[i + 5] = f; m[i + 6] = g; m[i + 7] = h;
		}
		if(s) {
			/* do a second pass to make all of the seed affect all of m[] */
			for(i = 0; i < 256; i += 8) {
				a = add(a, m[i + 0]); b = add(b, m[i + 1]);
				c = add(c, m[i + 2]); d = add(d, m[i + 3]);
				e = add(e, m[i + 4]); f = add(f, m[i + 5]);
				g = add(g, m[i + 6]); h = add(h, m[i + 7]);
				seed_mix();
				/* fill in m[] with messy stuff (again) */
				m[i + 0] = a; m[i + 1] = b; m[i + 2] = c; m[i + 3] = d;
				m[i + 4] = e; m[i + 5] = f; m[i + 6] = g; m[i + 7] = h;
			}
		}

		prng(); /* fill in the first set of results */
		gnt = 256;  /* prepare to use the first set of results */;
	}

	/* public: isaac generator, n = number of run */
	function prng(n){
		var i, x, y;

		n = (n && typeof(n) === 'number')
			? Math.abs(Math.floor(n)) : 1;

		while(n--) {
			cnt = add(cnt,   1);
			brs = add(brs, cnt);

			for(i = 0; i < 256; i++) {
				switch(i & 3) {
					case 0: acc ^= acc <<  13; break;
					case 1: acc ^= acc >>>  6; break;
					case 2: acc ^= acc <<   2; break;
					case 3: acc ^= acc >>> 16; break;
				}
				acc        = add(m[(i +  128) & 0xff], acc); x = m[i];
				m[i] =   y = add(m[(x >>>  2) & 0xff], add(acc, brs));
				r[i] = brs = add(m[(y >>> 10) & 0xff], x);
			}
		}
	}

	/* public: return a random number between */
	function rand() {
		if(!gnt--) {
			prng(); gnt = 255;
		}
		return r[gnt];
	}

	function random() {
		return ((rand() >>> 5) * 0x4000000 + (rand() >>> 6)) / 0x20000000000000;
	}

	/* public: return internals in an object*/
	function internals(obj) {
		var ret = {a: acc, b: brs, c: cnt, m: m.slice(), r: r.slice(), g: gnt};
		if (obj) {
			acc = obj.a; brs = obj.b; cnt = obj.c; m = obj.m.slice(); r = obj.r.slice(); gnt = obj.g;
		}
		return ret;
	}

	/* return class object */
	return {
		reset: reset,
		seed:  seed,
		prng:  prng,
		rand:  rand,
		random: random,
		internals: internals
	};
})(); /* declare and execute */

"use strict";

var mathlib = (function() {
	var Cnk = [],
		fact = [1];
	for (var i = 0; i < 32; ++i) {
		Cnk[i] = [];
		for (var j = 0; j < 32; ++j) {
			Cnk[i][j] = 0;
		}
	}
	for (var i = 0; i < 32; ++i) {
		Cnk[i][0] = Cnk[i][i] = 1;
		fact[i + 1] = fact[i] * (i + 1);
		for (var j = 1; j < i; ++j) {
			Cnk[i][j] = Cnk[i - 1][j - 1] + Cnk[i - 1][j];
		}
	}

	var permMul4 = [];
	for (var i = 0; i < 24; i++) {
		var perm1 = [];
		var perm2 = [];
		var perm3 = [];
		permMul4[i] = [];
		setNPerm(perm1, i, 4);
		for (var j = 0; j < 24; j++) {
			setNPerm(perm2, j, 4);
			for (var k = 0; k < 4; k++) {
				perm3[k] = perm1[perm2[k]];
			}
			permMul4[i][j] = getNPerm(perm3, 4);
		}
	}

	function circleOri(arr, a, b, c, d, ori) {
		var temp = arr[a];
		arr[a] = arr[d] ^ ori;
		arr[d] = arr[c] ^ ori;
		arr[c] = arr[b] ^ ori;
		arr[b] = temp ^ ori;
	}

	function circle(arr) {
		var length = arguments.length - 1,
			temp = arr[arguments[length]];
		for (var i = length; i > 1; i--) {
			arr[arguments[i]] = arr[arguments[i - 1]];
		}
		arr[arguments[1]] = temp;
		return circle;
	}

	//perm: [idx1, idx2, ..., idxn]
	//pow: 1, 2, 3, ...
	//ori: ori1, ori2, ..., orin, base
	// arr[perm[idx2]] = arr[perm[idx1]] + ori[idx2] - ori[idx1] + base
	function acycle(arr, perm, pow, ori) {
		pow = pow || 1;
		var plen = perm.length;
		var tmp = [];
		for (var i = 0; i < plen; i++) {
			tmp[i] = arr[perm[i]];
		}
		for (var i = 0; i < plen; i++) {
			var j = (i + pow) % plen;
			arr[perm[j]] = tmp[i];
			if (ori) {
				arr[perm[j]] += ori[j] - ori[i] + ori.at(-1);
			}
		}
		return acycle;
	}

	function getPruning(table, index) {
		return table[index >> 3] >> ((index & 7) << 2) & 15;
	}

	function setNPerm(arr, idx, n, even) {
		var prt = 0;
		if (even < 0) {
			idx <<= 1;
		}
		if (n >= 16) {
			arr[n - 1] = 0;
			for (var i = n - 2; i >= 0; i--) {
				arr[i] = idx % (n - i);
				prt ^= arr[i];
				idx = ~~(idx / (n - i));
				for (var j = i + 1; j < n; j++) {
					arr[j] >= arr[i] && arr[j]++;
				}
			}
			if (even < 0 && (prt & 1) != 0) {
				var tmp = arr[n - 1];
				arr[n - 1] = arr[n - 2];
				arr[n - 2] = tmp;
			}
			return arr;
		}
		var vall = 0x76543210;
		var valh = 0xfedcba98;
		for (var i = 0; i < n - 1; i++) {
			var p = fact[n - 1 - i];
			var v = idx / p;
			idx = idx % p;
			prt ^= v;
			v <<= 2;
			if (v >= 32) {
				v = v - 32;
				arr[i] = valh >> v & 0xf;
				var m = (1 << v) - 1;
				valh = (valh & m) + ((valh >> 4) & ~m);
			} else {
				arr[i] = vall >> v & 0xf;
				var m = (1 << v) - 1;
				vall = (vall & m) + ((vall >>> 4) & ~m) + (valh << 28);
				valh = valh >> 4;
			}
		}
		if (even < 0 && (prt & 1) != 0) {
			arr[n - 1] = arr[n - 2];
			arr[n - 2] = vall & 0xf;
		} else {
			arr[n - 1] = vall & 0xf;
		}
		return arr;
	}

	function getNPerm(arr, n, even) {
		n = n || arr.length;
		var idx = 0;
		if (n >= 16) {
			for (var i = 0; i < n - 1; i++) {
				idx *= n - i;
				for (var j = i + 1; j < n; j++) {
					arr[j] < arr[i] && idx++;
				}
			}
			return even < 0 ? (idx >> 1) : idx;
		}
		var vall = 0x76543210;
		var valh = 0xfedcba98;
		for (var i = 0; i < n - 1; i++) {
			var v = arr[i] << 2;
			idx *= n - i;
			if (v >= 32) {
				idx += (valh >> (v - 32)) & 0xf;
				valh -= 0x11111110 << (v - 32);
			} else {
				idx += (vall >> v) & 0xf;
				valh -= 0x11111111;
				vall -= 0x11111110 << v;
			}
		}
		return even < 0 ? (idx >> 1) : idx;
	}

	function getNParity(idx, n) {
		var i, p;
		p = 0;
		for (i = n - 2; i >= 0; --i) {
			p ^= idx % (n - i);
			idx = ~~(idx / (n - i));
		}
		return p & 1;
	}

	function getNOri(arr, n, evenbase) {
		var base = Math.abs(evenbase);
		var idx = evenbase < 0 ? 0 : arr[0] % base;
		for (var i = n - 1; i > 0; i--) {
			idx = idx * base + arr[i] % base;
		}
		return idx;
	}

	function setNOri(arr, idx, n, evenbase) {
		var base = Math.abs(evenbase);
		var parity = base * n;
		for (var i = 1; i < n; i++) {
			arr[i] = idx % base;
			parity -= arr[i];
			idx = ~~(idx / base);
		}
		arr[0] = (evenbase < 0 ? parity : idx) % base;
		return arr;
	}

	function bitCount(x) {
		x -= (x >> 1) & 0x55555555;
		x = (x & 0x33333333) + ((x >> 2) & 0x33333333);
		return ((x + (x >> 4) & 0xf0f0f0f) * 0x1010101) >> 24;
	}

	function getMPerm(arr, n, cnts, cums) {
		var seen = ~0;
		var idx = 0;
		var x = 1;
		for (var i = 0; i < n; i++) {
			var pi = arr[i];
			idx = idx * (n - i) + bitCount(seen & ((1 << cums[pi]) - 1)) * x;
			x = x * cnts[pi]--;
			seen &= ~(1 << (cums[pi] + cnts[pi]));
		}
		return Math.round(idx / x);
	}

	function setMPerm(arr, idx, n, cnts, x) {
		for (var i = 0; i < n; i++) {
			for (var j = 0; j < cnts.length; j++) {
				if (cnts[j] == 0) {
					continue;
				}
				var x2 = ~~(x * cnts[j] / (n - i));
				if (idx < x2) {
					cnts[j]--;
					arr[i] = j;
					x = x2;
					break;
				}
				idx -= x2;
			}
		}
		return arr;
	}

	// type: 'p' (permutation), 'o' (orientation), 'c' (combination)
	// evenbase: base for ori, sign for even parity, cnts for combination
	function Coord(type, length, evenbase) {
		this.length = length;
		this.evenbase = evenbase;
		if (type == 'p') {
			this.get = function(arr) {
				return getNPerm(arr, this.length, this.evenbase);
			};
			this.set = function(arr, idx) {
				return setNPerm(arr, idx, this.length, this.evenbase);
			};
		} else if (type == 'o') {
			this.get = function(arr) {
				return getNOri(arr, this.length, this.evenbase);
			};
			this.set = function(arr, idx) {
				return setNOri(arr, idx, this.length, this.evenbase);
			};
		} else if (type == 'c') {
			var cnts = evenbase;
			this.cnts = cnts.slice();
			this.cntn = this.cnts.length;
			this.cums = [0];
			for (var i = 1; i <= this.cntn; i++) {
				this.cums[i] = this.cums[i - 1] + cnts[i - 1];
			}
			this.n = this.cums[this.cntn];
			var n = this.n;
			var x = 1;
			for (var i = 0; i < this.cntn; i++) {
				for (var j = 1; j <= cnts[i]; j++, n--) {
					x *= n / j;
				}
			}
			this.x = Math.round(x);
			this.get = function(arr) {
				return getMPerm(arr, this.n, this.cnts.slice(), this.cums);
			};
			this.set = function(arr, idx) {
				return setMPerm(arr, idx, this.n, this.cnts.slice(), this.x);
			};
		} else { // invalid type
			debugger;
		}
	}

	function fillFacelet(facelets, f, perm, ori, divcol) {
		for (var i = 0; i < facelets.length; i++) {
			var cubie = facelets[i];
			var p = perm[i] === undefined ? i : perm[i];
			if (typeof(cubie) == 'number') {
				f[cubie] = ~~(facelets[p] / divcol);
				continue;
			}
			var o = ori[i] || 0;
			for (var j = 0; j < cubie.length; j++) {
				f[cubie[(j + o) % cubie.length]] = ~~(facelets[p][j] / divcol);
			}
		}
	}

	function detectFacelet(facelets, f, perm, ori, divcol) {
		for (var i = 0; i < facelets.length; i++) {
			var n_ori = facelets[i].length;
			out: for (var j = 0; j < facelets.length + 1; j++) {
				if (j == facelets.length) { // not matched
					return -1;
				} else if (facelets[j].length != n_ori) {
					continue;
				}
				for (var o = 0; o < n_ori; o++) {
					var isMatch = true;
					for (var t = 0; t < n_ori; t++) {
						if (~~(facelets[j][t] / divcol) != f[facelets[i][(t + o) % n_ori]]) {
							isMatch = false;
							break;
						}
					}
					if (isMatch) {
						perm[i] = j;
						ori[i] = o;
						break out;
					}
				}
			}
		}
		return 0;
	}

	function createMove(moveTable, size, doMove, N_MOVES) {
		N_MOVES = N_MOVES || 6;
		if ($.isArray(doMove)) {
			var cord = new Coord(doMove[1], doMove[2], doMove[3]);
			doMove = doMove[0];
			for (var j = 0; j < N_MOVES; j++) {
				moveTable[j] = [];
				for (var i = 0; i < size; i++) {
					var arr = cord.set([], i);
					doMove(arr, j);
					moveTable[j][i] = cord.get(arr);
				}
			}
		} else {
			for (var j = 0; j < N_MOVES; j++) {
				moveTable[j] = [];
				for (var i = 0; i < size; i++) {
					moveTable[j][i] = doMove(i, j);
				}
			}
		}
	}

	function createMoveHash(initState, validMoves, hashFunc, moveFunc) {
		var states = [initState];
		var hash2idx = {};
		var depthEnds = [];
		hash2idx[hashFunc(initState)] = 0;
		depthEnds[0] = 1;
		var moveTable = [];
		for (var m = 0; m < validMoves.length; m++) {
			moveTable[m] = [];
		}
		var tt = +new Date;
		for (var i = 0; i < states.length; i++) {
			if (i == depthEnds.at(-1)) {
				depthEnds.push(states.length);
			}
			if (i % 10000 == 9999) {
				DEBUG && console.log(i, 'states scanned, tt=', +new Date - tt);
			}
			var curState = states[i];
			for (var m = 0; m < validMoves.length; m++) {
				var newState = moveFunc(curState, validMoves[m]);
				if (!newState) {
					moveTable[m][i] = -1;
					continue;
				}
				var newHash = hashFunc(newState);
				if (!(newHash in hash2idx)) {
					hash2idx[newHash] = states.length;
					states.push(newState);
				}
				moveTable[m][i] = hash2idx[newHash];
			}
		}
		DEBUG && console.log('[move hash] ' + states.length + ' states generated, tt=', +new Date - tt, JSON.stringify(depthEnds));
		return [moveTable, hash2idx];
	}

	function edgeMove(arr, m) {
		if (m == 0) { //F
			circleOri(arr, 0, 7, 8, 4, 1);
		} else if (m == 1) { //R
			circleOri(arr, 3, 6, 11, 7, 0);
		} else if (m == 2) { //U
			circleOri(arr, 0, 1, 2, 3, 0);
		} else if (m == 3) { //B
			circleOri(arr, 2, 5, 10, 6, 1);
		} else if (m == 4) { //L
			circleOri(arr, 1, 4, 9, 5, 0);
		} else if (m == 5) { //D
			circleOri(arr, 11, 10, 9, 8, 0);
		}
	}

	function CubieCube() {
		this.ca = [0, 1, 2, 3, 4, 5, 6, 7];
		this.ea = [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22];
		this.ori = 0;
	}

	CubieCube.SOLVED = new CubieCube();

	CubieCube.EdgeMult = function(a, b, prod) {
		for (var ed = 0; ed < 12; ed++) {
			prod.ea[ed] = a.ea[b.ea[ed] >> 1] ^ (b.ea[ed] & 1);
		}
	};

	CubieCube.CornMult = function(a, b, prod) {
		for (var corn = 0; corn < 8; corn++) {
			var ori = ((a.ca[b.ca[corn] & 7] >> 3) + (b.ca[corn] >> 3)) % 3;
			prod.ca[corn] = a.ca[b.ca[corn] & 7] & 7 | ori << 3;
		}
	};

	CubieCube.CubeMult = function(a, b, prod) {
		CubieCube.CornMult(a, b, prod);
		CubieCube.EdgeMult(a, b, prod);
	};

	CubieCube.CentMult = function(a, b, prod) {
		prod.ct = [];
		for (var cent = 0; cent < 6; cent++) {
			prod.ct[cent] = a.ct[b.ct[cent]];
		}
	};

	CubieCube.prototype.init = function(ca, ea) {
		this.ca = ca.slice();
		this.ea = ea.slice();
		return this;
	};

	CubieCube.prototype.hashCode = function() {
		var ret = 0;
		for (var i = 0; i < 20; i++) {
			ret = 0 | (ret * 31 + (i < 12 ? this.ea[i] : this.ca[i - 12]));
		}
		return ret;
	}

	CubieCube.prototype.isEqual = function(c) {
		c = c || CubieCube.SOLVED;
		for (var i = 0; i < 8; i++) {
			if (this.ca[i] != c.ca[i]) {
				return false;
			}
		}
		for (var i = 0; i < 12; i++) {
			if (this.ea[i] != c.ea[i]) {
				return false;
			}
		}
		return true;
	};

	CubieCube.cFacelet = [
		[8, 9, 20], // URF
		[6, 18, 38], // UFL
		[0, 36, 47], // ULB
		[2, 45, 11], // UBR
		[29, 26, 15], // DFR
		[27, 44, 24], // DLF
		[33, 53, 42], // DBL
		[35, 17, 51]  // DRB
	];
	CubieCube.eFacelet = [
		[5, 10], // UR
		[7, 19], // UF
		[3, 37], // UL
		[1, 46], // UB
		[32, 16], // DR
		[28, 25], // DF
		[30, 43], // DL
		[34, 52], // DB
		[23, 12], // FR
		[21, 41], // FL
		[50, 39], // BL
		[48, 14]  // BR
	];
	CubieCube.ctFacelet = [4, 13, 22, 31, 40, 49]; // centers
	CubieCube.faceMap = (function() {
		var f = [];
		for (var c = 0; c < 8; c++) {
			for (var n = 0; n < 3; n++) {
				f[CubieCube.cFacelet[c][n]] = [0, c, n];
			}
		}
		for (var e = 0; e < 12; e++) {
			for (var n = 0; n < 2; n++)
				f[CubieCube.eFacelet[e][n]] = [1, e, n];
		}
		return f;
	})();

	CubieCube.prototype.toPerm = function(cFacelet, eFacelet, ctFacelet, withOri) {
		cFacelet = cFacelet || CubieCube.cFacelet;
		eFacelet = eFacelet || CubieCube.eFacelet;
		ctFacelet = ctFacelet || CubieCube.ctFacelet;
		var f = [];
		for (var i = 0; i < 54; i++) {
			f[i] = i;
		}
		var obj = this;
		if (withOri && obj.ori) {
			obj = new CubieCube();
			var rot = CubieCube.rotCube[CubieCube.rotMulI[0][this.ori]];
			CubieCube.CubeMult(this, rot, obj);
			for (var i = 0; i < 6; i++) {
				f[ctFacelet[i]] = ctFacelet[rot.ct[i]];
			}
		}
		for (var c = 0; c < 8; c++) {
			var j = obj.ca[c] & 0x7; // cornercubie with index j is at
			var ori = obj.ca[c] >> 3; // Orientation of this cubie
			for (var n = 0; n < 3; n++)
				f[cFacelet[c][(n + ori) % 3]] = cFacelet[j][n];
		}
		for (var e = 0; e < 12; e++) {
			var j = obj.ea[e] >> 1; // edgecubie with index j is at edgeposition
			var ori = obj.ea[e] & 1; // Orientation of this cubie
			for (var n = 0; n < 2; n++)
				f[eFacelet[e][(n + ori) % 2]] = eFacelet[j][n];
		}
		return f;
	}

	CubieCube.prototype.toFaceCube = function(cFacelet, eFacelet, ctFacelet, withOri) {
		var perm = this.toPerm(cFacelet, eFacelet, ctFacelet, withOri);
		var ts = "URFDLB";
		var f = [];
		for (var i = 0; i < 54; i++) {
			f[i] = ts[~~(perm[i] / 9)];
		}
		return f.join("");
	}

	CubieCube.prototype.prettyString = function(withOri) {
		var facelet = this.toFaceCube(null, null, null, withOri);
		var tmp =
			"        U0U1U2\n" +
			"        U3U4U5\n" +
			"        U6U7U8\n" +
			"L0L1L2  F0F1F2  R0R1R2  B0B1B2\n" +
			"L3L4L5  F3F4F5  R3R4R5  B3B4B5\n" +
			"L6L7L8  F6F7F8  R6R7R8  B6B7B8\n" +
			"        D0D1D2\n" +
			"        D3D4D5\n" +
			"        D6D7D8\n";
		return tmp.replace(/[URFDLB][0-8]/g, function(m) {
			var i1 = "URFDLB".indexOf(m[0]);
			return facelet[i1 * 9 + ~~m[1]] + ' ';
		});
	}

	CubieCube.prototype.invFrom = function(cc) {
		for (var edge = 0; edge < 12; edge++) {
			this.ea[cc.ea[edge] >> 1] = edge << 1 | cc.ea[edge] & 1;
		}
		for (var corn = 0; corn < 8; corn++) {
			this.ca[cc.ca[corn] & 0x7] = corn | 0x20 >> (cc.ca[corn] >> 3) & 0x18;
		}
		return this;
	}

	CubieCube.prototype.fromFacelet = function(facelet, cFacelet, eFacelet) {
		cFacelet = cFacelet || CubieCube.cFacelet;
		eFacelet = eFacelet || CubieCube.eFacelet;
		var count = 0;
		var f = [];
		var centers = facelet[4] + facelet[13] + facelet[22] + facelet[31] + facelet[40] + facelet[49];
		for (var i = 0; i < 54; ++i) {
			f[i] = centers.indexOf(facelet[i]);
			if (f[i] == -1) {
				return -1;
			}
			count += 1 << (f[i] << 2);
		}
		if (count != 0x999999) {
			return -1;
		}
		var col1, col2, i, j, ori;
		for (i = 0; i < 8; ++i) {
			for (ori = 0; ori < 3; ++ori)
				if (f[cFacelet[i][ori]] == 0 || f[cFacelet[i][ori]] == 3)
					break;
			col1 = f[cFacelet[i][(ori + 1) % 3]];
			col2 = f[cFacelet[i][(ori + 2) % 3]];
			for (j = 0; j < 8; ++j) {
				if (col1 == ~~(cFacelet[j][1] / 9) && col2 == ~~(cFacelet[j][2] / 9)) {
					this.ca[i] = j | ori % 3 << 3;
					break;
				}
			}
		}
		for (i = 0; i < 12; ++i) {
			for (j = 0; j < 12; ++j) {
				if (f[eFacelet[i][0]] == ~~(eFacelet[j][0] / 9) && f[eFacelet[i][1]] == ~~(eFacelet[j][1] / 9)) {
					this.ea[i] = j << 1;
					break;
				}
				if (f[eFacelet[i][0]] == ~~(eFacelet[j][1] / 9) && f[eFacelet[i][1]] == ~~(eFacelet[j][0] / 9)) {
					this.ea[i] = j << 1 | 1;
					break;
				}
			}
		}
		return this;
	}

	CubieCube.prototype.verify = function() {
		var mask = 0;
		var sum = 0;
		var ep = [];
		for (var e = 0; e < 12; e++) {
			mask |= 1 << 8 << (this.ea[e] >> 1);
			sum ^= this.ea[e] & 1;
			ep.push(this.ea[e] >> 1);
		}
		var cp = [];
		for (var c = 0; c < 8; c++) {
			mask |= 1 << (this.ca[c] & 7);
			sum += this.ca[c] >> 3 << 1;
			cp.push(this.ca[c] & 0x7);
		}
		if (mask != 0xfffff || sum % 6 != 0
				|| getNParity(getNPerm(ep, 12), 12) != getNParity(getNPerm(cp, 8), 8)) {
			return -1;
		}
		return 0;
	}

	CubieCube.moveCube = (function() {
		var moveCube = [];
		for (var i = 0; i < 18; i++) {
			moveCube[i] = new CubieCube();
		}
		moveCube[0].init([3, 0, 1, 2, 4, 5, 6, 7], [6, 0, 2, 4, 8, 10, 12, 14, 16, 18, 20, 22]);
		moveCube[3].init([20, 1, 2, 8, 15, 5, 6, 19], [16, 2, 4, 6, 22, 10, 12, 14, 8, 18, 20, 0]);
		moveCube[6].init([9, 21, 2, 3, 16, 12, 6, 7], [0, 19, 4, 6, 8, 17, 12, 14, 3, 11, 20, 22]);
		moveCube[9].init([0, 1, 2, 3, 5, 6, 7, 4], [0, 2, 4, 6, 10, 12, 14, 8, 16, 18, 20, 22]);
		moveCube[12].init([0, 10, 22, 3, 4, 17, 13, 7], [0, 2, 20, 6, 8, 10, 18, 14, 16, 4, 12, 22]);
		moveCube[15].init([0, 1, 11, 23, 4, 5, 18, 14], [0, 2, 4, 23, 8, 10, 12, 21, 16, 18, 7, 15]);
		for (var a = 0; a < 18; a += 3) {
			for (var p = 0; p < 2; p++) {
				CubieCube.CubeMult(moveCube[a + p], moveCube[a], moveCube[a + p + 1]);
			}
		}
		return moveCube;
	})();

	CubieCube.rotCube = (function() {
		var u4 = new CubieCube().init([3, 0, 1, 2, 7, 4, 5, 6], [6, 0, 2, 4, 14, 8, 10, 12, 23, 17, 19, 21]);
		u4.ct = [0, 5, 1, 3, 2, 4];
		var f2 = new CubieCube().init([5, 4, 7, 6, 1, 0, 3, 2], [12, 10, 8, 14, 4, 2, 0, 6, 18, 16, 22, 20]);
		f2.ct = [3, 4, 2, 0, 1, 5];
		var urf = new CubieCube().init([8, 20, 13, 17, 19, 15, 22, 10], [3, 16, 11, 18, 7, 22, 15, 20, 1, 9, 13, 5]);
		urf.ct = [2, 0, 1, 5, 3, 4];
		var c = new CubieCube();
		c.ct = [0, 1, 2, 3, 4, 5];
		var d = new CubieCube();
		var rotCube = [];
		for (var i = 0; i < 24; i++) {
			rotCube[i] = new CubieCube().init(c.ca, c.ea);
			rotCube[i].ct = c.ct.slice();
			CubieCube.CubeMult(c, u4, d);
			CubieCube.CentMult(c, u4, d);
			c.init(d.ca, d.ea);
			c.ct = d.ct.slice();
			if (i % 4 == 3) {
				CubieCube.CubeMult(c, f2, d);
				CubieCube.CentMult(c, f2, d);
				c.init(d.ca, d.ea);
				c.ct = d.ct.slice();
			}
			if (i % 8 == 7) {
				CubieCube.CubeMult(c, urf, d);
				CubieCube.CentMult(c, urf, d);
				c.init(d.ca, d.ea);
				c.ct = d.ct.slice();
			}
		}

		var movHash = [];
		var rotHash = [];
		var rotMult = [];
		var rotMulI = [];
		var rotMulM = [];
		for (var i = 0; i < 24; i++) {
			rotHash[i] = rotCube[i].hashCode();
			rotMult[i] = [];
			rotMulI[i] = [];
			rotMulM[i] = [];
		}
		for (var i = 0; i < 18; i++) {
			movHash[i] = CubieCube.moveCube[i].hashCode();
		}
		for (var i = 0; i < 24; i++) {
			for (var j = 0; j < 24; j++) {
				CubieCube.CubeMult(rotCube[i], rotCube[j], c);
				var k = rotHash.indexOf(c.hashCode());
				rotMult[i][j] = k;
				rotMulI[k][j] = i;
			}
		}
		for (var i = 0; i < 24; i++) {
			for (var j = 0; j < 18; j++) {
				CubieCube.CubeMult(rotCube[rotMulI[0][i]], CubieCube.moveCube[j], c);
				CubieCube.CubeMult(c, rotCube[i], d);
				var k = movHash.indexOf(d.hashCode());
				rotMulM[i][j] = k;
			}
		}

		var rot2str = [
			"", "y'", "y2", "y",
			"z2", "y' z2", "y2 z2", "y z2",
			"y' x'", "y2 x'", "y x'", "x'",
			"y' x", "y2 x", "y x", "x",
			"y z", "z", "y' z", "y2 z",
			"y' z'", "y2 z'", "y z'", "z'"
		];

		CubieCube.rotMult = rotMult;
		CubieCube.rotMulI = rotMulI;
		CubieCube.rotMulM = rotMulM;
		CubieCube.rot2str = rot2str;
		return rotCube;
	})();

	CubieCube.prototype.edgeCycles = function() {
		var visited = [];
		var small_cycles = [0, 0, 0];
		var cycles = 0;
		var parity = false;
		for (var x = 0; x < 12; ++x) {
			if (visited[x]) {
				continue
			}
			var length = -1;
			var flip = false;
			var y = x;
			do {
				visited[y] = true;
				++length;
				flip ^= this.ea[y] & 1;
				y = this.ea[y] >> 1;
			} while (y != x);
			cycles += length >> 1;
			if (length & 1) {
				parity = !parity;
				++cycles;
			}
			if (flip) {
				if (length == 0) {
					++small_cycles[0];
				} else if (length & 1) {
					small_cycles[2] ^= 1;
				} else {
					++small_cycles[1];
				}
			}
		}
		small_cycles[1] += small_cycles[2];
		if (small_cycles[0] < small_cycles[1]) {
			cycles += (small_cycles[0] + small_cycles[1]) >> 1;
		} else {
			var flip_cycles = [0, 2, 3, 5, 6, 8, 9];
			cycles += small_cycles[1] + flip_cycles[(small_cycles[0] - small_cycles[1]) >> 1];
		}
		return cycles - parity;
	};

	var CubeMoveRE = /^\s*([URFDLB]w?|[EMSyxz]|2-2[URFDLB]w)(['2]?)(@\d+)?\s*$/;
	var tmpCubie = new CubieCube();
	CubieCube.prototype.selfMoveStr = function(moveStr, isInv) {
		var m = CubeMoveRE.exec(moveStr);
		if (!m) {
			return;
		}
		var face = m[1];
		var pow = "2'".indexOf(m[2] || '-') + 2;
		if (isInv) {
			pow = 4 - pow;
		}
		if (m[3]) {
			this.tstamp = ~~m[3].slice(1);
		}
		this.ori = this.ori || 0;
		var axis = 'URFDLB'.indexOf(face);
		if (axis != -1) {
			m = axis * 3 + pow % 4 - 1
			m = CubieCube.rotMulM[this.ori][m];
			CubieCube.CubeMult(this, CubieCube.moveCube[m], tmpCubie);
			this.init(tmpCubie.ca, tmpCubie.ea);
			return m;
		}
		axis = 'UwRwFwDwLwBw'.indexOf(face);
		if (axis != -1) {
			axis >>= 1;
			m = (axis + 3) % 6 * 3 + pow % 4 - 1
			m = CubieCube.rotMulM[this.ori][m];
			CubieCube.CubeMult(this, CubieCube.moveCube[m], tmpCubie);
			this.init(tmpCubie.ca, tmpCubie.ea);
			var rot = [3, 15, 17, 1, 11, 23][axis];
			for (var i = 0; i < pow; i++) {
				this.ori = CubieCube.rotMult[rot][this.ori];
			}
			return m;
		}
		axis = ['2-2Uw', '2-2Rw', '2-2Fw', '2-2Dw', '2-2Lw', '2-2Bw'].indexOf(face);
		if (axis == -1) {
			axis = [null, null, 'S', 'E', 'M', null].indexOf(face);
		}
		if (axis != -1) {
			var m1 = axis * 3 + (4 - pow) % 4 - 1;
			var m2 = (axis + 3) % 6 * 3 + pow % 4 - 1;
			m1 = CubieCube.rotMulM[this.ori][m1];
			CubieCube.CubeMult(this, CubieCube.moveCube[m1], tmpCubie);
			this.init(tmpCubie.ca, tmpCubie.ea);
			m2 = CubieCube.rotMulM[this.ori][m2];
			CubieCube.CubeMult(this, CubieCube.moveCube[m2], tmpCubie);
			this.init(tmpCubie.ca, tmpCubie.ea);
			var rot = [3, 15, 17, 1, 11, 23][axis];
			for (var i = 0; i < pow; i++) {
				this.ori = CubieCube.rotMult[rot][this.ori];
			}
			return m1 + 18;
		}
		axis = 'yxz'.indexOf(face);
		if (axis != -1) {
			var rot = [3, 15, 17][axis];
			for (var i = 0; i < pow; i++) {
				this.ori = CubieCube.rotMult[rot][this.ori];
			}
			return;
		}
	}

	CubieCube.prototype.selfConj = function(conj) {
		if (conj === undefined) {
			conj = this.ori;
		}
		if (conj != 0) {
			CubieCube.CubeMult(CubieCube.rotCube[conj], this, tmpCubie);
			CubieCube.CubeMult(tmpCubie, CubieCube.rotCube[CubieCube.rotMulI[0][conj]], this);
			this.ori = CubieCube.rotMulI[this.ori][conj] || 0;
		}
	}

	var minx = (function() {
		var U = 0, R = 1, F = 2, L = 3, BL = 4, BR = 5, DR = 6, DL = 7, DBL = 8, B = 9, DBR = 10, D = 11;
		var oppFace = [D, DBL, B, DBR, DR, DL, BL, BR, R, F, L, U];
		var adjFaces = [
			[BR, R, F, L, BL], //U
			[DBR, DR, F, U, BR], //R
			[DR, DL, L, U, R], //F
			[DL, DBL, BL, U, F], //L
			[DBL, B, BR, U, L], //BL
			[B, DBR, R, U, BL], //BR
			[D, DL, F, R, DBR], //DR
			[D, DBL, L, F, DR], //DL
			[D, B, BL, L, DL], //DBL
			[D, DBR, BR, BL, DBL], //B
			[D, DR, R, BR, B], //DBR
			[DR, DBR, B, DBL, DL]  //D
		];

		// wide: 0=single, 1=all, 2=all but single
		// state: corn*5, edge*5, center*1
		function doMove(state, face, pow, wide) {
			pow = (pow % 5 + 5) % 5;
			if (pow == 0) {
				return;
			}
			var base = face * 11;
			var adjs = [];
			var swaps = [[], [], [], [], []];
			for (var i = 0; i < 5; i++) {
				var aface = adjFaces[face][i];
				var ridx = adjFaces[aface].indexOf(face);
				if (wide == 0 || wide == 1) {
					swaps[i].push(
						base + i,
						base + i + 5,
						aface * 11 + ridx % 5 + 5,
						aface * 11 + ridx % 5,
						aface * 11 + (ridx + 1) % 5);
				}
				if (wide == 1 || wide == 2) {
					swaps[i].push(aface * 11 + 10);
					for (var j = 1; j < 5; j++) {
						swaps[i].push(aface * 11 + (ridx + j) % 5 + 5);
					}
					for (var j = 2; j < 5; j++) {
						swaps[i].push(aface * 11 + (ridx + j) % 5);
					}
					var ii = 4 - i;
					var opp = oppFace[face];
					var oaface = adjFaces[opp][ii];
					var oridx = adjFaces[oaface].indexOf(opp);
					swaps[i].push(
						opp * 11 + ii,
						opp * 11 + ii + 5,
						oaface * 11 + 10);
					for (var j = 0; j < 5; j++) {
						swaps[i].push(
							oaface * 11 + (oridx + j) % 5 + 5,
							oaface * 11 + (oridx + j) % 5);
					}
				}
			}
			for (var i = 0; i < swaps[0].length; i++) {
				mathlib.acycle(state, [swaps[0][i], swaps[1][i], swaps[2][i], swaps[3][i], swaps[4][i]], pow);
			}
		}

		return {
			doMove: doMove,
			oppFace: oppFace,
			adjFaces: adjFaces
		}
	})();

	function createPrun(prun, init, size, maxd, doMove, N_MOVES, N_POWER, N_INV) {
		var isMoveTable = $.isArray(doMove);
		N_MOVES = N_MOVES || 6;
		N_POWER = N_POWER || 3;
		N_INV = N_INV || 256;
		maxd = maxd || 256;
		for (var i = 0, len = (size + 7) >>> 3; i < len; i++) {
			prun[i] = -1;
		}
		if (!$.isArray(init)) {
			init = [init];
		}
		for (var i = 0; i < init.length; i++) {
			prun[init[i] >> 3] ^= 15 << ((init[i] & 7) << 2);
		}
		var val = 0;
		// var t = +new Date;
		for (var l = 0; l <= maxd; l++) {
			var done = 0;
			var inv = l >= N_INV;
			var fill = (l + 1) ^ 15;
			var find = inv ? 0xf : l;
			var check = inv ? l : 0xf;

			out: for (var p = 0; p < size; p++, val >>= 4) {
				if ((p & 7) == 0) {
					val = prun[p >> 3];
					if (!inv && val == -1) {
						p += 7;
						continue;
					}
				}
				if ((val & 0xf) != find) {
					continue;
				}
				for (var m = 0; m < N_MOVES; m++) {
					var q = p;
					for (var c = 0; c < N_POWER; c++) {
						q = isMoveTable ? doMove[m][q] : doMove(q, m);
						if (q < 0) {
							break;
						}
						if (getPruning(prun, q) != check) {
							continue;
						}
						++done;
						if (inv) {
							prun[p >> 3] ^= fill << ((p & 7) << 2);
							continue out;
						}
						prun[q >> 3] ^= fill << ((q & 7) << 2);
					}
				}
			}
			if (done == 0) {
				break;
			}
			DEBUG && console.log('[prun]', done);
		}
	}

	// stateParams: [[init, doMove, size, [maxd], [N_INV]], [...]...]
	function Solver(N_MOVES, N_POWER, stateParams) {
		this.N_STATES = stateParams.length;
		this.N_MOVES = N_MOVES;
		this.N_POWER = N_POWER;
		this.stateParams = stateParams;
		this.coords = [];
		for (var i = 0; i < this.N_STATES; i++) {
			var doMove = stateParams[i][1];
			if ($.isArray(doMove)) {
				this.coords[i] = new Coord(doMove[1], doMove[2], doMove[3]);
			}
		}
		this.inited = false;
	}

	var _ = Solver.prototype;

	_.init = function() {
		if (this.inited) {
			return;
		}
		this.move = [];
		this.prun = [];
		for (var i = 0; i < this.N_STATES; i++) {
			var stateParam = this.stateParams[i];
			var init = stateParam[0];
			var doMove = stateParam[1];
			var size = stateParam[2];
			var maxd = stateParam[3];
			var N_INV = stateParam[4];
			this.move[i] = [];
			this.prun[i] = [];
			createMove(this.move[i], size, doMove, this.N_MOVES);
			createPrun(this.prun[i], init, size, maxd, this.move[i], this.N_MOVES, this.N_POWER, N_INV);
		}
		this.solv = new Searcher(null, (state) => {
			var prun = 0;
			for (var i = 0; i < this.N_STATES; i++) {
				prun = Math.max(prun, getPruning(this.prun[i], state[i]));
			}
			return prun;
		}, (state, move) => {
			for (var i = 0; i < this.N_STATES; i++) {
				state[i] = this.move[i][move][state[i]];
			}
			return state;
		}, this.N_MOVES, this.N_POWER);
		this.inited = true;
	}

	_.search = function(state, minl, MAXL) {
		MAXL = (MAXL || 99) + 1;
		if (!this.inited) {
			this.init();
		}
		this.sol = this.solv.solve(state, minl, MAXL);
		return this.sol;
	};

	_.toStr = function(sol, move_map, power_map) {
		return sol.map((move) => move_map[move[0]] + power_map[move[1]]).join(' ').replace(/ +/g, ' ');
	};

	// ida search algorithm
	// isSolved(state) -> boolean, in case isSolved = null, getPrun(idx) == 0 is used
	// getPrun(state) -> int
	// doMove(state) -> newState
	// if ckmv[last_move] >> cur_move & 1, cur_move is skipped
	function Searcher(isSolved, getPrun, doMove, N_AXIS, N_POWER, ckmv) {
		this.isSolved = isSolved || function() { return true; };
		this.getPrun = getPrun;
		this.doMove = doMove;
		this.N_AXIS = N_AXIS;
		this.N_POWER = N_POWER;
		this.ckmv = ckmv || valuedArray(N_AXIS, function(i) { return 1 << i; } );
	}

	_ = Searcher.prototype;

	_.solve = function(idx, minl, MAXL, callback, cost) {
		var sols = this.solveMulti([idx], minl, MAXL, callback, cost);
		return sols == null ? null : sols[0];
	};

	_.solveMulti = function(idxs, minl, MAXL, callback, cost) {
		this.sidx = 0;
		this.sol = [];
		this.length = minl;
		this.idxs = idxs;
		return this.nextMulti(MAXL, callback, cost);
	};

	_.next = function(MAXL, callback, cost) {
		var sols = this.nextMulti(MAXL, callback, cost);
		return sols == null ? null : sols[0];
	};

	_.nextMulti = function(MAXL, callback, cost) {
		this.cost = (cost || 1e9) + 1;
		this.callback = callback || function() { return true; };
		for (; this.length <= MAXL; this.length++) {
			for (; this.sidx < this.idxs.length; this.sidx++) {
				if (this.idaSearch(this.idxs[this.sidx], this.length, 0, -1, this.sol) == 0) {
					return this.cost <= 0 ? null : [this.sol, this.sidx];
				}
			}
			this.sidx = 0;
		}
		return null;
	}

	_.idaSearch = function(idx, maxl, depth, lm, sol) {
		if (--this.cost <= 0) {
			return 0;
		}
		var prun = this.getPrun(idx);
		if (prun > maxl) {
			return prun > maxl + 1 ? 2 : 1;
		} else if (maxl == 0) {
			return this.isSolved(idx) && this.callback(sol, this.sidx) ? 0 : 1;
		} else if (prun == 0 && maxl == 1 && this.isSolved(idx)) {
			return 1;
		}
		var axis = sol.length > depth ? sol[depth][0] : 0;
		for (; axis < this.N_AXIS; axis++) {
			if (this.ckmv[lm] >> axis & 1) {
				continue;
			}
			var idx1 = $.isArray(idx) ? idx.slice() : idx;
			var pow = sol.length > depth ? sol[depth][1] : 0;
			for (; pow < this.N_POWER; pow++) {
				idx1 = this.doMove(idx1, axis, pow);
				if (idx1 == null) {
					break;
				}
				sol[depth] = [axis, pow];
				var ret = this.idaSearch(idx1, maxl - 1, depth + 1, axis, sol);
				if (ret == 0) {
					return 0;
				}
				sol.pop();
				if (ret == 2) {
					break;
				}
			}
		}
		return 1;
	};

	// state: string not null
	// solvedStates: [solvedstate, solvedstate, ...], string not null
	// moveFunc: function(state, move);
	// moves: {move: face0 | axis0}, face0 | axis0 = 4 + 4 bits
	function gSolver(solvedStates, doMove, moves) {
		this.solvedStates = solvedStates;
		this.doMove = doMove;
		this.movesList = [];
		for (var move in moves) {
			this.movesList.push([move, moves[move]]);
		}
		this.prunTable = {};
		this.toUpdateArr = null;
		this.prunTableSize = 0;
		this.prunDepth = -1;
		this.cost = 0;
		this.MAX_PRUN_SIZE = 100000;
	}

	_ = gSolver.prototype;

	_.updatePrun = function(targetDepth) {
		targetDepth = targetDepth === undefined ? this.prunDepth + 1 : targetDepth;
		for (var depth = this.prunDepth + 1; depth <= targetDepth; depth++) {
			if (this.prevSize >= this.MAX_PRUN_SIZE) {
				DEBUG && console.log('[gSolver] skipPrun', depth, this.prunTableSize);
				break;
			}
			var t = +new Date;
			if (depth < 1) {
				this.prevSize = 0;
				for (var i = 0; i < this.solvedStates.length; i++) {
					var state = this.solvedStates[i];
					if (!(state in this.prunTable)) {
						this.prunTable[state] = depth;
						this.prunTableSize++;
					}
				}
			} else {
				this.updatePrunBFS(depth - 1);
			}
			if (this.cost == 0) {
				return;
			}
			this.prunDepth = depth;
			DEBUG && console.log('[gSolver] updatePrun', depth, this.prunTableSize - this.prevSize, +new Date - t);
			this.prevSize = this.prunTableSize;
		}
	};

	_.updatePrunBFS = function(fromDepth) {
		if (this.toUpdateArr == null) {
			this.toUpdateArr = [];
			for (var state in this.prunTable) {
				if (this.prunTable[state] != fromDepth) {
					continue;
				}
				this.toUpdateArr.push(state);
			}
		}
		while (this.toUpdateArr.length != 0) {
			var state = this.toUpdateArr.pop();
			for (var moveIdx = 0; moveIdx < this.movesList.length; moveIdx++) {
				var newState = this.doMove(state, this.movesList[moveIdx][0]);
				if (!newState || newState in this.prunTable) {
					continue;
				}
				this.prunTable[newState] = fromDepth + 1;
				this.prunTableSize++;
			}
			if (this.cost >= 0) {
				if (this.cost == 0) {
					return;
				}
				this.cost--;
			}
		}
		this.toUpdateArr = null;
	};

	_.search = function(state, minl, MAXL) {
		this.sol = [];
		this.subOpt = false;
		this.state = state;
		this.visited = {};
		this.maxl = minl = minl || 0;
		return this.searchNext(MAXL);
	};

	_.searchNext = function(MAXL, cost) {
		MAXL = (MAXL + 1) || 99;
		this.prevSolStr = this.solArr ? this.solArr.join(',') : null;
		this.solArr = null;
		this.cost = cost || -1;
		for (; this.maxl < MAXL; this.maxl++) {
			this.updatePrun(Math.ceil(this.maxl / 2));
			if (this.cost == 0) {
				return null;
			}
			if (this.idaSearch(this.state, this.maxl, null, 0)) {
				break;
			}
		}
		return this.solArr;
	}

	_.getPruning = function(state) {
		var prun = this.prunTable[state];
		return prun === undefined ? this.prunDepth + 1 : prun;
	};

	_.idaSearch = function(state, maxl, lm, depth) {
		if (this.getPruning(state) > maxl) {
			return false;
		}
		if (maxl == 0) {
			if (this.solvedStates.indexOf(state) == -1) {
				return false;
			}
			var solArr = this.sol.map((move) => this.movesList[move][0]);
			this.subOpt = true;
			if (solArr.join(',') == this.prevSolStr) {
				return false;
			}
			this.solArr = solArr;
			return true;
		}
		if (!this.subOpt) {
			if (state in this.visited && this.visited[state] < depth) {
				return false;
			}
			this.visited[state] = depth;
		}
		if (this.cost >= 0) {
			if (this.cost == 0) {
				return true;
			}
			this.cost--;
		}
		var lastMove = lm == null ? '' : this.movesList[lm][0];
		var lastAxisFace = lm == null ? -1 : this.movesList[lm][1];
		for (var moveIdx = this.sol[depth] || 0; moveIdx < this.movesList.length; moveIdx++) {
			var moveArgs = this.movesList[moveIdx];
			var axisface = moveArgs[1] ^ lastAxisFace;
			var move = moveArgs[0];
			if (axisface == 0 ||
				(axisface & 0xf) == 0 && move <= lastMove) {
				continue;
			}
			var newState = this.doMove(state, move);
			if (!newState || newState == state) {
				continue;
			}
			this.sol[depth] = moveIdx;
			if (this.idaSearch(newState, maxl - 1, moveIdx, depth + 1)) {
				return true;
			}
			this.sol.pop();
		}
		return false;
	};

	var randGen = (function() {
		var rndFunc;
		var rndCnt;
		var seedStr; // '' + new Date().getTime();

		function random() {
			rndCnt++;
			// console.log(rndCnt);
			return rndFunc();
		}

		function getSeed() {
			return [rndCnt, seedStr];
		}

		function setSeed(_rndCnt, _seedStr) {
			if (_seedStr && (_seedStr != seedStr || rndCnt > _rndCnt)) {
				var seed = [];
				for (var i = 0; i < _seedStr.length; i++) {
					seed[i] = _seedStr.charCodeAt(i);
				}
				isaac.seed(seed);
				rndFunc = isaac.random;
				rndCnt = 0;
				seedStr = _seedStr;
			}
			while (rndCnt < _rndCnt) {
				rndFunc();
				rndCnt++;
			}
		}

		var seed = '' + new Date().getTime();
		if (typeof crypto != 'undefined' && crypto.getRandomValues) {
			seed = String.fromCharCode.apply(null, crypto.getRandomValues(new Uint16Array(256)));
			DEBUG && console.log('[mathlib] use crypto seed', seed);
		} else {
			DEBUG && console.log('[mathlib] use datetime seed', seed);
		}
		setSeed(256, seed);

		return {
			random: random,
			getSeed: getSeed,
			setSeed: setSeed
		};
	})();

	function rndEl(x) {
		return x[~~(randGen.random() * x.length)];
	}

	function rn(n) {
		return ~~(randGen.random() * n)
	}

	function rndHit(prob) {
		return randGen.random() < prob;
	}

	function rndPerm(n, isEven) {
		var p = 0;
		var arr = [];
		for (var i = 0; i < n; i++) {
			arr[i] = i;
		}
		for (var i = 0; i < n - 1; i++) {
			var k = rn(n - i);
			circle(arr, i, i + k);
			p ^= k != 0;
		}
		if (isEven && p) {
			circle(arr, 0, 1);
		}
		return arr;
	}

	function rndProb(plist) {
		var cum = 0;
		var curIdx = 0;
		for (var i = 0; i < plist.length; i++) {
			if (plist[i] == 0) {
				continue;
			}
			if (randGen.random() < plist[i] / (cum + plist[i])) {
				curIdx = i;
			}
			cum += plist[i];
		}
		return curIdx;
	}

	function time2str(unix, format) {
		if (!unix) {
			return 'N/A';
		}
		format = format || '%Y-%M-%D %h:%m:%s';
		var date = new Date(unix * 1000);
		return format
			.replace('%Y', date.getFullYear())
			.replace('%M', ('0' + (date.getMonth() + 1)).slice(-2))
			.replace('%D', ('0' + date.getDate()).slice(-2))
			.replace('%h', ('0' + date.getHours()).slice(-2))
			.replace('%m', ('0' + date.getMinutes()).slice(-2))
			.replace('%s', ('0' + date.getSeconds()).slice(-2))
			.replace('%S', ('00' + date.getMilliseconds()).slice(-3));
	}

	var timeRe = /^\s*(\d+)-(\d+)-(\d+) (\d+):(\d+):(\d+)\s*$/;

	function str2time(val) {
		var m = timeRe.exec(val);
		if (!m) {
			return null;
		}
		var date = new Date(1970, 0, 1);
		date.setFullYear(~~m[1]);
		date.setMonth(~~m[2] - 1);
		date.setDate(~~m[3]);
		date.setHours(~~m[4]);
		date.setMinutes(~~m[5]);
		date.setSeconds(~~m[6]);
		return ~~(date.getTime() / 1000);
	}

	function obj2str(val) {
		if (typeof val == 'string') {
			return val;
		}
		return JSON.stringify(val);
	}

	function str2obj(val) {
		if (typeof val != 'string') {
			return val;
		}
		return JSON.parse(val);
	}

	function valuedArray(len, val) {
		var ret = [];
		var isFun = typeof val == 'function';
		for (var i = 0; i < len; i++) {
			ret[i] = isFun ? val(i) : val;
		}
		return ret;
	}

	function idxArray(arr, idx) {
		return arr.map((elem) => elem[idx]);
	}

	function permOriMult(p1, p2, prod, o1, o2, ori, oriMod) {
		for (var i = 0; i < p2.length; i++) {
			if (oriMod) {
				ori[i] = (o1[p2[i]] + o2[i]) % oriMod;
			}
			prod[i] = p1[p2[i]];
		}
	}

	Math.TAU = Math.PI * 2;

	return {
		Cnk: Cnk,
		fact: fact,
		bitCount: bitCount,
		getPruning: getPruning,
		setNOri: setNOri,
		getNOri: getNOri,
		setNPerm: setNPerm,
		getNPerm: getNPerm,
		getNParity: getNParity,
		permMul4: permMul4,
		Coord: Coord,
		createMove: createMove,
		createMoveHash: createMoveHash,
		edgeMove: edgeMove,
		circle: circle,
		circleOri: circleOri,
		acycle: acycle,
		createPrun: createPrun,
		CubieCube: CubieCube,
		minx: minx,
		SOLVED_FACELET: "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB",
		fillFacelet: fillFacelet,
		detectFacelet: detectFacelet,
		rn: rn,
		rndEl: rndEl,
		rndProb: rndProb,
		rndHit: rndHit,
		time2str: time2str,
		str2time: str2time,
		obj2str: obj2str,
		str2obj: str2obj,
		valuedArray: valuedArray,
		idxArray: idxArray,
		Solver: Solver,
		Searcher: Searcher,
		rndPerm: rndPerm,
		permOriMult: permOriMult,
		gSolver: gSolver,
		getSeed: randGen.getSeed,
		setSeed: randGen.setSeed
	};
})();

"use strict";

var min2phase = (function() {
	var USE_TWST_FLIP_PRUN = true;
	var PARTIAL_INIT_LEVEL = 2;

	var MAX_PRE_MOVES = 20;
	var TRY_INVERSE = true;
	var TRY_THREE_AXES = true;

	var USE_CONJ_PRUN = USE_TWST_FLIP_PRUN;
	var MIN_P1LENGTH_PRE = 7;
	var MAX_DEPTH2 = 13;

	var INVERSE_SOLUTION = 0x2;

	function Search() {
		this.move = [];
		this.moveSol = [];

		this.nodeUD = [];

		this.valid1 = 0;
		this.allowShorter = false;
		this.cc = new CubieCube();
		this.urfCubieCube = [];
		this.urfCoordCube = [];
		this.phase1Cubie = [];

		this.preMoveCubes = [];
		this.preMoves = [];
		this.preMoveLen = 0;
		this.maxPreMoves = 0;

		this.isRec = false;
		for (var i = 0; i < 21; i++) {
			this.nodeUD[i] = new CoordCube();
			this.phase1Cubie[i] = new CubieCube();
		}
		for (var i = 0; i < 6; i++) {
			this.urfCubieCube[i] = new CubieCube();
			this.urfCoordCube[i] = new CoordCube();
		}
		for (var i = 0; i < MAX_PRE_MOVES; i++) {
			this.preMoveCubes[i + 1] = new CubieCube();
		}
	}

	var Ux1 = 0;
	var Ux2 = 1;
	var Ux3 = 2;
	var Rx1 = 3;
	var Rx2 = 4;
	var Rx3 = 5;
	var Fx1 = 6;
	var Fx2 = 7;
	var Fx3 = 8;
	var Dx1 = 9;
	var Dx2 = 10;
	var Dx3 = 11;
	var Lx1 = 12;
	var Lx2 = 13;
	var Lx3 = 14;
	var Bx1 = 15;
	var Bx2 = 16;
	var Bx3 = 17;

	var N_MOVES = 18;
	var N_MOVES2 = 10;
	var N_FLIP = 2048;
	var N_FLIP_SYM = 336;
	var N_TWST = 2187;
	var N_TWST_SYM = 324;
	var N_PERM = 40320;
	var N_PERM_SYM = 2768;
	var N_MPERM = 24;
	var N_SLICE = 495;
	var N_COMB = 140;

	var SYM_E2C_MAGIC = 0x00DDDD00;
	var Cnk = [];
	var fact = [1];
	var move2str = [
		"U ", "U2", "U'", "R ", "R2", "R'", "F ", "F2", "F'",
		"D ", "D2", "D'", "L ", "L2", "L'", "B ", "B2", "B'"
	];
	var ud2std = [Ux1, Ux2, Ux3, Rx2, Fx2, Dx1, Dx2, Dx3, Lx2, Bx2, Rx1, Rx3, Fx1, Fx3, Lx1, Lx3, Bx1, Bx3];
	var std2ud = [];
	var ckmv2bit = [];
	var urfMove = [
		[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17],
		[6, 7, 8, 0, 1, 2, 3, 4, 5, 15, 16, 17, 9, 10, 11, 12, 13, 14],
		[3, 4, 5, 6, 7, 8, 0, 1, 2, 12, 13, 14, 15, 16, 17, 9, 10, 11],
		[2, 1, 0, 5, 4, 3, 8, 7, 6, 11, 10, 9, 14, 13, 12, 17, 16, 15],
		[8, 7, 6, 2, 1, 0, 5, 4, 3, 17, 16, 15, 11, 10, 9, 14, 13, 12],
		[5, 4, 3, 8, 7, 6, 2, 1, 0, 14, 13, 12, 17, 16, 15, 11, 10, 9]
	];

	{ // init util
		for (var i = 0; i < 18; i++) {
			std2ud[ud2std[i]] = i;
		}
		for (var i = 0; i < 10; i++) {
			var ix = ~~(ud2std[i] / 3);
			ckmv2bit[i] = 0;
			for (var j = 0; j < 10; j++) {
				var jx = ~~(ud2std[j] / 3);
				ckmv2bit[i] |= ((ix == jx) || ((ix % 3 == jx % 3) && (ix >= jx)) ? 1 : 0) << j;
			}
		}
		ckmv2bit[10] = 0;
		for (var i = 0; i < 13; i++) {
			Cnk[i] = [];
			fact[i + 1] = fact[i] * (i + 1);
			Cnk[i][0] = Cnk[i][i] = 1;
			for (var j = 1; j < 13; j++) {
				Cnk[i][j] = j <= i ? Cnk[i - 1][j - 1] + Cnk[i - 1][j] : 0;
			}
		}
	}

	function setPruning(table, index, value) {
		table[index >> 3] ^= value << (index << 2); // index << 2 <=> (index & 7) << 2
	}

	function getPruning(table, index) {
		return table[index >> 3] >> (index << 2) & 0xf; // index << 2 <=> (index & 7) << 2
	}

	function getPruningMax(maxValue, table, index) {
		return Math.min(maxValue, table[index >> 3] >> (index << 2) & 0xf);
	}

	function hasZero(val) {
		return ((val - 0x11111111) & ~val & 0x88888888) != 0;
	}

	function ESym2CSym(idx) {
		return idx ^ (SYM_E2C_MAGIC >> ((idx & 0xf) << 1) & 3);
	}

	function getPermSymInv(idx, sym, isCorner) {
		var idxi = PermInvEdgeSym[idx];
		if (isCorner) {
			idxi = ESym2CSym(idxi);
		}
		return idxi & 0xfff0 | SymMult[idxi & 0xf][sym];
	}

	function CubieCube() {
		this.ca = [0, 1, 2, 3, 4, 5, 6, 7];
		this.ea = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
	}

	function setNPerm(arr, idx, n) {
		n--;
		var val = 0x76543210;
		for (var i = 0; i < n; ++i) {
			var p = fact[n - i];
			var v = ~~(idx / p);
			idx %= p;
			v <<= 2;
			arr[i] = arr[i] & 0xf0 | val >> v & 0xf;
			var m = (1 << v) - 1;
			val = (val & m) + (val >> 4 & ~m);
		}
		arr[n] = arr[n] & 0xf0 | val & 0xf;
	}

	function getNPerm(arr, n) {
		var idx = 0,
			val = 0x76543210;
		for (var i = 0; i < n - 1; ++i) {
			var v = (arr[i] & 0xf) << 2;
			idx = (n - i) * idx + (val >> v & 0xf);
			val -= 0x11111110 << v;
		}
		return idx;
	}

	function setNPermFull(arr, idx, n) {
		arr[n - 1] = arr[n - 1] & 0xf0;
		for (var i = n - 2; i >= 0; --i) {
			arr[i] = arr[i] & 0xf0 | idx % (n - i);
			idx = ~~(idx / (n - i));
			for (var j = i + 1; j < n; ++j) {
				if ((arr[j] & 0xf) >= (arr[i] & 0xf)) {
					arr[j] += 1;
				}
			}
		}
	}

	function getNPermFull(arr, n) {
		var idx = 0;
		for (var i = 0; i < n; ++i) {
			idx *= n - i;
			for (var j = i + 1; j < n; ++j) {
				if ((arr[j] & 0xf) < (arr[i] & 0xf)) {
					++idx;
				}
			}
		}
		return idx;
	}

	function getComb(arr, mask) {
		var end = arr.length - 1;
		var idxC = 0,
			r = 4;
		for (var i = end; i >= 0; i--) {
			var perm = arr[i] & 0xf;
			if ((perm & 0xc) == mask) {
				idxC += Cnk[i][r--];
			}
		}
		return idxC;
	}

	function setComb(arr, idxC, mask) {
		var end = arr.length - 1;
		var r = 4,
			fill = end;
		for (var i = end; i >= 0; i--) {
			if (idxC >= Cnk[i][r]) {
				idxC -= Cnk[i][r--];
				arr[i] = arr[i] & 0xf0 | r | mask;
			} else {
				if ((fill & 0xc) == mask) {
					fill -= 4;
				}
				arr[i] = arr[i] & 0xf0 | fill--;
			}
		}
	}

	function getNParity(idx, n) {
		var p = 0;
		for (var i = n - 2; i >= 0; i--) {
			p ^= idx % (n - i);
			idx = ~~(idx / (n - i));
		}
		return p & 1;
	}
	CubieCube.EdgeMult = function(a, b, prod) {
		for (var ed = 0; ed < 12; ed++) {
			prod.ea[ed] = a.ea[b.ea[ed] & 0xf] ^ (b.ea[ed] & 0x10);
		}
	}
	CubieCube.CornMult = function(a, b, prod) {
		for (var corn = 0; corn < 8; corn++) {
			var ori = ((a.ca[b.ca[corn] & 0xf] >> 4) + (b.ca[corn] >> 4)) % 3;
			prod.ca[corn] = a.ca[b.ca[corn] & 0xf] & 0xf | ori << 4;
		}
	}
	CubieCube.CornMultFull = function(a, b, prod) {
		for (var corn = 0; corn < 8; corn++) {
			var oriA = a.ca[b.ca[corn] & 0xf] >> 4;
			var oriB = b.ca[corn] >> 4;
			var ori = oriA + ((oriA < 3) ? oriB : 6 - oriB);
			ori = ori % 3 + ((oriA < 3) == (oriB < 3) ? 0 : 3);
			prod.ca[corn] = a.ca[b.ca[corn] & 0xf] & 0xf | ori << 4;
		}
	}
	CubieCube.CornConjugate = function(a, idx, b) {
		var sinv = SymCube[SymMultInv[0][idx]];
		var s = SymCube[idx];
		for (var corn = 0; corn < 8; corn++) {
			var oriA = sinv.ca[a.ca[s.ca[corn] & 0xf] & 0xf] >> 4;
			var oriB = a.ca[s.ca[corn] & 0xf] >> 4;
			var ori = (oriA < 3) ? oriB : (3 - oriB) % 3;
			b.ca[corn] = sinv.ca[a.ca[s.ca[corn] & 0xf] & 0xf] & 0xf | ori << 4;
		}
	}
	CubieCube.EdgeConjugate = function(a, idx, b) {
		var sinv = SymCube[SymMultInv[0][idx]];
		var s = SymCube[idx];
		for (var ed = 0; ed < 12; ed++) {
			b.ea[ed] = sinv.ea[a.ea[s.ea[ed] & 0xf] & 0xf] ^ (a.ea[s.ea[ed] & 0xf] & 0x10) ^ (s.ea[ed] & 0x10);
		}
	}
	CubieCube.prototype.init = function(ca, ea) {
		this.ca = ca.slice();
		this.ea = ea.slice();
		return this;
	}
	CubieCube.prototype.initCoord = function(cperm, twst, eperm, flip) {
		setNPerm(this.ca, cperm, 8);
		this.setTwst(twst);
		setNPermFull(this.ea, eperm, 12);
		this.setFlip(flip);
		return this;
	}
	CubieCube.prototype.isEqual = function(c) {
		for (var i = 0; i < 8; i++) {
			if (this.ca[i] != c.ca[i]) {
				return false;
			}
		}
		for (var i = 0; i < 12; i++) {
			if (this.ea[i] != c.ea[i]) {
				return false;
			}
		}
		return true;
	}
	CubieCube.prototype.setFlip = function(idx) {
		var parity = 0;
		for (var i = 10; i >= 0; i--, idx >>= 1) {
			this.ea[i] = this.ea[i] & 0xf | (idx & 1) << 4;
			parity ^= this.ea[i];
		}
		this.ea[11] = this.ea[11] & 0xf | parity & 0x10;
	}
	CubieCube.prototype.getFlip = function() {
		var idx = 0;
		for (var i = 0; i < 11; i++) {
			idx = idx << 1 | this.ea[i] >> 4 & 1;
		}
		return idx;
	}
	CubieCube.prototype.getFlipSym = function() {
		return FlipR2S[this.getFlip()];
	}
	CubieCube.prototype.setTwst = function(idx) {
		var twst = 15;
		for (var i = 6; i >= 0; i--, idx = ~~(idx / 3)) {
			this.ca[i] = this.ca[i] & 0xf | idx % 3 << 4;
			twst -= this.ca[i] >> 4;
		}
		this.ca[7] = this.ca[7] & 0xf | (twst % 3) << 4;
	}
	CubieCube.prototype.getTwst = function() {
		var idx = 0;
		for (var i = 0; i < 7; i++) {
			idx += (idx << 1) + (this.ca[i] >> 4);
		}
		return idx;
	}
	CubieCube.prototype.getTwstSym = function() {
		return TwstR2S[this.getTwst()];
	}
	CubieCube.prototype.setCPerm = function(idx) {
		setNPerm(this.ca, idx, 8);
	}
	CubieCube.prototype.getCPerm = function() {
		return getNPerm(this.ca, 8);
	}
	CubieCube.prototype.getCPermSym = function() {
		return ESym2CSym(EPermR2S[getNPerm(this.ca, 8)]);
	}
	CubieCube.prototype.setEPerm = function(idx) {
		setNPerm(this.ea, idx, 8);
	}
	CubieCube.prototype.getEPerm = function() {
		return getNPerm(this.ea, 8);
	}
	CubieCube.prototype.getEPermSym = function() {
		return EPermR2S[getNPerm(this.ea, 8)];
	}
	CubieCube.prototype.getSlice = function() {
		return 494 - getComb(this.ea, 8);
	}
	CubieCube.prototype.setSlice = function(idx) {
		setComb(this.ea, 494 - idx, 8);
	}
	CubieCube.prototype.getMPerm = function() {
		return getNPermFull(this.ea, 12) % 24;
	}
	CubieCube.prototype.setMPerm = function(idx) {
		setNPermFull(this.ea, idx, 12);
	}
	CubieCube.prototype.getCComb = function() {
		return getComb(this.ca, 0);
	}
	CubieCube.prototype.setCComb = function(idx) {
		setComb(this.ca, idx, 0);
	}
	CubieCube.prototype.URFConjugate = function() {
		var temps = new CubieCube();
		CubieCube.CornMult(CubieCube.urf2, this, temps);
		CubieCube.CornMult(temps, CubieCube.urf1, this);
		CubieCube.EdgeMult(CubieCube.urf2, this, temps);
		CubieCube.EdgeMult(temps, CubieCube.urf1, this);
	}
	var cornerFacelet = [
		[8, 9, 20],
		[6, 18, 38],
		[0, 36, 47],
		[2, 45, 11],
		[29, 26, 15],
		[27, 44, 24],
		[33, 53, 42],
		[35, 17, 51]
	];
	var edgeFacelet = [
		[5, 10],
		[7, 19],
		[3, 37],
		[1, 46],
		[32, 16],
		[28, 25],
		[30, 43],
		[34, 52],
		[23, 12],
		[21, 41],
		[50, 39],
		[48, 14]
	];
	CubieCube.prototype.toFaceCube = function(cFacelet, eFacelet) {
		cFacelet = cFacelet || cornerFacelet;
		eFacelet = eFacelet || edgeFacelet;
		var ts = "URFDLB";
		var f = [];
		for (var i = 0; i < 54; i++) {
			f[i] = ts[~~(i / 9)];
		}
		for (var c = 0; c < 8; c++) {
			var j = this.ca[c] & 0xf; // cornercubie with index j is at
			var ori = this.ca[c] >> 4; // Orientation of this cubie
			for (var n = 0; n < 3; n++)
				f[cFacelet[c][(n + ori) % 3]] = ts[~~(cFacelet[j][n] / 9)];
		}
		for (var e = 0; e < 12; e++) {
			var j = this.ea[e] & 0xf; // edgecubie with index j is at edgeposition
			var ori = this.ea[e] >> 4; // Orientation of this cubie
			for (var n = 0; n < 2; n++)
				f[eFacelet[e][(n + ori) % 2]] = ts[~~(eFacelet[j][n] / 9)];
		}
		return f.join("");
	}
	CubieCube.prototype.invFrom = function(cc) {
		for (var edge = 0; edge < 12; edge++) {
			this.ea[cc.ea[edge] & 0xf] = edge & 0xf | cc.ea[edge] & 0x10;
		}
		for (var corn = 0; corn < 8; corn++) {
			this.ca[cc.ca[corn] & 0xf] = corn | 0x40 >> (cc.ca[corn] >> 4) & 0x30;
		}
		return this;
	}
	CubieCube.prototype.fromFacelet = function(facelet, cFacelet, eFacelet) {
		cFacelet = cFacelet || cornerFacelet;
		eFacelet = eFacelet || edgeFacelet;
		var count = 0;
		var f = [];
		var centers = facelet[4] + facelet[13] + facelet[22] + facelet[31] + facelet[40] + facelet[49];
		for (var i = 0; i < 54; ++i) {
			f[i] = centers.indexOf(facelet[i]);
			if (f[i] == -1) {
				return -1;
			}
			count += 1 << (f[i] << 2);
		}
		if (count != 0x999999) {
			return -1;
		}
		var col1, col2, i, j, ori;
		for (i = 0; i < 8; ++i) {
			for (ori = 0; ori < 3; ++ori)
				if (f[cFacelet[i][ori]] == 0 || f[cFacelet[i][ori]] == 3)
					break;
			col1 = f[cFacelet[i][(ori + 1) % 3]];
			col2 = f[cFacelet[i][(ori + 2) % 3]];
			for (j = 0; j < 8; ++j) {
				if (col1 == ~~(cFacelet[j][1] / 9) && col2 == ~~(cFacelet[j][2] / 9)) {
					this.ca[i] = j | ori % 3 << 4;
					break;
				}
			}
		}
		for (i = 0; i < 12; ++i) {
			for (j = 0; j < 12; ++j) {
				if (f[eFacelet[i][0]] == ~~(eFacelet[j][0] / 9) && f[eFacelet[i][1]] == ~~(eFacelet[j][1] / 9)) {
					this.ea[i] = j;
					break;
				}
				if (f[eFacelet[i][0]] == ~~(eFacelet[j][1] / 9) && f[eFacelet[i][1]] == ~~(eFacelet[j][0] / 9)) {
					this.ea[i] = j | 0x10;
					break;
				}
			}
		}
	}

	function CoordCube() {
		this.twst = 0;
		this.flip = 0;
		this.slice = 0;
		this.prun = 0;
		this.twstc = 0;
		this.flipc = 0;
	}
	CoordCube.prototype.set = function(node) {
		this.twst = node.twst;
		this.flip = node.flip;
		this.slice = node.slice;
		this.prun = node.prun;
		if (USE_CONJ_PRUN) {
			this.twstc = node.twstc;
			this.flipc = node.flipc;
		}
	}
	CoordCube.prototype.calcPruning = function(isPhase1) {
		this.prun = Math.max(
			getPruningMax(SliceTwstPrunMax, SliceTwstPrun,
				(this.twst >> 3) * N_SLICE + SliceConj[this.slice << 3 | this.twst & 7]),
			getPruningMax(SliceFlipPrunMax, SliceFlipPrun,
				(this.flip >> 3) * N_SLICE + SliceConj[this.slice << 3 | this.flip & 7]),
			USE_CONJ_PRUN ? getPruningMax(TwstFlipPrunMax, TwstFlipPrun,
				(this.twstc >> 3) << 11 | FlipS2RF[this.flipc ^ (this.twstc & 7)]) : 0,
			USE_TWST_FLIP_PRUN ? getPruningMax(TwstFlipPrunMax, TwstFlipPrun,
				(this.twst >> 3) << 11 | FlipS2RF[this.flip ^ (this.twst & 7)]) : 0
		);
	}
	CoordCube.prototype.setWithPrun = function(cc, depth) {
		this.twst = cc.getTwstSym();
		this.flip = cc.getFlipSym();
		this.prun = USE_TWST_FLIP_PRUN ? getPruningMax(TwstFlipPrunMax, TwstFlipPrun,
			(this.twst >> 3) << 11 | FlipS2RF[this.flip ^ (this.twst & 7)]) : 0;
		if (this.prun > depth) {
			return false;
		}
		this.slice = cc.getSlice();
		this.prun = Math.max(this.prun,
			getPruningMax(SliceTwstPrunMax, SliceTwstPrun,
				(this.twst >> 3) * N_SLICE + SliceConj[this.slice << 3 | this.twst & 7]),
			getPruningMax(SliceFlipPrunMax, SliceFlipPrun,
				(this.flip >> 3) * N_SLICE + SliceConj[this.slice << 3 | this.flip & 7]));
		if (this.prun > depth) {
			return false;
		}
		if (USE_CONJ_PRUN) {
			var pc = new CubieCube();
			CubieCube.CornConjugate(cc, 1, pc);
			CubieCube.EdgeConjugate(cc, 1, pc);
			this.twstc = pc.getTwstSym();
			this.flipc = pc.getFlipSym();
			this.prun = Math.max(this.prun,
				getPruningMax(TwstFlipPrunMax, TwstFlipPrun,
					(this.twstc >> 3) << 11 | FlipS2RF[this.flipc ^ (this.twstc & 7)]));
		}
		return this.prun <= depth;
	}
	CoordCube.prototype.doMovePrun = function(cc, m, isPhase1) {
		this.slice = SliceMove[cc.slice * N_MOVES + m];
		this.flip = FlipMove[(cc.flip >> 3) * N_MOVES + Sym8Move[m << 3 | cc.flip & 7]] ^ (cc.flip & 7);
		this.twst = TwstMove[(cc.twst >> 3) * N_MOVES + Sym8Move[m << 3 | cc.twst & 7]] ^ (cc.twst & 7);
		this.prun = Math.max(
			getPruningMax(SliceTwstPrunMax, SliceTwstPrun,
				(this.twst >> 3) * N_SLICE + SliceConj[this.slice << 3 | this.twst & 7]),
			getPruningMax(SliceFlipPrunMax, SliceFlipPrun,
				(this.flip >> 3) * N_SLICE + SliceConj[this.slice << 3 | this.flip & 7]),
			USE_TWST_FLIP_PRUN ? getPruningMax(TwstFlipPrunMax, TwstFlipPrun,
				(this.twst >> 3) << 11 | FlipS2RF[this.flip ^ (this.twst & 7)]) : 0);
		return this.prun;
	}
	CoordCube.prototype.doMovePrunConj = function(cc, m) {
		m = SymMove[3][m];
		this.flipc = FlipMove[(cc.flipc >> 3) * N_MOVES + Sym8Move[m << 3 | cc.flipc & 7]] ^ (cc.flipc & 7);
		this.twstc = TwstMove[(cc.twstc >> 3) * N_MOVES + Sym8Move[m << 3 | cc.twstc & 7]] ^ (cc.twstc & 7);
		return getPruningMax(TwstFlipPrunMax, TwstFlipPrun,
			(this.twstc >> 3) << 11 | FlipS2RF[this.flipc ^ (this.twstc & 7)]);
	}
	Search.prototype.solution = function(facelets, maxDepth, probeMax, probeMin, verbose, firstAxisFilter, lastAxisFilter) {
		initPrunTables();
		var check = this.verify(facelets);
		if (check != 0) {
			return "Error " + Math.abs(check);
		}
		if (maxDepth === undefined) {
			maxDepth = 21;
		}
		if (probeMax === undefined) {
			probeMax = 1e9;
		}
		if (probeMin === undefined) {
			probeMin = 0;
		}
		if (verbose === undefined) {
			verbose = 0;
		}
		this.sol = maxDepth + 1;
		this.probe = 0;
		this.probeMax = probeMax;
		this.probeMin = Math.min(probeMin, probeMax);
		this.verbose = verbose;
		this.moveSol = null;
		this.isRec = false;
		this.firstFilters = [0, 0, 0, 0, 0, 0];
		this.lastFilters = [0, 0, 0, 0, 0, 0];
		for (var i = 0; i < 3; i++) {
			if (firstAxisFilter !== undefined) {
				this.firstFilters[i] |= 0xe07 << (~~(urfMove[(3 - i) % 3][firstAxisFilter * 3] / 3)) * 3;
				this.lastFilters[i + 3] |= 0xe07 << (~~(urfMove[(3 - i) % 3][firstAxisFilter * 3] / 3)) * 3;
			}
			if (lastAxisFilter !== undefined) {
				this.lastFilters[i] |= 0xe07 << (~~(urfMove[(3 - i) % 3][lastAxisFilter * 3] / 3)) * 3;
				this.firstFilters[i + 3] |= 0xe07 << (~~(urfMove[(3 - i) % 3][lastAxisFilter * 3] / 3)) * 3;
			}
		}
		this.initSearch();
		return this.search();
	}

	Search.prototype.initSearch = function() {
		this.conjMask = (TRY_INVERSE ? 0 : 0x38) | (TRY_THREE_AXES ? 0 : 0x36);
		this.maxPreMoves = this.conjMask > 7 ? 0 : MAX_PRE_MOVES;

		for (var i = 0; i < 6; i++) {
			this.urfCubieCube[i].init(this.cc.ca, this.cc.ea);
			this.urfCoordCube[i].setWithPrun(this.urfCubieCube[i], 20);
			this.cc.URFConjugate();
			if (i % 3 == 2) {
				var tmp = new CubieCube().invFrom(this.cc);
				this.cc.init(tmp.ca, tmp.ea);
			}
		}
	}

	Search.prototype.next = function(probeMax, probeMin, verbose) {
		this.probe = 0;
		this.probeMax = probeMax;
		this.probeMin = Math.min(probeMin, probeMax);
		this.moveSol = null;
		this.isRec = true;
		this.verbose = verbose;
		return this.search();
	}

	Search.prototype.verify = function(facelets) {
		if (this.cc.fromFacelet(facelets) == -1) {
			return -1;
		}
		var sum = 0;
		var edgeMask = 0;
		for (var e = 0; e < 12; e++) {
			edgeMask |= 1 << (this.cc.ea[e] & 0xf);
			sum ^= this.cc.ea[e] >> 4;
		}
		if (edgeMask != 0xfff) {
			return -2; // missing edges
		}
		if (sum != 0) {
			return -3;
		}
		var cornMask = 0;
		sum = 0;
		for (var c = 0; c < 8; c++) {
			cornMask |= 1 << (this.cc.ca[c] & 0xf);
			sum += this.cc.ca[c] >> 4;
		}
		if (cornMask != 0xff) {
			return -4; // missing corners
		}
		if (sum % 3 != 0) {
			return -5; // twisted corner
		}
		if ((getNParity(getNPermFull(this.cc.ea, 12), 12) ^ getNParity(this.cc.getCPerm(), 8)) != 0) {
			return -6; // parity error
		}
		return 0; // cube ok
	}

	Search.prototype.phase1PreMoves = function(maxl, lm, cc) {
		if (maxl == this.maxPreMoves - 1 && (this.lastFilter >> lm & 1) != 0) {
			return 1;
		}
		this.preMoveLen = this.maxPreMoves - maxl;
		if (this.isRec ? (this.depth1 == this.length1 - this.preMoveLen) :
			(this.preMoveLen == 0 || (0x36FB7 >> lm & 1) == 0)) {
			this.depth1 = this.length1 - this.preMoveLen;
			this.phase1Cubie[0].init(cc.ca, cc.ea) /* = cc*/ ;
			this.allowShorter = this.depth1 == MIN_P1LENGTH_PRE && this.preMoveLen != 0;

			if (this.nodeUD[this.depth1 + 1].setWithPrun(cc, this.depth1) &&
				this.phase1(this.nodeUD[this.depth1 + 1], this.depth1, -1) == 0) {
				return 0;
			}
		}

		if (maxl == 0 || this.preMoveLen + MIN_P1LENGTH_PRE >= this.length1) {
			return 1;
		}

		var skipMoves = 0;
		if (maxl == 1 || this.preMoveLen + 1 + MIN_P1LENGTH_PRE >= this.length1) { //last pre move
			skipMoves |= 0x36FB7; // 11 0110 1111 1011 0111
		}

		lm = ~~(lm / 3) * 3;
		for (var m = 0; m < 18; m++) {
			if (m == lm || m == lm - 9 || m == lm + 9) {
				m += 2;
				continue;
			}
			if (this.isRec && m != this.preMoves[this.maxPreMoves - maxl] || (skipMoves & 1 << m) != 0) {
				continue;
			}
			CubieCube.CornMult(moveCube[m], cc, this.preMoveCubes[maxl]);
			CubieCube.EdgeMult(moveCube[m], cc, this.preMoveCubes[maxl]);
			this.preMoves[this.maxPreMoves - maxl] = m;
			var ret = this.phase1PreMoves(maxl - 1, m, this.preMoveCubes[maxl]);
			if (ret == 0) {
				return 0;
			}
		}
		return 1;
	}

	Search.prototype.search = function() {
		for (this.length1 = this.isRec ? this.length1 : 0; this.length1 < this.sol; this.length1++) {
			for (this.urfIdx = this.isRec ? this.urfIdx : 0; this.urfIdx < 6; this.urfIdx++) {
				if ((this.conjMask & 1 << this.urfIdx) != 0) {
					continue;
				}
				this.firstFilter = this.firstFilters[this.urfIdx];
				this.lastFilter = this.lastFilters[this.urfIdx];
				if (this.phase1PreMoves(this.maxPreMoves, -30, this.urfCubieCube[this.urfIdx], 0) == 0) {
					return this.moveSol == null ? "Error 8" : this.moveSol;
				}
			}
		}
		return this.moveSol == null ? "Error 7" : this.moveSol;
	}

	Search.prototype.initPhase2Pre = function() {
		this.isRec = false;
		if (this.probe >= (this.moveSol == null ? this.probeMax : this.probeMin)) {
			return 0;
		}
		++this.probe;

		for (var i = this.valid1; i < this.depth1; i++) {
			CubieCube.CornMult(this.phase1Cubie[i], moveCube[this.move[i]], this.phase1Cubie[i + 1]);
			CubieCube.EdgeMult(this.phase1Cubie[i], moveCube[this.move[i]], this.phase1Cubie[i + 1]);
		}
		this.valid1 = this.depth1;

		var ret = this.initPhase2(this.phase1Cubie[this.depth1]);
		if (ret == 0 || this.preMoveLen == 0 || ret == 2) {
			return ret;
		}

		var m = ~~(this.preMoves[this.preMoveLen - 1] / 3) * 3 + 1;
		CubieCube.CornMult(moveCube[m], this.phase1Cubie[this.depth1], this.phase1Cubie[this.depth1 + 1]);
		CubieCube.EdgeMult(moveCube[m], this.phase1Cubie[this.depth1], this.phase1Cubie[this.depth1 + 1]);

		this.preMoves[this.preMoveLen - 1] += 2 - this.preMoves[this.preMoveLen - 1] % 3 * 2;
		ret = this.initPhase2(this.phase1Cubie[this.depth1 + 1]);
		this.preMoves[this.preMoveLen - 1] += 2 - this.preMoves[this.preMoveLen - 1] % 3 * 2;
		return ret;
	}
	Search.prototype.initPhase2 = function(phase2Cubie) {
		var p2corn = phase2Cubie.getCPermSym();
		var p2csym = p2corn & 0xf;
		p2corn >>= 4;
		var p2edge = phase2Cubie.getEPermSym();
		var p2esym = p2edge & 0xf;
		p2edge >>= 4;
		var p2mid = phase2Cubie.getMPerm();
		var prun = Math.max(
			getPruningMax(EPermCCombPPrunMax, EPermCCombPPrun,
				p2edge * N_COMB + CCombPConj[(Perm2CombP[p2corn] & 0xff) << 4 | SymMultInv[p2esym][p2csym]]),
			getPruningMax(MCPermPrunMax, MCPermPrun,
				p2corn * N_MPERM + MPermConj[p2mid << 4 | p2csym]));
		var maxDep2 = Math.min(MAX_DEPTH2, this.sol - this.length1);
		if (prun >= maxDep2) {
			return prun > maxDep2 ? 2 : 1;
		}
		var depth2;
		for (depth2 = maxDep2 - 1; depth2 >= prun; depth2--) {
			var ret = this.phase2(p2edge, p2esym, p2corn, p2csym, p2mid, depth2, this.depth1, 10);
			if (ret < 0) {
				break;
			}
			depth2 -= ret;
			this.moveSol = [];
			for (var i = 0; i < this.depth1 + depth2; i++) {
				this.appendSolMove(this.move[i]);
			}
			for (var i = this.preMoveLen - 1; i >= 0; i--) {
				this.appendSolMove(this.preMoves[i]);
			}
			this.sol = this.moveSol.length;
			this.moveSol = this.solutionToString();
		}
		if (depth2 != maxDep2 - 1) { //At least one solution has been found.
			return this.probe >= this.probeMin ? 0 : 1;
		} else {
			return 1;
		}
	}
	Search.prototype.phase1 = function(node, maxl, lm) {
		if (maxl == this.depth1 - 1 && (this.firstFilter >> lm & 1) != 0) {
			return 1;
		}
		if (node.prun == 0 && maxl < 5) {
			if (this.allowShorter || maxl == 0) {
				this.depth1 -= maxl;
				var ret = this.initPhase2Pre();
				this.depth1 += maxl;
				return ret;
			} else {
				return 1;
			}
		}
		for (var axis = 0; axis < 18; axis += 3) {
			if (axis == lm || axis == lm - 9) {
				continue;
			}
			for (var power = 0; power < 3; power++) {
				var m = axis + power;

				if (this.isRec && m != this.move[this.depth1 - maxl]) {
					continue;
				}

				var prun = this.nodeUD[maxl].doMovePrun(node, m, true);
				if (prun > maxl) {
					break;
				} else if (prun == maxl) {
					continue;
				}

				if (USE_CONJ_PRUN) {
					prun = this.nodeUD[maxl].doMovePrunConj(node, m);
					if (prun > maxl) {
						break;
					} else if (prun == maxl) {
						continue;
					}
				}
				this.move[this.depth1 - maxl] = m;
				this.valid1 = Math.min(this.valid1, this.depth1 - maxl);
				var ret = this.phase1(this.nodeUD[maxl], maxl - 1, axis);
				if (ret == 0) {
					return 0;
				} else if (ret == 2) {
					break;
				}
			}
		}
		return 1;
	}
	Search.prototype.appendSolMove = function(curMove) {
		if (this.moveSol.length == 0) {
			this.moveSol.push(curMove);
			return;
		}
		var axisCur = ~~(curMove / 3);
		var axisLast = ~~(this.moveSol.at(-1) / 3);
		if (axisCur == axisLast) {
			var pow = (curMove % 3 + this.moveSol.at(-1) % 3 + 1) % 4;
			if (pow == 3) {
				this.moveSol.pop();
			} else {
				this.moveSol.splice(-1, 1, axisCur * 3 + pow);
			}
			return;
		}
		if (this.moveSol.length > 1 &&
			axisCur % 3 == axisLast % 3 &&
			axisCur == ~~(this.moveSol.at(-2) / 3)) {
			var pow = (curMove % 3 + this.moveSol.at(-2) % 3 + 1) % 4;
			if (pow == 3) {
				this.moveSol.splice(-2, 1, this.moveSol.at(-1));
				this.moveSol.pop();
			} else {
				this.moveSol.splice(-2, 1, axisCur * 3 + pow);
			}
			return;
		}
		this.moveSol.push(curMove);
	}
	Search.prototype.phase2 = function(edge, esym, corn, csym, mid, maxl, depth, lm) {
		if (this.depth1 == 0 && depth == 1 && (this.firstFilter >> ud2std[lm] & 1) != 0) {
			return -1;
		}
		if (edge == 0 && corn == 0 && mid == 0 && (this.preMoveLen > 0 || (this.lastFilter >> ud2std[lm] & 1) == 0)) {
			return maxl;
		}
		var moveMask = ckmv2bit[lm];
		for (var m = 0; m < 10; m++) {
			if ((moveMask >> m & 1) != 0) {
				m += 0x42 >> m & 3;
				continue;
			}
			var midx = MPermMove[mid * N_MOVES2 + m];
			var cornx = CPermMove[corn * N_MOVES2 + SymMoveUD[csym][m]];
			var csymx = SymMult[cornx & 0xf][csym];
			cornx >>= 4;
			if (getPruningMax(MCPermPrunMax, MCPermPrun,
					cornx * N_MPERM + MPermConj[midx << 4 | csymx]) >= maxl) {
				continue;
			}
			var edgex = EPermMove[edge * N_MOVES2 + SymMoveUD[esym][m]];
			var esymx = SymMult[edgex & 0xf][esym];
			edgex >>= 4;
			if (getPruningMax(EPermCCombPPrunMax, EPermCCombPPrun,
					edgex * N_COMB + CCombPConj[(Perm2CombP[cornx] & 0xff) << 4 | SymMultInv[esymx][csymx]]) >= maxl) {
				continue;
			}
			var edgei = getPermSymInv(edgex, esymx, false);
			var corni = getPermSymInv(cornx, csymx, true);
			if (getPruningMax(EPermCCombPPrunMax, EPermCCombPPrun,
					(edgei >> 4) * N_COMB + CCombPConj[(Perm2CombP[corni >> 4] & 0xff) << 4 | SymMultInv[edgei & 0xf][corni & 0xf]]) >= maxl) {
				continue;
			}

			var ret = this.phase2(edgex, esymx, cornx, csymx, midx, maxl - 1, depth + 1, m);
			if (ret >= 0) {
				this.move[depth] = ud2std[m];
				return ret;
			}
		}
		return -1;
	}
	Search.prototype.solutionToString = function() {
		var sb = '';
		var urf = (this.verbose & INVERSE_SOLUTION) != 0 ? (this.urfIdx + 3) % 6 : this.urfIdx;
		if (urf < 3) {
			for (var s = 0; s < this.moveSol.length; ++s) {
				sb += move2str[urfMove[urf][this.moveSol[s]]] + ' ';
			}
		} else {
			for (var s = this.moveSol.length - 1; s >= 0; --s) {
				sb += move2str[urfMove[urf][this.moveSol[s]]] + ' ';
			}
		}
		return sb;
	}

	var moveCube = [];
	var SymCube = [];
	var SymMult = [];
	var SymMultInv = [];
	var SymMove = [];
	var SymMoveUD = [];
	var Sym8Move = [];
	var FlipS2R = [];
	var FlipR2S = [];
	var FlipSelfSym = [];
	var FlipS2RF = [];
	var TwstS2R = [];
	var TwstR2S = [];
	var TwstSelfSym = [];
	var EPermS2R = [];
	var EPermR2S = [];
	var PermSelfSym = [];
	var Perm2CombP = [];
	var PermInvEdgeSym = [];
	var TwstMove = [];
	var FlipMove = [];
	var SliceMove = [];
	var SliceConj = [];
	var SliceTwstPrun = [];
	var SliceFlipPrun = [];
	var TwstFlipPrun = [];

	//phase2
	var CPermMove = [];
	var EPermMove = [];
	var MPermMove = [];
	var MPermConj = [];
	var CCombPMove = [];
	var CCombPConj = [];
	var MCPermPrun = [];
	var EPermCCombPPrun = [];

	var TwstFlipPrunMax = 15;
	var SliceTwstPrunMax = 15;
	var SliceFlipPrunMax = 15;
	var MCPermPrunMax = 15;
	var EPermCCombPPrunMax = 15;

	{ //init move cubes
		for (var i = 0; i < 18; i++) {
			moveCube[i] = new CubieCube()
		}
		moveCube[0].initCoord(15120, 0, 119750400, 0);
		moveCube[3].initCoord(21021, 1494, 323403417, 0);
		moveCube[6].initCoord(8064, 1236, 29441808, 550);
		moveCube[9].initCoord(9, 0, 5880, 0);
		moveCube[12].initCoord(1230, 412, 2949660, 0);
		moveCube[15].initCoord(224, 137, 328552, 137);
		for (var a = 0; a < 18; a += 3) {
			for (var p = 0; p < 2; p++) {
				CubieCube.EdgeMult(moveCube[a + p], moveCube[a], moveCube[a + p + 1]);
				CubieCube.CornMult(moveCube[a + p], moveCube[a], moveCube[a + p + 1]);
			}
		}
		CubieCube.urf1 = new CubieCube().initCoord(2531, 1373, 67026819, 1367);
		CubieCube.urf2 = new CubieCube().initCoord(2089, 1906, 322752913, 2040);
	}

	function initBasic() {
		//init sym cubes
		var c = new CubieCube();
		var d = new CubieCube();
		var t;

		var f2 = new CubieCube().initCoord(28783, 0, 259268407, 0);
		var u4 = new CubieCube().initCoord(15138, 0, 119765538, 7);
		var lr2 = new CubieCube().initCoord(5167, 0, 83473207, 0);
		for (var i = 0; i < 8; i++) {
			lr2.ca[i] |= 3 << 4;
		}
		for (var i = 0; i < 16; i++) {
			SymCube[i] = new CubieCube().init(c.ca, c.ea);
			CubieCube.CornMultFull(c, u4, d);
			CubieCube.EdgeMult(c, u4, d);
			c.init(d.ca, d.ea);
			if (i % 4 == 3) {
				CubieCube.CornMultFull(c, lr2, d);
				CubieCube.EdgeMult(c, lr2, d);
				c.init(d.ca, d.ea);
			}
			if (i % 8 == 7) {
				CubieCube.CornMultFull(c, f2, d);
				CubieCube.EdgeMult(c, f2, d);
				c.init(d.ca, d.ea);
			}
		}

		// gen sym tables
		for (var i = 0; i < 16; i++) {
			SymMult[i] = [];
			SymMultInv[i] = [];
			SymMove[i] = [];
			Sym8Move[i] = [];
			SymMoveUD[i] = [];
		}
		for (var i = 0; i < 16; i++) {
			for (var j = 0; j < 16; j++) {
				SymMult[i][j] = i ^ j ^ (0x14ab4 >> j & i << 1 & 2); // SymMult[i][j] = (i ^ j ^ (0x14ab4 >> j & i << 1 & 2)));
				SymMultInv[SymMult[i][j]][j] = i;
			}
		}

		c = new CubieCube();
		for (var s = 0; s < 16; s++) {
			for (var j = 0; j < 18; j++) {
				CubieCube.CornConjugate(moveCube[j], SymMultInv[0][s], c);
				outloop: for (var m = 0; m < 18; m++) {
					for (var k = 0; k < 8; k++) {
						if (moveCube[m].ca[k] != c.ca[k]) {
							continue outloop;
						}
					}
					SymMove[s][j] = m;
					SymMoveUD[s][std2ud[j]] = std2ud[m];
					break;
				}
				if (s % 2 == 0) {
					Sym8Move[j << 3 | s >> 1] = SymMove[s][j];
				}
			}
		}

		// init sym 2 raw tables
		function initSym2Raw(N_RAW, Sym2Raw, Raw2Sym, SelfSym, coord, setFunc, getFunc) {
			var N_RAW_HALF = (N_RAW + 1) >> 1;
			var c = new CubieCube();
			var d = new CubieCube();
			var count = 0;
			var sym_inc = coord >= 2 ? 1 : 2;
			var conjFunc = coord != 1 ? CubieCube.EdgeConjugate : CubieCube.CornConjugate;

			for (var i = 0; i < N_RAW; i++) {
				if (Raw2Sym[i] !== undefined) {
					continue;
				}
				setFunc.call(c, i);
				for (var s = 0; s < 16; s += sym_inc) {
					conjFunc(c, s, d);
					var idx = getFunc.call(d);
					if (USE_TWST_FLIP_PRUN && coord == 0) {
						FlipS2RF[count << 3 | s >> 1] = idx;
					}
					if (idx == i) {
						SelfSym[count] |= 1 << (s / sym_inc);
					}
					Raw2Sym[idx] = (count << 4 | s) / sym_inc;
				}
				Sym2Raw[count++] = i;
			}
			return count;
		}

		initSym2Raw(N_FLIP, FlipS2R, FlipR2S, FlipSelfSym, 0, CubieCube.prototype.setFlip, CubieCube.prototype.getFlip);
		initSym2Raw(N_TWST, TwstS2R, TwstR2S, TwstSelfSym, 1, CubieCube.prototype.setTwst, CubieCube.prototype.getTwst);
		initSym2Raw(N_PERM, EPermS2R, EPermR2S, PermSelfSym, 2, CubieCube.prototype.setEPerm, CubieCube.prototype.getEPerm);

		var cc = new CubieCube();
		for (var i = 0; i < N_PERM_SYM; i++) {
			setNPerm(cc.ea, EPermS2R[i], 8);
			Perm2CombP[i] = getComb(cc.ea, 0) + getNParity(EPermS2R[i], 8) * 70;
			c.invFrom(cc);
			PermInvEdgeSym[i] = EPermR2S[c.getEPerm()];
		}

		// init coord tables
		c = new CubieCube();
		d = new CubieCube();
		function initSymMoveTable(moveTable, SymS2R, N_SIZE, N_MOVES, setFunc, getFunc, multFunc, ud2std) {
			for (var i = 0; i < N_SIZE; i++) {
				setFunc.call(c, SymS2R[i]);
				for (var j = 0; j < N_MOVES; j++) {
					multFunc(c, moveCube[ud2std ? ud2std[j] : j], d);
					moveTable[i * N_MOVES + j] = getFunc.call(d);
				}
			}
		}

		initSymMoveTable(FlipMove, FlipS2R, N_FLIP_SYM, N_MOVES,
			CubieCube.prototype.setFlip, CubieCube.prototype.getFlipSym, CubieCube.EdgeMult);
		initSymMoveTable(TwstMove, TwstS2R, N_TWST_SYM, N_MOVES,
			CubieCube.prototype.setTwst, CubieCube.prototype.getTwstSym, CubieCube.CornMult);
		initSymMoveTable(EPermMove, EPermS2R, N_PERM_SYM, N_MOVES2,
			CubieCube.prototype.setEPerm, CubieCube.prototype.getEPermSym, CubieCube.EdgeMult, ud2std);
		initSymMoveTable(CPermMove, EPermS2R, N_PERM_SYM, N_MOVES2,
			CubieCube.prototype.setCPerm, CubieCube.prototype.getCPermSym, CubieCube.CornMult, ud2std);

		for (var i = 0; i < N_SLICE; i++) {
			c.setSlice(i);
			for (var j = 0; j < N_MOVES; j++) {
				CubieCube.EdgeMult(c, moveCube[j], d);
				SliceMove[i * N_MOVES + j] = d.getSlice();
			}
			for (var j = 0; j < 16; j += 2) {
				CubieCube.EdgeConjugate(c, SymMultInv[0][j], d);
				SliceConj[i << 3 | j >> 1] = d.getSlice();
			}
		}

		for (var i = 0; i < N_MPERM; i++) {
			c.setMPerm(i);
			for (var j = 0; j < N_MOVES2; j++) {
				CubieCube.EdgeMult(c, moveCube[ud2std[j]], d);
				MPermMove[i * N_MOVES2 + j] = d.getMPerm();
			}
			for (var j = 0; j < 16; j++) {
				CubieCube.EdgeConjugate(c, SymMultInv[0][j], d);
				MPermConj[i << 4 | j] = d.getMPerm();
			}
		}

		for (var i = 0; i < N_COMB; i++) {
			c.setCComb(i % 70);
			for (var j = 0; j < N_MOVES2; j++) {
				CubieCube.CornMult(c, moveCube[ud2std[j]], d);
				CCombPMove[i * N_MOVES2 + j] = d.getCComb() + 70 * ((0xA5 >> j & 1) ^ ~~(i / 70));
			}
			for (var j = 0; j < 16; j++) {
				CubieCube.CornConjugate(c, SymMultInv[0][j], d);
				CCombPConj[i << 4 | j] = d.getCComb() + 70 * ~~(i / 70);
			}
		}
	}

	//init pruning tables
	var InitPrunProgress = -1;

	function initRawSymPrun(PrunTable, N_RAW, N_SYM, RawMove, RawConj, SymMove, SelfSym, PrunFlag) {
		var SYM_SHIFT = PrunFlag & 0xf;
		var SYM_E2C_MAGIC = ((PrunFlag >> 4) & 1) == 1 ? 0x00DDDD00 : 0x00000000;
		var IS_PHASE2 = ((PrunFlag >> 5) & 1) == 1;
		var INV_DEPTH = PrunFlag >> 8 & 0xf;
		var MAX_DEPTH = PrunFlag >> 12 & 0xf;
		var MIN_DEPTH = PrunFlag >> 16 & 0xf;

		var SYM_MASK = (1 << SYM_SHIFT) - 1;
		var ISTFP = RawMove == null;
		var N_SIZE = N_RAW * N_SYM;
		var N_MOVES = IS_PHASE2 ? 10 : 18;
		var NEXT_AXIS_MAGIC = N_MOVES == 10 ? 0x42 : 0x92492;

		var depth = getPruning(PrunTable, N_SIZE) - 1;

		if (depth == -1) {
			for (var i = 0; i < (N_SIZE >> 3) + 1; i++) {
				PrunTable[i] = -1;
			}
			setPruning(PrunTable, 0, 0 ^ 0xf);
			depth = 0;
		} else {
			setPruning(PrunTable, N_SIZE, 0xf ^ (depth + 1));
		}

		var SEARCH_DEPTH = PARTIAL_INIT_LEVEL > 0 ?
			Math.min(Math.max(depth + 1, MIN_DEPTH), MAX_DEPTH) : MAX_DEPTH;

		while (depth < SEARCH_DEPTH) {
			var inv = depth > INV_DEPTH;
			var select = inv ? 0xf : depth;
			var selArrMask = select * 0x11111111;
			var check = inv ? depth : 0xf;
			depth++;
			InitPrunProgress++;
			var xorVal = depth ^ 0xf;
			var done = 0;
			var val = 0;
			for (var i = 0; i < N_SIZE; i++, val >>= 4) {
				if ((i & 7) == 0) {
					val = PrunTable[i >> 3];
					if (!hasZero(val ^ selArrMask)) {
						i += 7;
						continue;
					}
				}
				if ((val & 0xf) != select) {
					continue;
				}
				var raw = i % N_RAW;
				var sym = ~~(i / N_RAW);
				var flip = 0,
					fsym = 0;
				if (ISTFP) {
					flip = FlipR2S[raw];
					fsym = flip & 7;
					flip >>= 3;
				}

				for (var m = 0; m < N_MOVES; m++) {
					var symx = SymMove[sym * N_MOVES + m];
					var rawx;
					if (ISTFP) {
						rawx = FlipS2RF[
							FlipMove[flip * N_MOVES + Sym8Move[m << 3 | fsym]] ^
							fsym ^ (symx & SYM_MASK)];
					} else {
						rawx = RawConj[RawMove[raw * N_MOVES + m] << SYM_SHIFT | symx & SYM_MASK];
					}
					symx >>= SYM_SHIFT;
					var idx = symx * N_RAW + rawx;
					var prun = getPruning(PrunTable, idx);
					if (prun != check) {
						if (prun < depth - 1) {
							m += NEXT_AXIS_MAGIC >> m & 3;
						}
						continue;
					}
					done++;
					if (inv) {
						setPruning(PrunTable, i, xorVal);
						break;
					}
					setPruning(PrunTable, idx, xorVal);
					for (var j = 1, selfSym = SelfSym[symx];
							(selfSym >>= 1) != 0; j++) {
						if ((selfSym & 1) != 1) {
							continue;
						}
						var idxx = symx * N_RAW;
						if (ISTFP) {
							idxx += FlipS2RF[FlipR2S[rawx] ^ j];
						} else {
							idxx += RawConj[rawx << SYM_SHIFT | (j ^ (SYM_E2C_MAGIC >> (j << 1) & 3))];
						}
						if (getPruning(PrunTable, idxx) == check) {
							setPruning(PrunTable, idxx, xorVal);
							done++;
						}
					}
				}
			}
			// console.log(depth, done, InitPrunProgress);
		}
		setPruning(PrunTable, N_SIZE, (depth + 1) ^ 0xf);
		return depth + 1;
	}

	function doInitPrunTables(targetProgress) {
		if (USE_TWST_FLIP_PRUN) {
			TwstFlipPrunMax = initRawSymPrun(
				TwstFlipPrun, N_FLIP, N_TWST_SYM,
				null, null,
				TwstMove, TwstSelfSym, 0x19603
			);
		}
		if (InitPrunProgress > targetProgress) {
			return;
		}
		SliceTwstPrunMax = initRawSymPrun(
			SliceTwstPrun, N_SLICE, N_TWST_SYM,
			SliceMove, SliceConj,
			TwstMove, TwstSelfSym, 0x69603
		);
		if (InitPrunProgress > targetProgress) {
			return;
		}
		SliceFlipPrunMax = initRawSymPrun(
			SliceFlipPrun, N_SLICE, N_FLIP_SYM,
			SliceMove, SliceConj,
			FlipMove, FlipSelfSym, 0x69603
		);
		if (InitPrunProgress > targetProgress) {
			return;
		}
		MCPermPrunMax = initRawSymPrun(
			MCPermPrun, 24, N_PERM_SYM,
			MPermMove, MPermConj,
			CPermMove, PermSelfSym, 0x8ea34
		);
		if (InitPrunProgress > targetProgress) {
			return;
		}
		EPermCCombPPrunMax = initRawSymPrun(
			EPermCCombPPrun, N_COMB, N_PERM_SYM,
			CCombPMove, CCombPConj,
			EPermMove, PermSelfSym, 0x7d824
		);
	}

	function initPrunTables() {
		if (InitPrunProgress < 0) {
			initBasic();
			InitPrunProgress = 0;
		}
		if (InitPrunProgress == 0) {
			doInitPrunTables(99);
		} else if (InitPrunProgress < 54) {
			doInitPrunTables(InitPrunProgress);
		} else {
			return true;
		}
		return false;
	}

	function randomCube() {
		var ep, cp;
		var eo = ~~(Math.random() * 2048);
		var co = ~~(Math.random() * 2187);
		do {
			ep = ~~(Math.random() * fact[12]);
			cp = ~~(Math.random() * fact[8]);
		} while (getNParity(cp, 8) != getNParity(ep, 12));
		var cc = new CubieCube().initCoord(cp, co, ep, eo);
		return cc.toFaceCube();
	}
	function fromScramble(s) {
		var j = 0;
		var axis = -1;
		var c1 = new CubieCube();
		var c2 = new CubieCube();
		for (var i = 0; i < s.length; i++) {
			switch (s[i]) {
				case 'U':
				case 'R':
				case 'F':
				case 'D':
				case 'L':
				case 'B':
					axis = "URFDLB".indexOf(s[i]) * 3;
					break;
				case ' ':
					if (axis != -1) {
						CubieCube.CornMult(c1, moveCube[axis], c2);
						CubieCube.EdgeMult(c1, moveCube[axis], c2);
						c1.init(c2.ca, c2.ea);
					}
					axis = -1;
					break;
				case '2':
					axis++;
					break;
				case '\'':
					axis += 2;
					break;
				default:
					continue;
			}
		}
		if (axis != -1) {
			CubieCube.CornMult(c1, moveCube[axis], c2);
			CubieCube.EdgeMult(c1, moveCube[axis], c2);
			c1.init(c2.ca, c2.ea);
		}
		return c2.toFaceCube();
	}

	return {
		Search: Search,
		solve: function(facelet) {
			return new Search().solution(facelet);
		},
		randomCube: randomCube,
		fromScramble: fromScramble,
		initFull: function() {
			PARTIAL_INIT_LEVEL = 0;
			initPrunTables();
		},
		INVERSE_SOLUTION: INVERSE_SOLUTION
	}
})();

if (typeof module !== 'undefined' && typeof module.exports !== 'undefined') {
	module.exports = min2phase;
}

"use strict";

var cubeutil = (function() {

	function toEqus(facelet) {
		var col2equ = {};
		for (var i = 0; i < facelet.length; i++) {
			var col = facelet[i];
			if (col == '-') {
				continue;
			}
			col2equ[col] = col2equ[col] || [];
			col2equ[col].push(i);
		}
		var equs = [];
		for (var col in col2equ) {
			if (col2equ[col].length > 1) {
				equs.push(col2equ[col]);
			}
		}
		return equs;
	}

	var crossMask = toEqus("----U--------R--R-----F--F--D-DDD-D-----L--L-----B--B-");
	var f2l1Mask = toEqus("----U-------RR-RR-----FF-FF-DDDDD-D-----L--L-----B--B-");
	var f2l2Mask = toEqus("----U--------R--R----FF-FF-DD-DDD-D-----LL-LL----B--B-");
	var f2l3Mask = toEqus("----U--------RR-RR----F--F--D-DDD-DD----L--L----BB-BB-");
	var f2l4Mask = toEqus("----U--------R--R-----F--F--D-DDDDD----LL-LL-----BB-BB");
	var f2lMask = toEqus("----U-------RRRRRR---FFFFFFDDDDDDDDD---LLLLLL---BBBBBB");
	var ollMask = toEqus("UUUUUUUUU---RRRRRR---FFFFFFDDDDDDDDD---LLLLLL---BBBBBB");
	var eollMask = toEqus("-U-UUU-U----RRRRRR---FFFFFFDDDDDDDDD---LLLLLL---BBBBBB");
	var cpllMask = toEqus("UUUUUUUUUr-rRRRRRRf-fFFFFFFDDDDDDDDDl-lLLLLLLb-bBBBBBB");
	var roux1Mask = toEqus("---------------------F--F--D--D--D-----LLLLLL-----B--B");
	var roux2Mask = toEqus("------------RRRRRR---F-FF-FD-DD-DD-D---LLLLLL---B-BB-B");
	var roux3Mask = toEqus("U-U---U-Ur-rRRRRRRf-fF-FF-FD-DD-DD-Dl-lLLLLLLb-bB-BB-B");
	var LLPattern = "012345678cdeRRRRRR9abFFFFFFDDDDDDDDDijkLLLLLLfghBBBBBB";
	var c2LLPattern = "0-1---2-36-7---R-R4-5---F-FD-D---D-Da-b---L-L8-9---B-B";
	var c2LLMask = toEqus("---------------R-R------F-FD-D---D-D------L-L------B-B");
	var solvedMask = toEqus(mathlib.SOLVED_FACELET);

	var cubeRots = (function genRots() {

		function faceletRot(facelet, rot) {
			var ret = [];
			for (var i = 0; i < 54; i++) {
				ret[rot[i]] = facelet[i];
			}
			return ret;
		}

		var cubeRotY = mathlib.CubieCube.rotCube[3].toPerm(); mathlib.circle(cubeRotY, 13, 49, 40, 22);
		var cubeRotX = mathlib.CubieCube.rotCube[15].toPerm(); mathlib.circle(cubeRotX, 4, 22, 31, 49);
		var cubeRotZ = mathlib.CubieCube.rotCube[17].toPerm(); mathlib.circle(cubeRotZ, 4, 40, 31, 13);

		var ret = [];
		var cur = [];
		for (var i = 0; i < 54; i++) {
			cur[i] = i;
		}
		for (var a = 0; a < 24; a++) {
			ret[a] = cur.slice();
			cur = faceletRot(cur, (a & 1) ? cubeRotX : cubeRotZ);
			if (a % 6 == 5) {
				cur = faceletRot(cur, cubeRotZ);
				cur = faceletRot(cur, cubeRotZ);
			}
			if (a % 12 == 11) {
				cur = faceletRot(cur, cubeRotY);
				cur = faceletRot(cur, cubeRotY);
			}
		}
		return ret;
	})();

	function solvedProgressCubie(param, mask) {
		var faceGetter = function(i) {
			var src = mathlib.CubieCube.faceMap[i];
			var ret = 0;
			if (!src) {
				ret = i;
			} else if (src[0] == 0) {
				var val = facelet.ca[src[1]];
				ret = mathlib.CubieCube.cFacelet[val & 0x7][(3 - (val >> 3) + src[2]) % 3];
			} else if (src[0] == 1) {
				var val = facelet.ea[src[1]];
				ret = mathlib.CubieCube.eFacelet[val >> 1][(val & 1) ^ src[2]];
			}
			return ~~(ret / 9);
		};
		var cubeRot = cubeRots[param[1]];
		var facelet = param[0];
		mask = mask || solvedMask;
		for (var i = 0; i < mask.length; i++) {
			var equ = mask[i];
			var col = faceGetter(cubeRot[equ[0]]);
			for (var j = 1; j < equ.length; j++) {
				if (faceGetter(cubeRot[equ[j]]) != col) {
					return 1;
				}
			}
		}
		return 0;
	}

	function solvedProgress(param, mask) {
		if (param[0] instanceof mathlib.CubieCube) {
			return solvedProgressCubie(param, mask);
		}
		var cubeRot = cubeRots[param[1]];
		var facelet = param[0];
		mask = mask || solvedMask;
		for (var i = 0; i < mask.length; i++) {
			var equ = mask[i];
			var col = facelet[cubeRot[equ[0]]];
			for (var j = 1; j < equ.length; j++) {
				if (facelet[cubeRot[equ[j]]] != col) {
					return 1;
				}
			}
		}
		return 0;
	}

	//return 9: nothing, 8: cross solved, 4~7: nth f2l solved, 2~3 c/e oll solved, 1 cpll solved, 0: solved
	function getCF4O2P2Progress(param) {
		if (solvedProgress(param, crossMask)) {
			return 9;
		} else if (solvedProgress(param, f2lMask)) {
			return 4 +
				solvedProgress(param, f2l1Mask) +
				solvedProgress(param, f2l2Mask) +
				solvedProgress(param, f2l3Mask) +
				solvedProgress(param, f2l4Mask);
		} else if (solvedProgress(param, eollMask)) {
			return 4;
		} else if (solvedProgress(param, ollMask)) {
			return 3;
		} else if (solvedProgress(param, cpllMask)) {
			return 2;
		} else if (solvedProgress(param)) {
			return 1;
		}
		return 0;
	}

	//return 7: nothing, 6: cross solved, 2~5: nth f2l solved, 1 oll solved, 0: solved
	function getCF4OPProgress(param) {
		if (solvedProgress(param, crossMask)) {
			return 7;
		} else if (solvedProgress(param, f2lMask)) {
			return 2 +
				solvedProgress(param, f2l1Mask) +
				solvedProgress(param, f2l2Mask) +
				solvedProgress(param, f2l3Mask) +
				solvedProgress(param, f2l4Mask);
		} else if (solvedProgress(param, ollMask)) {
			return 2;
		} else if (solvedProgress(param)) {
			return 1;
		}
		return 0;
	}

	//return 4: nothing, 3: cross solved, 2: f2l solved, 1 oll solved, 0: solved
	function getCFOPProgress(param) {
		if (solvedProgress(param, crossMask)) {
			return 4;
		} else if (solvedProgress(param, f2lMask)) {
			return 3;
		} else if (solvedProgress(param, ollMask)) {
			return 2;
		} else if (solvedProgress(param)) {
			return 1;
		}
		return 0;
	}

	//return 6: nothing, 5: cross solved, 2~4: nth f2l solved, 1 eoll solved, 0: solved
	function getCF3ZBProgress(param) {
		if (solvedProgress(param, crossMask)) {
			return 6;
		} else if (solvedProgress(param, eollMask)) {
			return 1 + Math.max(1,
				solvedProgress(param, f2l1Mask) +
				solvedProgress(param, f2l2Mask) +
				solvedProgress(param, f2l3Mask) +
				solvedProgress(param, f2l4Mask)
			);
		} else if (solvedProgress(param)) {
			return 1;
		}
		return 0;
	}

	//return 2: nothing, 1: f2l solved, 0: solved
	function getFPProgress(param) {
		if (solvedProgress(param, f2lMask)) {
			return 2;
		} else if (solvedProgress(param)) {
			return 1;
		}
		return 0;
	}

	//return 4: nothing, 3: block1, 2: block2, 1: cll, 0: solved
	function getRouxProgress(param) {
		if (solvedProgress(param, roux1Mask)) {
			return 4;
		} else if (solvedProgress(param, roux2Mask)) {
			return 3;
		} else if (solvedProgress(param, roux3Mask)) {
			return 2;
		} else if (solvedProgress(param)) {
			return 1;
		}
		return 0;
	}

	var stepParams = {
		'cross': [6, crossMask],
		'f2l':   [6, f2lMask],
		'oll':   [6, ollMask],
		'eoll':  [6, eollMask],
		'cpll':  [6, cpllMask],
		'fb':    [24, roux1Mask],
		'sb':    [24, roux2Mask],
		'cmll':  [24, roux3Mask]
	};

	//return 0: solved, 1 not solved
	function calcStepProgress(step, param) {
		if (step in stepParams) {
			return solvedProgress(param, stepParams[step][1]);
		}
		return solvedProgress(param);
	}

	function getStepProgress(step, facelet, n_axis) {
		if (!n_axis) {
			n_axis = (step in stepParams) ? stepParams[step][0] : 1;
		}
		return getProgressNAxis(facelet, calcStepProgress.bind(null, step), n_axis);
	}

	//6 axes: D -x-> B -z-> R -x-> U -z-> F -x-> L -z-> D
	function getProgressNAxis(facelet, process, n_axis) {
		var minRet = 99;
		for (var a = 0; a < n_axis; a++) {
			minRet = Math.min(minRet, process([facelet, a]));
		}
		return minRet;
	}

	var centerRot = [
		[0, 2, 4, 3, 5, 1], // y'
		[5, 1, 0, 2, 4, 3], // x'
		[4, 0, 2, 1, 3, 5] // z
	];

	function moveSeq2str(moveSeq) {
		return $.map(moveSeq, function(val) {
			return val[0].trim() + '@' + val[1];
		}).join(' ');
	}

	//rawMoveSeqs: [moveseq, moveseq, ..., moveseq]
	//moveseq: [[move, timestamp], [move, timestamp], ..., [move, timestamp]]
	//move: string [URFDLB][ 2']
	//return: [[movestr, length], [movestr, length], ..., [movestr, length]]
	function getPrettyMoves(rawMoveSeqs) {
		// console.log(rawAlgs);
		var center = [0, 1, 2, 3, 4, 5];

		return $.map(rawMoveSeqs, function(moveSeq, seqIdx) {
			var ret = [];

			function pushSol(axis, pow) {
				if (ret.length == 0 || ~~(ret.at(-1) / 3) != axis) {
					ret.push(axis * 3 + pow);
					return;
				}
				pow = (pow + ret.at(-1) % 3 + 1) % 4;
				if (pow == 3) {
					ret.pop();
				} else {
					ret.splice(-1, 1, axis * 3 + pow);
				}
			}

			for (var i = 0; i < moveSeq.length; i++) {
				var axis = center.indexOf("URFDLB".indexOf(moveSeq[i][0][0]));
				var pow = " 2'".indexOf(moveSeq[i][0][1]) % 3;
				if (i == moveSeq.length - 1 || moveSeq[i + 1][1] - moveSeq[i][1] > 100) {
					pushSol(axis, pow);
					continue;
				}
				var axis2 = center.indexOf("URFDLB".indexOf(moveSeq[i + 1][0][0]));
				var pow2 = " 2'".indexOf(moveSeq[i + 1][0][1]) % 3;
				if (axis != axis2 && axis % 3 == axis2 % 3 && pow + pow2 == 2) {
					var axisM = axis % 3;
					var powM = (pow - 1) * [1, 1, -1, -1, -1, 1][axis] + 1;
					pushSol(axisM + 6, powM)
					for (var p = 0; p < powM + 1; p++) {
						var center_ = [];
						for (var c = 0; c < 6; c++) {
							center_[c] = center[centerRot[axisM][c]];
						}
						center = center_;
					}
					i++;
					continue;
				}
				pushSol(axis, pow);
			}
			return [
				[$.map(ret, function(val) {
					return "URFDLBEMS".charAt(~~(val / 3)) + " 2'".charAt(val % 3);
				}).join(""), ret.length]
			];
		});
	}

	var identPLL = (function() {
		var pllPattern = [];
		return function(facelet) {
			for (var i = pllPattern.length; i < 22; i++) {
				var param = i == 21 ? 'UUUUUUUUUFFFRRRBBBLLL' : scramble_333.getPLLImage(i)[0];
				pllPattern.push(toEqus(LLPattern.replace(/[0-9a-z]/g, function(v) {
					return param[parseInt(v, 36)].toLowerCase();
				})));
			}
			return searchCaseByPattern(facelet, ollMask, pllPattern);
		}
	})();

	var identOLL = (function() {
		var ollPattern = [];
		return function(facelet) {
			for (var i = ollPattern.length; i < 58; i++) {
				var param = scramble_333.getOLLImage(i)[0].replace(/G/g, '-');
				ollPattern.push(toEqus(LLPattern.replace(/[0-9a-z]/g, function(v) {
					return param[parseInt(v, 36)].toLowerCase();
				})));
			}
			return searchCaseByPattern(facelet, f2lMask, ollPattern);
		}
	})();

	var identCOLL = (function() {
		var collPattern = [];
		return function(facelet) {
			for (var i = collPattern.length; i < 43; i++) {
				var param = scramble_333.getCOLLImage('D', i)[0].replace(/G/g, '-');
				collPattern.push(toEqus(LLPattern.replace(/[0-9a-z]/g, function(v) {
					return param[parseInt(v, 36)].toLowerCase();
				})));
			}
			return searchCaseByPattern(facelet, eollMask, collPattern);
		}
	})();

	var identZBLL = (function() {
		var zbllPattern = [];
		return function(facelet) {
			for (var i = zbllPattern.length; i < 493; i++) {
				var param = scramble_333.getZBLLImage(i)[0].replace(/G/g, '-');
				zbllPattern.push(toEqus(LLPattern.replace(/[0-9a-z]/g, function(v) {
					return param[parseInt(v, 36)].toLowerCase();
				})));
			}
			return searchCaseByPattern(facelet, eollMask, zbllPattern);
		}
	})();

	var identC2CLL = (function() {
		var cllPattern = [];
		return function(facelet) {
			for (var i = cllPattern.length; i < 40; i++) {
				var param = scramble_222.getEGLLImage(i)[0].replace(/G/g, '-');
				cllPattern.push(toEqus(c2LLPattern.replace(/[0-9a-z]/g, function(v) {
					return param[parseInt(v, 36)].toLowerCase();
				})));
			}
			return searchCaseByPattern(facelet, c2LLMask, cllPattern);
		}
	})();

	function searchCaseByPattern(facelet, baseMask, patterns) {
		var chkList = [];
		for (var a = 0; a < 24; a++) {
			if (solvedProgress([facelet, a], baseMask) == 0) {
				chkList.push(a);
			}
		}
		for (var i = 0; i < patterns.length; i++) {
			for (var j = 0; j < chkList.length; j++) {
				if (solvedProgress([facelet, chkList[j]], patterns[i]) == 0) {
					return i;
				}
			}
		}
		return -1;
	}

	function identStep(step, facelet) {
		switch (step) {
			case 'PLL':
				return identPLL(facelet);
			case 'OLL':
				return identOLL(facelet);
			case 'COLL':
				return identCOLL(facelet);
			case 'ZBLL':
				return identZBLL(facelet);
			case 'C2CLL':
				return identC2CLL(facelet);
		}
	}

	function getIdentData(method) {
		var identData = {
			//name: [ident, genImg, startIdx, endIdx, stageIdx]
			'PLL': [identPLL, scramble_333.getPLLImage, 0, 21, 0],
			'OLL': [identOLL, scramble_333.getOLLImage, 1, 58, 1],
			'COLL': [identCOLL, scramble_333.getCOLLImage.bind(null, 'D'), 0, 40, 1],
			'ZBLL': [identZBLL, scramble_333.getZBLLImage, 0, 493, 0],
			'CLL': [identC2CLL, scramble_222.getEGLLImage, 0, 40, 1]
		};
		return method ? identData[method] : identData;
	}

	function getScrambledState(scramble, reqFace) {
		var scrType = scramble[0];
		var scrSeq = scramble[1];
		if (!tools.isPuzzle('333', scramble)) {
			return;
		}
		var scr = parseScramble(scrSeq, "URFDLB");
		var c = new mathlib.CubieCube();
		var d = new mathlib.CubieCube();
		c.ori = 0;
		for (var i = 0; i < scr.length; i++) {
			var axis = scr[i][0];
			var pow = scr[i][2];
			var m = axis * 3 + pow - 1;
			if (m < 0 || m >= 18) {
				continue;
			}
			if (scr[i][1] == 2) { //Xw
				var rot = [3, 15, 17, 1, 11, 23][axis];
				for (var j = 0; j < pow; j++) {
					c.ori = mathlib.CubieCube.rotMult[rot][c.ori];
				}
				m = mathlib.CubieCube.rotMulM[c.ori][(axis + 3) % 6 * 3 + pow - 1];
			}
			mathlib.CubieCube.CubeMult(c, mathlib.CubieCube.moveCube[m], d);
			c.init(d.ca, d.ea);
		}
		return reqFace ? c.toFaceCube() : c;
	}

	function getProgress(facelet, method) {
		switch (method) {
			case 'cfop':
				return getProgressNAxis(facelet, getCFOPProgress, 6);
			case 'fp':
				return getProgressNAxis(facelet, getFPProgress, 6);
			case 'cf4op':
				return getProgressNAxis(facelet, getCF4OPProgress, 6);
			case 'roux':
				return getProgressNAxis(facelet, getRouxProgress, 24);
			case 'cf4o2p2':
				return getProgressNAxis(facelet, getCF4O2P2Progress, 6);
			case 'cf3zb':
				return getProgressNAxis(facelet, getCF3ZBProgress, 6);
			case 'n':
				return getProgressNAxis(facelet, solvedProgress, 1);
		}
	}

	function getStepNames(method) {
		switch (method) {
			case 'cfop':
				return ['pll', 'oll', 'f2l', 'cross'];
			case 'fp':
				return ['op', 'cf'];
			case 'cf4op':
				return ['pll', 'oll', 'f2l-4', 'f2l-3', 'f2l-2', 'f2l-1', 'cross'];
			case 'roux':
				return ['l6e', 'cmll', 'sb', 'fb'];
			case 'cf4o2p2':
				return ['pll', 'cpll', 'oll', 'eoll', 'f2l-4', 'f2l-3', 'f2l-2', 'f2l-1', 'cross'];
			case 'cf3zb':
				return ['zbll', 'zbf2l', 'f2l-3', 'f2l-2', 'f2l-1', 'cross'];
			case 'n':
				return ['solve'];
		}
	}

	function getStepCount(method) {
		var stepNames = getStepNames(method);
		return stepNames ? stepNames.length : 0;
	}

	function getPrettyReconstruction(rawMoves, method) {
		var prettySolve = "";
		var prettyMoves = getPrettyMoves(rawMoves);
		var stepNames = getStepNames(method).reverse();
		var totalMoves = 0;
		for (var i = 0; i < prettyMoves.length; i++) {
			totalMoves += prettyMoves[i][1];
			prettySolve += prettyMoves[i][0].replace(/ /g, '') + (stepNames[i] ? " // " + stepNames[i] + " " + prettyMoves[i][1] + " move(s)" : "") + "\n";
		}
		return {
			prettySolve: prettySolve,
			totalMoves: totalMoves
		};
	}

	var scrambleReg = /^([\d]+(?:-\d+)?)?([FRUBLDfrubldzxySME])(?:([w])|&sup([\d]);)?([2'])?$/;

	function parseScramble(scramble, moveMap, addPreScr) {
		scramble = scramble || '';
		if (addPreScr) {
			scramble = kernel.getProp(tools.isCurTrainScramble() ? 'preScrT' : 'preScr') + ' ' + scramble;
		}
		var moveseq = [];
		var moves = scramble.split(' ');
		var m, w, f, p;
		for (var s = 0; s < moves.length; s++) {
			m = scrambleReg.exec(moves[s]);
			if (m == null) {
				continue;
			}
			f = "FRUBLDfrubldzxySME".indexOf(m[2]);
			if (f > 14) {
				p = "2'".indexOf(m[5] || 'X') + 2;
				f = [0, 4, 5][f % 3];
				moveseq.push([moveMap.indexOf("FRUBLD".charAt(f)), 2, p]);
				moveseq.push([moveMap.indexOf("FRUBLD".charAt(f)), 1, 4 - p]);
				continue;
			}
			w = (m[1] || '').split('-');
			var w2 = ~~w[1] || -1;
			w = f < 12 ? (~~w[0] || ~~m[4] || ((m[3] == "w" || f > 5) && 2) || 1) : -1;
			p = (f < 12 ? 1 : -1) * ("2'".indexOf(m[5] || 'X') + 2);
			moveseq.push([moveMap.indexOf("FRUBLD".charAt(f % 6)), w, p, w2]);
		}
		return moveseq;
	}

	function getConjMoves(moves, inv, conj) {
		if (!moves) {
			return moves;
		}
		if (conj === undefined) {
			conj = getPreConj();
		}
		if (inv) {
			conj = mathlib.CubieCube.rotMulI[0][conj || 0];
		}
		return moves.replace(/[URFDLB]/g, function(face) {
			return "URFDLB".charAt(mathlib.CubieCube.rotMulM[conj]["URFDLB".indexOf(face) * 3] / 3);
		});
	}

	function getPreConj() { // TODO 24 cases, use map insetad of calculation
		var preScr = kernel.getProp(tools.isCurTrainScramble() ? 'preScrT' : 'preScr', '').split(' ');
		var cc = new mathlib.CubieCube();
		for (var i = 0; i < preScr.length; i++) {
			cc.selfMoveStr(preScr[i]);
		}
		return cc.ori || 0;
	}

	function getAlgCubingUrl(alg, setup, puzzle) {
		var dim = {
			'222': '2x2x2',
			'333': '3x3x3',
			'444': '4x4x4',
			'555': '5x5x5',
			'666': '6x6x6',
			'777': '7x7x7'
		};
		if (puzzle && !dim[puzzle] && typeof tools != 'undefined' && tools.puzzleType) {
			puzzle = tools.puzzleType(puzzle) || puzzle;
		}
		if (!puzzle || !dim[puzzle]) {
			puzzle = (typeof tools != 'undefined' && tools.getCurPuzzle && tools.getCurPuzzle()) || '333';
		}
		var url = 'https://alg.cubing.net/?alg=' + encodeURIComponent(alg || '') +
			'&setup=' + encodeURIComponent(setup || '');
		if (dim[puzzle]) {
			url += '&puzzle=' + dim[puzzle];
		}
		return url;
	}

	function cornersEqual(a, b) {
		for (var i = 0; i < 8; i++) {
			if (a.ca[i] != b.ca[i]) {
				return false;
			}
		}
		return true;
	}

	function cubiesMatch(a, b, cornerOnly) {
		return cornerOnly ? cornersEqual(a, b) : a.isEqual(b);
	}

	function isFaceletSolved(facelet, cornerOnly) {
		if (!cornerOnly) {
			return facelet == mathlib.SOLVED_FACELET;
		}
		for (var i = 0; i < 54; i++) {
			if (i % 9 % 2 == 0 && facelet[i] != mathlib.SOLVED_FACELET[i]) {
				return false;
			}
		}
		return true;
	}

	return {
		getProgress: getProgress,
		getStepNames: getStepNames,
		getStepCount: getStepCount,
		getStepProgress: getStepProgress,
		getPrettyMoves: getPrettyMoves,
		getPrettyReconstruction: getPrettyReconstruction,
		moveSeq2str: moveSeq2str,
		getScrambledState: getScrambledState,
		identStep: identStep,
		getIdentData: getIdentData,
		parseScramble: parseScramble,
		getConjMoves: getConjMoves,
		getPreConj: getPreConj,
		getAlgCubingUrl: getAlgCubingUrl,
		cubiesMatch: cubiesMatch,
		isFaceletSolved: isFaceletSolved
	}
})();

"use strict";

var cross = (function(createMove, edgeMove, createPrun, setNPerm, getNPerm, Cnk, getPruning) {
	var permPrun, flipPrun, ecPrun, fullPrun;
	var cmv = [];
	var pmul = [];
	var fmul = [];

	var e1mv = [];
	var c1mv = [];
	var xxPrun01 = [];
	var xxPrun02 = [];

	function pmv(a, c) {
		var b = cmv[c][~~(a / 24)];
		return 24 * ~~(b / 384) + pmul[a % 24][(b >> 4) % 24]
	}

	function fmv(b, c) {
		var a = cmv[c][b >> 4];
		return ~~(a / 384) << 4 | fmul[b & 15][(a >> 4) % 24] ^ a & 15
	}

	function i2f(a, c) {
		for (var b = 3; 0 <= b; b--) c[b] = a & 1, a >>= 1
	}

	function f2i(c) {
		for (var a = 0, b = 0; 4 > b; b++) a <<= 1, a |= c[b];
		return a
	}

	function fullmv(idx, move) {
		var slice = cmv[move][~~(idx / 384)];
		var flip = fmul[idx & 15][(slice >> 4) % 24] ^ slice & 15;
		var perm = pmul[(idx >> 4) % 24][(slice >> 4) % 24];
		return (~~(slice / 384)) * 384 + 16 * perm + flip;
	}

	function init() {
		init = $.noop;
		for (var i = 0; i < 24; i++) {
			pmul[i] = [];
		}
		for (var i = 0; i < 16; i++) {
			fmul[i] = [];
		}
		var pm1 = [];
		var pm2 = [];
		var pm3 = [];
		for (var i = 0; i < 24; i++) {
			for (var j = 0; j < 24; j++) {
				setNPerm(pm1, i, 4);
				setNPerm(pm2, j, 4);
				for (var k = 0; k < 4; k++) {
					pm3[k] = pm1[pm2[k]];
				}
				pmul[i][j] = getNPerm(pm3, 4);
				if (i < 16) {
					i2f(i, pm1);
					for (var k = 0; k < 4; k++) {
						pm3[k] = pm1[pm2[k]];
					}
					fmul[i][j] = f2i(pm3);
				}
			}
		}
		createMove(cmv, 495, getmv);

		permPrun = [];
		flipPrun = [];
		createPrun(permPrun, 0, 11880, 5, pmv);
		createPrun(flipPrun, 0, 7920, 6, fmv);

		//combMove[comb][m] = comb*, flip*, perm*
		//newcomb = comb*, newperm = perm x perm*, newflip = flip x perm* ^ flip*
		function getmv(comb, m) {
			var arr = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
			var r = 4;
			for (var i = 0; i < 12; i++) {
				if (comb >= Cnk[11 - i][r]) {
					comb -= Cnk[11 - i][r--];
					arr[i] = r << 1;
				} else {
					arr[i] = -1;
				}
			}
			edgeMove(arr, m);
			comb = 0, r = 4;
			var t = 0;
			var pm = [];
			for (var i = 0; i < 12; i++) {
				if (arr[i] >= 0) {
					comb += Cnk[11 - i][r--];
					pm[r] = arr[i] >> 1;
					t |= (arr[i] & 1) << (3 - r);
				}
			}
			return (comb * 24 + getNPerm(pm, 4)) << 4 | t;
		}
	}

	function xxinit() {
		xxinit = $.noop;
		xinit();
		var obj1 = 4;
		var obj2 = 5;
		createPrun(xxPrun01, obj1 * 3 * 24 + obj1 * 2 + 576 * (obj2 * 3 * 24 + obj2 * 2), 576 * 576, 7, function(q, m) {
			var ec1 = q % 576;
			var ec2 = ~~(q / 576);
			return c1mv[~~(ec1 / 24)][m] * 24 + e1mv[ec1 % 24][m] + 576 * (c1mv[~~(ec2 / 24)][m] * 24 + e1mv[ec2 % 24][m])
		});
		obj2 = 6;
		createPrun(xxPrun02, obj1 * 3 * 24 + obj1 * 2 + 576 * (obj2 * 3 * 24 + obj2 * 2), 576 * 576, 7, function(q, m) {
			var ec1 = q % 576;
			var ec2 = ~~(q / 576);
			return c1mv[~~(ec1 / 24)][m] * 24 + e1mv[ec1 % 24][m] + 576 * (c1mv[~~(ec2 / 24)][m] * 24 + e1mv[ec2 % 24][m])
		});
	}

	function xinit() {
		xinit = $.noop;
		init();
		for (var i = 0; i < 24; i++) {
			c1mv[i] = [];
			e1mv[i] = [];
			for (var m = 0; m < 6; m++) {
				c1mv[i][m] = cornMove(i, m);
				var edge = [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1];
				edge[i >> 1] = i & 1;
				edgeMove(edge, m);
				for (var e = 0; e < 12; e++) {
					if (edge[e] >= 0) {
						e1mv[i][m] = e << 1 | edge[e];
						break;
					}
				}
			}
		}
		ecPrun = [];
		for (var obj = 0; obj < 4; obj++) {
			var prun = [];
			createPrun(prun, (obj + 4) * 3 * 24 + (obj + 4) * 2, 576, 5, function(q, m) {
				return c1mv[~~(q / 24)][m] * 24 + e1mv[q % 24][m]
			});
			ecPrun[obj] = prun;
		}

		function cornMove(corn, m) {
			var idx = ~~(corn / 3);
			var twst = corn % 3;
			var idxt = [
				[3, 1, 2, 7, 0, 5, 6, 4],
				[0, 1, 6, 2, 4, 5, 7, 3],
				[1, 2, 3, 0, 4, 5, 6, 7],
				[0, 5, 1, 3, 4, 6, 2, 7],
				[4, 0, 2, 3, 5, 1, 6, 7],
				[0, 1, 2, 3, 7, 4, 5, 6]
			];
			var twstt = [
				[2, 0, 0, 1, 1, 0, 0, 2],
				[0, 0, 1, 2, 0, 0, 2, 1],
				[0, 0, 0, 0, 0, 0, 0, 0],
				[0, 1, 2, 0, 0, 2, 1, 0],
				[1, 2, 0, 0, 2, 1, 0, 0],
				[0, 0, 0, 0, 0, 0, 0, 0]
			];
			twst = (twst + twstt[m][idx]) % 3;
			return idxt[m][idx] * 3 + twst;
		}
	}

	var solvCross = new mathlib.Searcher(
		(idx) => idx[0] + idx[1] == 0,
		(idx) => Math.max(getPruning(permPrun, idx[0]), getPruning(flipPrun, idx[1])),
		(idx, move) => [pmv(idx[0], move), fmv(idx[1], move)],
		6, 3, [1, 2, 4, 9, 18, 36]
	);

	var solvXCross = new mathlib.Searcher(
		(idx) => idx[0] + idx[1] == 0 && idx[2] == (idx[4] + 4) * 2 && idx[3] == (idx[4] + 4) * 3,
		(idx) => Math.max(
			getPruning(permPrun, idx[0]),
			getPruning(flipPrun, idx[1]),
			getPruning(ecPrun[idx[4]], idx[3] * 24 + idx[2])
		),
		(idx, move) => [pmv(idx[0], move), fmv(idx[1], move), e1mv[idx[2]][move], c1mv[idx[3]][move], idx[4]],
		6, 3, [1, 2, 4, 9, 18, 36]
	);

	var solvXXCross = new mathlib.Searcher(
		(idx) => idx[0] + idx[1] == 0 && idx[2] == 4 * 2 && idx[3] == 4 * 3
				&& (idx[4] == 5 * 2 && idx[5] == 5 * 3 || idx[4] == 6 * 2 && idx[5] == 6 * 3),
		(idx) => Math.max(
			getPruning(permPrun, idx[0]),
			getPruning(flipPrun, idx[1]),
			getPruning(idx[6], idx[3] * 24 + idx[2] + 576 * (idx[5] * 24 + idx[4]))
		),
		(idx, move) => [
			pmv(idx[0], move), fmv(idx[1], move),
			e1mv[idx[2]][move], c1mv[idx[3]][move],
			e1mv[idx[4]][move], c1mv[idx[5]][move],
			idx[6]
		],
		6, 3, [1, 2, 4, 9, 18, 36]
	);

	var solvXXXCross = new mathlib.Searcher(
		(idx) => idx[0] + idx[1] == 0 && idx[2] == 4 * 2 && idx[3] == 4 * 3
				&& idx[4] == 5 * 2 && idx[5] == 5 * 3 && idx[6] == 6 * 2 && idx[7] == 6 * 3,
		(idx) => Math.max(
			getPruning(permPrun, idx[0]),
			getPruning(flipPrun, idx[1]),
			getPruning(xxPrun01, idx[3] * 24 + idx[2] + 576 * (idx[5] * 24 + idx[4])),
			getPruning(xxPrun02, idx[3] * 24 + idx[2] + 576 * (idx[7] * 24 + idx[6]))
		),
		(idx, move) => [
			pmv(idx[0], move), fmv(idx[1], move),
			e1mv[idx[2]][move], c1mv[idx[3]][move],
			e1mv[idx[4]][move], c1mv[idx[5]][move],
			e1mv[idx[6]][move], c1mv[idx[7]][move]
		],
		6, 3, [1, 2, 4, 9, 18, 36]
	);

	var faceStr = ["D", "U", "L", "R", "F", "B"];
	var moveIdx = ["FRUBLD", "FLDBRU", "FDRBUL", "FULBDR", "URBDLF", "DRFULB"]
	var rotIdx = ["&nbsp;&nbsp;", "z2", "z'", "z&nbsp;", "x'", "x&nbsp;"];
	var yrotIdx = ["FRUBLD", "RBULFD", "BLUFRD", "LFURBD"];

	var curScramble;

	function solve_cross(moves) {
		init();
		var ret = [];
		for (var face = 0; face < 6; face++) {
			var flip = 0;
			var perm = 0;
			for (var i = 0; i < moves.length; i++) {
				var m = moveIdx[face].indexOf("FRUBLD".charAt(moves[i][0]));
				var p = moves[i][2];
				for (var j = 0; j < p; j++) {
					flip = fmv(flip, m);
					perm = pmv(perm, m);
				}
			}
			var sol = solvCross.solve([perm, flip], 0, 50);
			for (var i = 0; i < sol.length; i++) {
				sol[i] = "FRUBLD".charAt(sol[i][0]) + " 2'".charAt(sol[i][1])
			}
			ret.push(sol);
		}
		return ret;
	}

	function solve_xcross(moves, face) {
		xinit();
		var flip = 0;
		var perm = 0;
		var e1 = [8, 10, 12, 14];
		var c1 = [12, 15, 18, 21];
		for (var i = 0; i < moves.length; i++) {
			var m = moveIdx[face].indexOf("FRUBLD".charAt(moves[i][0]));
			var p = moves[i][2];
			for (var j = 0; j < p; j++) {
				flip = fmv(flip, m);
				perm = pmv(perm, m);
				for (var obj = 0; obj < 4; obj++) {
					e1[obj] = e1mv[e1[obj]][m];
					c1[obj] = c1mv[c1[obj]][m];
				}
			}
		}
		var idxs = [];
		for (var i = 0; i < 4; i++) {
			idxs.push([perm, flip, e1[i], c1[i], i]);
		}
		var sol = solvXCross.solveMulti(idxs, 0, 20)[0];
		for (var i = 0; i < sol.length; i++) {
			sol[i] = "FRUBLD".charAt(sol[i][0]) + " 2'".charAt(sol[i][1])
		}
		return sol;
	}

	function solve_xxcross(moves, face, is3x) {
		xxinit();
		var idxs = [];
		var id3s = [];
		var yrot = 0;
		for (yrot = 0; yrot < 4; yrot++) {
			var flip = 0;
			var perm = 0;
			var e1 = [8, 10, 12];
			var c1 = [12, 15, 18];
			for (var i = 0; i < moves.length; i++) {
				var m = yrotIdx[yrot].indexOf("FRUBLD".charAt(moveIdx[face].indexOf("FRUBLD".charAt(moves[i][0]))));
				var p = moves[i][2];
				for (var j = 0; j < p; j++) {
					flip = fmv(flip, m);
					perm = pmv(perm, m);
					e1 = [e1mv[e1[0]][m], e1mv[e1[1]][m], e1mv[e1[2]][m]];
					c1 = [c1mv[c1[0]][m], c1mv[c1[1]][m], c1mv[c1[2]][m]];
				}
			}
			idxs.push([perm, flip, e1[0], c1[0], e1[1], c1[1], xxPrun01]);
			idxs.push([perm, flip, e1[0], c1[0], e1[2], c1[2], xxPrun02]);
			id3s.push([perm, flip, e1[0], c1[0], e1[1], c1[1], e1[2], c1[2]]);
		}
		var tt = +new Date;
		var sol = is3x ? solvXXXCross.solveMulti(id3s, 0, 20) : solvXXCross.solveMulti(idxs, 0, 20);
		var yrot = is3x ? sol[1] : (sol[1] >> 1);
		return sol[0].map((move) => yrotIdx[yrot][move[0]] + " 2'"[move[1]]);
	}

	function fullInit() {
		fullInit = $.noop;
		init();
		fullPrun = [];
		createPrun(fullPrun, 0, 190080, 7, fullmv, 6, 3, 6);
	}

	function mapCross(idx) {
		var comb = ~~(idx / 384);
		var perm = (idx >> 4) % 24;
		var flip = idx & 15;

		var arrp = [];
		var arrf = [];
		var pm = [];
		var fl = [];
		i2f(flip, fl);
		setNPerm(pm, perm, 4);
		var r = 4;
		var map = [7, 6, 5, 4, 10, 9, 8, 11, 3, 2, 1, 0];
		for (var i = 0; i < 12; i++) {
			if (comb >= Cnk[11 - i][r]) {
				comb -= Cnk[11 - i][r--];
				arrp[map[i]] = pm[r];
				arrf[map[i]] = fl[r];
			} else {
				arrp[map[i]] = arrf[map[i]] = -1;
			}
		}
		return [arrp, arrf];
	}

	function getEasyCross(length) {
		fullInit();
		var lenA = Math.min(length % 10, 8);
		var lenB = Math.min(~~(length / 10), 8);
		var minLen = Math.min(lenA, lenB);
		var maxLen = Math.max(lenA, lenB);
		var ncase = [0, 1, 16, 174, 1568, 11377, 57758, 155012, 189978, 190080];
		var cases = mathlib.rn(ncase[maxLen + 1] - ncase[minLen]) + 1;
		var i;
		for (i = 0; i < 190080; i++) {
			var prun = getPruning(fullPrun, i);
			if (prun <= maxLen && prun >= minLen && --cases == 0) {
				break;
			}
		}
		return mapCross(i);
	}

	function getEasyXCross(length) {
		fullInit();
		xinit();
		var lenA = length % 10;
		var lenB = ~~(length / 10);
		var minLen = Math.min(lenA, lenB, 8);
		var maxLen = Math.max(lenA, lenB);
		var ncase = [1, 16, 174, 1568, 11377, 57758, 155012, 189978, 190080];
		length = Math.max(0, Math.min(maxLen, 8)); // cross length
		var remain = ncase[length];
		var isFound = false;
		var testCnt = 0;
		while (!isFound) {
			var rndIdx = [];
			var sample = 500;
			for (var i = 0; i < sample; i++) {
				rndIdx.push(mathlib.rn(remain));
			}
			rndIdx.sort(function(a, b) { return b - a; });
			var rndCases = [];
			var cnt = 0;
			for (var i = 0; i < 190080; i++) {
				var prun = getPruning(fullPrun, i);
				if (prun > length) {
					continue;
				}
				while (rndIdx.at(-1) == cnt) {
					rndCases.push(i);
					rndIdx.pop();
				}
				if (rndIdx.length == 0) {
					break;
				}
				cnt++;
			}
			rndIdx = mathlib.rndPerm(sample);
			for (var i = 0; i < sample; i++) {
				var caze = rndCases[rndIdx[i]];
				var comb = ~~(caze / 384);
				var perm = comb * 24 + (caze >> 4) % 24;
				var flip = comb << 4 | caze & 15;
				var sol = [];
				var corns = mathlib.rndPerm(8).slice(4);
				var edges = mathlib.rndPerm(8);

				var arr = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
				var r = 4;
				for (var j = 0; j < 12; j++) {
					if (comb >= Cnk[11 - j][r]) {
						comb -= Cnk[11 - j][r--];
						arr[j] = -1;
					} else {
						arr[j] = edges.pop();
					}
				}
				for (var j = 0; j < 4; j++) {
					corns[j] = corns[j] * 3 + mathlib.rn(3);
					edges[j] = arr.indexOf(j) * 2 + mathlib.rn(2);
					var sol = solvXCross.solve([perm, flip, edges[j], corns[j], j], 0, isFound ? minLen - 1 : maxLen);
					if (sol == null) {
						continue;
					} else if (sol.length < minLen) {
						isFound = false; // too short
						break;
					} else if (sol.length <= maxLen) {
						isFound = true;
					}
				}
				if (!isFound) {
					continue;
				}
				var crossArr = mapCross(caze);
				crossArr[2] = mathlib.valuedArray(8, -1);
				crossArr[3] = mathlib.valuedArray(8, -1);
				var map = [7, 6, 5, 4, 10, 9, 8, 11, 3, 2, 1, 0];
				var map2 = [6, 5, 4, 7, 2, 1, 0, 3];
				for (var i = 0; i < 4; i++) {
					crossArr[0][map[edges[i] >> 1]] = map[i + 4];
					crossArr[1][map[edges[i] >> 1]] = edges[i] % 2;
					crossArr[2][map2[~~(corns[i] / 3)]] = map2[i + 4];
					crossArr[3][map2[~~(corns[i] / 3)]] = (30 - corns[i]) % 3;
				}
				return crossArr;
			}
		}
	}


	ISCSTIMER && execMain(function() {

		function execCross(scramble, fdiv) {
			fdiv.empty();
			curScramble = cubeutil.parseScramble(scramble, "FRUBLD");
			var solutions = solve_cross(curScramble);
			for (var face = 0; face < 6; face++) {
				var span = $('<span class="sol"/>');
				var clk = $('<span />').html('ec').addClass('click').click(ecClick);
				span.append(faceStr[face] + "(", clk, "): " + rotIdx[face], tools.getSolutionSpan(solutions[face]), '<br>');
				fdiv.append(span);
			}
		}

		function ecClick() {
			var span = $(this).parent();
			var face = "DULRFB".indexOf(span.html()[0]);
			var sol = solve_xcross(curScramble, face);
			var clktxt = $(this).html();
			if (clktxt == 'ec') {
				sol = solve_xcross(curScramble, face);
				clktxt = 'xx';
			} else if (clktxt == 'xx') {
				sol = solve_xxcross(curScramble, face);
				clktxt = '3x';
			} else if (clktxt == '3x') {
				sol = solve_xxcross(curScramble, face, true);
				clktxt = '<<';
			} else {
				sol = solve_cross(curScramble)[face];
				clktxt = 'ec';
			}
			var clk = $('<span />').html(clktxt).addClass('click').click(ecClick);
			span.empty().append(faceStr[face] + "(", clk, "): " + rotIdx[face], tools.getSolutionSpan(sol), '<br>');
		}

		function execFunc(fdiv) {
			if (!fdiv) {
				return;
			}
			if (tools.isPuzzle('333')) {
				var scramble = tools.getCurScramble();
				execCross(scramble[1], fdiv);
			} else {
				fdiv.html(IMAGE_UNAVAILABLE);
			}
		}

		$(function() {
			tools.regTool('cross', TOOLS_SOLVERS + '>' + TOOLS_CROSS, execFunc);
		});
	});

	return {
		solve: solve_cross,
		solveXCross: solve_xcross,
		getEasyCross: getEasyCross,
		getEasyXCross: getEasyXCross
	}
})(mathlib.createMove, mathlib.edgeMove, mathlib.createPrun, mathlib.setNPerm, mathlib.getNPerm, mathlib.Cnk, mathlib.getPruning);

	function MoveCounterHTM() {
		this.clear();
	}

	MoveCounterHTM.prototype.push = function(move) {
		var axis = ~~(move / 3);
		var amask = 1 << axis;
		if (axis % 3 != this.lastMove % 3) {
			this.lastMove = axis;
			this.lastPow = 0;
		}
		this.moveCnt += (this.lastPow & amask) == amask ? 0 : 1;
		this.lastPow |= amask;
		this.moves.push(move);
	}

	MoveCounterHTM.prototype.clear = function() {
		this.lastPow = 0;
		this.lastMove = -3;
		this.moveCnt = 0;
		this.moves = [];
	}

	function getMoveCnt(sol, metric) {
		sol = sol.split(/ +/);
		var c = new mathlib.CubieCube();
		c.ori = 0;
		var cnter = metric == 'lfmc' ? new MoveCounterLFMC() : new MoveCounterHTM();
		for (var i = 0; i < sol.length; i++) {
			var effMove = c.selfMoveStr(sol[i], false);
			if (effMove != undefined) {
				cnter.push(effMove);
			}
		}
		return cnter.moveCnt;
	}


var recons={getMoveCnt:getMoveCnt};
 function tokens(algorithm) {
  return String(algorithm || '').trim().split(/\s+/).filter(Boolean).map(function(m) {
   return m.replace(/^([urfdlb])/, function(_, f) { return f.toUpperCase() + 'w'; }).replace(/2'$/, '2');
  });
 }
 function technique(record) {
  var sequence = record && record[4] && record[4][0];
  if (typeof sequence !== 'string') return null;
  var moves = sequence.split(/\s+/).map(function(m) { var match = /^(.*?)@(\d+)$/.exec(m); return match ? {move:match[1],time:Number(match[2])} : null; }).filter(Boolean);
  var gap = null;
  for (var i = 1; i < moves.length; i++) {
   var duration = moves[i].time - moves[i - 1].time;
   if (duration >= 0 && (!gap || duration > gap.time)) gap = {time:duration, before:moves[i - 1].move, after:moves[i].move};
  }
  return {gap:gap, u2:moves.filter(function(m) { return /^U2'?$/i.test(m.move); }).length,
   rotations:moves.filter(function(m) { return /^[xyz]/.test(m.move); }).length,
   trigger:moves.map(function(m) { return m.move; }).join(' ').indexOf("R U R' U'") >= 0};
 }
  function goal(cube, method){return method==='PLL'?cube.ca.every(function(v,i){return v===i;})&&cube.ea.every(function(v,i){return v===i*2;}):cubeutil.getProgress(cube,'cfop')<=1;}
  // Verify each proposed variant on the reconstructed start, including the
  // required orientation/AUF. No prediction of personal speed is made.
  function align(algorithm, start, method) {
   var best=null, alg=tokens(algorithm), tilts=['','x','x2',"x'",'z',"z'"];
   tilts.forEach(function(tilt){for(var y=0;y<4;y++)for(var pre=0;pre<4;pre++){
    var prefix=[tilt,['','y','y2',"y'"][y],['','U','U2',"U'"][pre]].filter(Boolean), cube=new mathlib.CubieCube().init(start.ca,start.ea);
    prefix.concat(alg).forEach(function(m){cube.selfMoveStr(m);});
    for(var post=0;post<(method==='PLL'?4:1);post++){
     var test=new mathlib.CubieCube().init(cube.ca,cube.ea);test.ori=cube.ori;
     var tail=['','U','U2',"U'"][post];if(tail)test.selfMoveStr(tail);
     if(!goal(test,method))continue;
     var moves=prefix.concat(alg,tail?[tail]:[]), rotations=moves.filter(function(m){return /^[xyz]/.test(m);}).length;
     var notation=moves.join(' '), count=recons.getMoveCnt(notation);
     if(!best||count<best.moves||count===best.moves&&rotations<best.rotations)best={algorithm:notation,moves:count,rotations:rotations};
    }
   }});
   return best;
  }

var nativeCatalog={"source":{"name":"Tao Yu \u00b7 Alg-Trainer","url":"https://github.com/tao-yu/Alg-Trainer","revision":"e94551529fc0db8dccb11a5257c0e39dbaed2726","license":"MIT"},"cases":[{"id":"oll-1","stage":"OLL","name":"OLL 1","number":1,"variants":["R U2 R2' F R F' U2 R' F R F'","R U B' R B R2 U' R' F R F'","U R U' R2 D' r U' r' D R2 U R'","r U R' U R' r2 U' R' U R' r2 U2 r'"]},{"id":"oll-2","stage":"OLL","name":"OLL 2","number":2,"variants":["F R U R' U' F' f R U R' U' f'","F R U R' U' S R U R' U' f'","U r U r' U2 R U2 R' U2 r U' r'","R' U2 r U' r' U2' r U r' U2 R"]},{"id":"oll-3","stage":"OLL","name":"OLL 3","number":3,"variants":["U' f R U R' U' f' U' F R U R' U' F'","r' R2 U R' U r U2 r' U M'","r' R U R' F2 R U L' U L M'","U F U R U' R' F' U F R U R' U' F'"]},{"id":"oll-4","stage":"OLL","name":"OLL 4","number":4,"variants":["U' f R U R' U' f' U F R U R' U' F'","M U' r U2 r' U' R U' R2 r","U F U R U' R' F' U' F R U R' U' F'","U' f R U R' d' l' F R U R' U' F'"]},{"id":"oll-5","stage":"OLL","name":"OLL 5","number":5,"variants":["r' U2 R U R' U r","U2 l' U2 L U L' U l","U2 R' F2 r U r' F R","F R U R' U' F' U' F R U R' U' F'"]},{"id":"oll-6","stage":"OLL","name":"OLL 6","number":6,"variants":["r U2 R' U' R U' r'","U2 l U2 L' U' L U' l'","U2 R U R2 F R F2 U F","U' x' D R2 U' R' U R' D' x"]},{"id":"oll-7","stage":"OLL","name":"OLL 7","number":7,"variants":["r U R' U R U2 r'","F R' F' R U2 R U2 R'","L' U2 L U2 L F' L' F","U2 l U L' U L U2 l'"]},{"id":"oll-8","stage":"OLL","name":"OLL 8","number":8,"variants":["U2 r' U' R U' R' U2 r","l' U' L U' L' U2 l","R U2 R' U2 R' F R F'","F' L F L' U2 L' U2 L"]},{"id":"oll-9","stage":"OLL","name":"OLL 9","number":9,"variants":["U R U R' U' R' F R2 U R' U' F'","U' L' U' L U' L F' L' F L' U2 L","U2 R' U' R U' R' U R' F R F' U R","r' R2 U2 R' U' R U' R' U' M'"]},{"id":"oll-10","stage":"OLL","name":"OLL 10","number":10,"variants":["R U R' U R' F R F' R U2 R'","U2 L' U' L U L F' L2 U' L U F","R U R' y R' F R U' R' F' R","R U R' y' r' U r U' r' U' r"]},{"id":"oll-11","stage":"OLL","name":"OLL 11","number":11,"variants":["r' R2 U R' U R U2 R' U M'","M R U R' U R U2 R' U M'","U F' L' U' L U F U F R U R' U' F'","U2 r U R' U R' F R F' R U2 r'"]},{"id":"oll-12","stage":"OLL","name":"OLL 12","number":12,"variants":["F R U R' U' F' U F R U R' U' F'","U' M' R' U' R U' R' U2 R U' M","U' r R2' U' R U' R' U2 R U' R r'","U M U2 R' U' R U' R' U2 R U M'"]},{"id":"oll-13","stage":"OLL","name":"OLL 13","number":13,"variants":["r U' r' U' r U r' F' U F","F U R U2 R' U' R U R' F'","F U R U' R2 F' R U R U' R'","r U' r' U' r U r' y' R' U R"]},{"id":"oll-14","stage":"OLL","name":"OLL 14","number":14,"variants":["R' F R U R' F' R F U' F'","R' F R U R' F' R y' R U' R'","r' U r U r' U' r y R U' R'","l' U l U l' U' l y' R U' R'"]},{"id":"oll-15","stage":"OLL","name":"OLL 15","number":15,"variants":["U2 l' U' l L' U' L U l' U l","r' U' r R' U' R U r' U r","r' U' M' U' R U r' U r","U2 R' F' R L' U' L U R' F R"]},{"id":"oll-16","stage":"OLL","name":"OLL 16","number":16,"variants":["r U r' R U R' U' r U' r'","r U M U R' U' r U' r'","U2 R' F R U R' U' F' R U' R' U2 R","U2 l U l' L U L' U' l U' l'"]},{"id":"oll-17","stage":"OLL","name":"OLL 17","number":17,"variants":["R U R' U R' F R F' U2 R' F R F'","U2 F R' F' R2 r' U R U' R' U' M'","f R U R' U' f' U' R U R' U' R' F R F'","R' F R U' M' U2 r' U' F' U R"]},{"id":"oll-18","stage":"OLL","name":"OLL 18","number":18,"variants":["r U R' U R U2 r2 U' R U' R' U2 r","U R U2 R2 F R F' U2 M' U R U' r'","U2 F R U R' d R' U2 R' F R F'","U2 F R U R' U y' R' U2 R' F R F'"]},{"id":"oll-19","stage":"OLL","name":"OLL 19","number":19,"variants":["M U R U R' U' M' R' F R F'","r' R U R U R' U' r R2' F R F'","r' U2 R U R' U r2 U2 R' U' R U' r'","R' U2 F R U R' U' F2 U2 F R"]},{"id":"oll-20","stage":"OLL","name":"OLL 20","number":20,"variants":["M U R U R' U' M2 U R U' r'","r U R' U' M2 U R U' R' U' M'","M' U M' U M' U M' U' M' U M' U M' U M'","M' U2 M U2 M' U M U2 M' U2 M"]},{"id":"oll-21","stage":"OLL","name":"OLL 21","number":21,"variants":["U R U2 R' U' R U R' U' R U' R'","U F R U R' U' R U R' U' R U R' U' F'","R U R' U R U' R' U R U2 R'","R' U' R U' R' U R U' R' U2 R"]},{"id":"oll-22","stage":"OLL","name":"OLL 22","number":22,"variants":["R U2 R2 U' R2 U' R2 U2 R","f R U R' U' f' F R U R' U' F'","R U2' R2' U' R2 U' R2' U2 R","R' U2 R2 U R2 U R2 U2 R'"]},{"id":"oll-23","stage":"OLL","name":"OLL 23","number":23,"variants":["R2 D R' U2 R D' R' U2 R'","U2 R2 D' R U2 R' D R U2 R","U R U R' U' R U' R' U2 R U' R' U2 R U R'","R U R' U R U2 R2 U' R U' R' U2 R"]},{"id":"oll-24","stage":"OLL","name":"OLL 24","number":24,"variants":["r U R' U' r' F R F'","U2 l' U' L U R U' r' F","U' x' R U R' D R U' R' D' x","L F R' F' L' F R F'"]},{"id":"oll-25","stage":"OLL","name":"OLL 25","number":25,"variants":["U F' r U R' U' r' F R","R' F R B' R' F' R B","F R' F' r U R U' r'","U' x' R U' R' D R U R' D' x"]},{"id":"oll-26","stage":"OLL","name":"OLL 26","number":26,"variants":["U R U2 R' U' R U' R'","R' U' R U' R' U2 R","U2 L' U' L U' L' U2 L","U2 L' U R U' L U R'"]},{"id":"oll-27","stage":"OLL","name":"OLL 27","number":27,"variants":["R U R' U R U2 R'","U' R' U2 R U R' U R","R U' L' U R' U' L","U2 L U L' U L U2 L'"]},{"id":"oll-28","stage":"OLL","name":"OLL 28","number":28,"variants":["r U R' U' M U R U' R'","U2 M' U M U2 M' U M","M U M' U2 M U M'","U' M' U' M U2 M' U' M"]},{"id":"oll-29","stage":"OLL","name":"OLL 29","number":29,"variants":["M U R U R' U' R' F R F' M'","r2 D' r U r' D r2 U' r' U' r","U R U R' U' R U' R' F' U' F R U R'","U2 R' F R F' R U2 R' U' F' U' F"]},{"id":"oll-30","stage":"OLL","name":"OLL 30","number":30,"variants":["U' r' D' r U' r' D r2 U' r' U r U r'","M U' L' U' L U L F' L' F M'","U2 F R' F R2 U' R' U' R U R' F2","R2 U R' B' R U' R2 U R B R'"]},{"id":"oll-31","stage":"OLL","name":"OLL 31","number":31,"variants":["R' U' F U R U' R' F' R","U2 S' L' U' L U L F' L' f","U' F R' F' R U R U R' U' R U' R'","U S R U R' U' f' U' F"]},{"id":"oll-32","stage":"OLL","name":"OLL 32","number":32,"variants":["S R U R' U' R' F R f'","R U B' U' R' U R B R'","U2 L U F' U' L' U L F L'","R d L' d' R' U l U l'"]},{"id":"oll-33","stage":"OLL","name":"OLL 33","number":33,"variants":["R U R' U' R' F R F'","F R U' R' U R U R' F'","U2 L' U' L U L F' L' F","U' r' U' r' D' r U r' D r2"]},{"id":"oll-34","stage":"OLL","name":"OLL 34","number":34,"variants":["U2 R U R' U' B' R' F R F' B","U2 R U R2 U' R' F R U R U' F'","F R U R' U' R' F' r U R U' r'","U2 R U R' U' x D' R' U R U' D x'"]},{"id":"oll-35","stage":"OLL","name":"OLL 35","number":35,"variants":["R U2 R2' F R F' R U2 R'","f R U R' U' f' R U R' U R U2 R'","U' R' U2 R l U' R' U l' U2 R","U' R U2 R' U' R U' R' U2 F R U R' U' F'"]},{"id":"oll-36","stage":"OLL","name":"OLL 36","number":36,"variants":["U2 L' U' L U' L' U L U L F' L' F","R' U' R U' R' U R U l U' R' U x","U2 R U R' F' R U R' U' R' F R U' R' F R F'","R' U' R U' R' U R U R y R' F' R"]},{"id":"oll-37","stage":"OLL","name":"OLL 37","number":37,"variants":["F R U' R' U' R U R' F'","F R' F' R U R U' R'","R' F R F' U' F' U F","U' R U2 R' F R' F' R2 U2 R'"]},{"id":"oll-38","stage":"OLL","name":"OLL 38","number":38,"variants":["R U R' U R U' R' U' R' F R F'","L' U' L F L' U' L U L F' L' U L F' L' F","R' U2 r' D' r U2 r' D R r","U2 R U R' U R U2 R' U F R U R' U' F'"]},{"id":"oll-39","stage":"OLL","name":"OLL 39","number":39,"variants":["U L F' L' U' L U F U' L'","U' R U R' F' U' F U R U2 R'","U' R B' R' U' R U B U' R'","F R U R' U' R U R' U' R U' R' U' R U R' F'"]},{"id":"oll-40","stage":"OLL","name":"OLL 40","number":40,"variants":["U R' F R U R' U' F' U R","U' f R' F' R U R U' R' S'","U' F R U R' U' F' R U R' U R U2 R'","R r D r' U r D' r' U' R'"]},{"id":"oll-41","stage":"OLL","name":"OLL 41","number":41,"variants":["U2 R U R' U R U2' R' F R U R' U' F'","U' L F' L' F L F' L' F L' U' L U L' U' L","R U' R' U2 R U y R U' R' U' F'","f R U R' U' f' U' R U R' U R U2 R'"]},{"id":"oll-42","stage":"OLL","name":"OLL 42","number":42,"variants":["R' U' R U' R' U2 R F R U R' U' F'","U R' F R F' R' F R F' R U R' U' R U R'","M U F R U R' U' F' M'","L' U L U2 L' U' y' L' U L U F"]},{"id":"oll-43","stage":"OLL","name":"OLL 43","number":43,"variants":["f' L' U' L U f","U2 F' U' L' U L F","U R' U' F' U F R","B' U' R' U R B"]},{"id":"oll-44","stage":"OLL","name":"OLL 44","number":44,"variants":["f R U R' U' f'","U2 F U R U' R' F'","U2 r U x' R U' R' U x U' r'","U' L d R U' R' F'"]},{"id":"oll-45","stage":"OLL","name":"OLL 45","number":45,"variants":["F R U R' U' F'","U2 f U R U' R' f'","U2 F' L' U' L U F","F R2 D R' U R D' R2 U' F'"]},{"id":"oll-46","stage":"OLL","name":"OLL 46","number":46,"variants":["R' U' R' F R F' U R","U F R U R' y' R' U R U2 R'","U2 r' F' L' U L U' F r","U' F R U R2 U' R F' U' R' U2 R"]},{"id":"oll-47","stage":"OLL","name":"OLL 47","number":47,"variants":["F' L' U' L U L' U' L U F","R' U' R' F R F' R' F R F' U R","R' U' l' U R U' R' U R U' x' U R","U2 B' R' U' R U R' U' R U B"]},{"id":"oll-48","stage":"OLL","name":"OLL 48","number":48,"variants":["F R U R' U' R U R' U' F'","R U2 R' U' R U R' U2 R' F R F'","U2 f U R U' R' U R U' R' f'"]},{"id":"oll-49","stage":"OLL","name":"OLL 49","number":49,"variants":["U2 r U' r2 U r2 U r2 U' r","l U' l2 U l2 U l2 U' l","R B' R2 F R2 B R2 F' R","U2 R' F R' F' R2 U2 B' R B R'"]},{"id":"oll-50","stage":"OLL","name":"OLL 50","number":50,"variants":["r' U r2 U' r2' U' r2 U r'","U2 R' F R2 B' R2 F' R2 B R'","U' R U2 R' U' R U' R' F R U R' U' F'","U2 l' U l2 U' l2 U' l2 U l'"]},{"id":"oll-51","stage":"OLL","name":"OLL 51","number":51,"variants":["f R U R' U' R U R' U' f'","U2 F U R U' R' U R U' R' F'","U' R' U' R' F R F' R U' R' U2 R","U2 f' L' U' L U L' U' L U f"]},{"id":"oll-52","stage":"OLL","name":"OLL 52","number":52,"variants":["R U R' U R d' R U' R' F'","R' U' R U' R' U F' U F R","R' U' R U' R' d R' U R B","R U R' U R U' y R U' R' F'"]},{"id":"oll-53","stage":"OLL","name":"OLL 53","number":53,"variants":["r' U' R U' R' U R U' R' U2 r","U2 l' U' L U' L' U L U' L' U2 l","U r' U2 R U R' U' R U R' U r","U' l' U2 L U L' U' L U L' U l"]},{"id":"oll-54","stage":"OLL","name":"OLL 54","number":54,"variants":["r U R' U R U' R' U R U2 r'","U' r U2 R' U' R U R' U' R U' r'","U' r U r' R U R' U' R U R' U' r U' r'","F' L' U' L U L' U L U' L' U' L F"]},{"id":"oll-55","stage":"OLL","name":"OLL 55","number":55,"variants":["R U2 R2 U' R U' R' U2 F R F'","U R' F R U R U' R2 F' R2 U' R' U R U R'","r U2 R2 F R F' U2 r' F R F'","U r U2 R' U' R2 r' U R' U' r U' r'"]},{"id":"oll-56","stage":"OLL","name":"OLL 56","number":56,"variants":["r U r' U R U' R' U R U' R' r U' r'","F R U R' U' R F' r U R' U' r'","r' U' r U' R' U R U' R' U R r' U r","U f R U R' U' f' F R U R' U' R U R' U' F'"]},{"id":"oll-57","stage":"OLL","name":"OLL 57","number":57,"variants":["R U R' U' M' U R U' r'","M' U M' U M' U2 M U M U M","R U R' U' r R' U R U' r'","M' U M' U M' U M' U2 M' U M' U M' U M'"]},{"id":"pll-ua","stage":"PLL","name":"Ua perm","variants":["U2 R2 U' R' U' R U R U R U' R","R U' R U R U R U' R' U' R2","M2 U M U2 M' U M2","U2 M2' U M' U2' M U M2'","U2 R U R' U R' U' R2 U' R' U R' U R","U' R2 U' S' U2 S U' R2"]},{"id":"pll-ub","stage":"PLL","name":"Ub perm","variants":["R' U R' U' R' U' R' U R U R2","U2 M2 U' M U2 M' U' M2","M2' U' M' U2' M U' M2'"]},{"id":"pll-h","stage":"PLL","name":"H perm","variants":["M2 U' M2 U2 M2 U' M2","M2 U M2 U2 M2 U M2"]},{"id":"pll-z","stage":"PLL","name":"Z perm","variants":["M' U' M2 U' M2 U' M' U2 M2","U M' U M2 U M2 U M' U2 M2","U M2' U' M2' U' M' U2' M2' U2' M'","M2' U M2' U M' U2' M2' U2' M'"]},{"id":"pll-aa","stage":"PLL","name":"Aa perm","variants":["l' U R' D2 R U' R' D2 R2 x'","U x' R2 D2 R' U' R D2 R' U R' x"]},{"id":"pll-ab","stage":"PLL","name":"Ab perm","variants":["x R2 D2 R U R' D2 R U' R x'","U x' R U' R D2 R' U R D2 R2 x"]},{"id":"pll-e","stage":"PLL","name":"E perm","variants":["x' R U' R' D R U R' D' R U R' D R U' R' D' x"]},{"id":"pll-ga","stage":"PLL","name":"Ga perm","variants":["R2 u R' U R' U' R u' R2 y' R' U R","R2 U R' U R' U' R U' R2 D U' R' U R D'"]},{"id":"pll-gb","stage":"PLL","name":"Gb perm","variants":["U F' U' F R2 u R' U R U' R u' R2","R' U' R y R2 u R' U R U' R u' R2","R' U' R U D' R2 U R' U R U' R U' R2 D"]},{"id":"pll-gc","stage":"PLL","name":"Gc perm","variants":["R2 u' R U' R U R' u R2 y R U' R'","R2 u' R U' R U R' D x' U2 r U' r'","U2 R2' F2 R U2 R U2' R' F R U R' U' R' F R2","R2' U' R U' R U R' U R2 D' U R U' R' D"]},{"id":"pll-gd","stage":"PLL","name":"Gd perm","variants":["R U R' y' R2 u' R U' R' U R' u R2","R U R' U' D R2 U' R U' R' U R' U R2 D'"]},{"id":"pll-ra","stage":"PLL","name":"Ra perm","variants":["R U R' F' R U2 R' U2 R' F R U R U2 R'","R U' R' U' R U R D R' U' R D' R' U2 R'","U' L U2 L' U2 L F' L' U' L U L F L2"]},{"id":"pll-rb","stage":"PLL","name":"Rb perm","variants":["R' U2 R U2 R' F R U R' U' R' F' R2","R' U2 R' D' R U' R' D R U R U' R' U' R"]},{"id":"pll-ja","stage":"PLL","name":"Ja perm","variants":["R' U L' U2 R U' R' U2 R L","U2 L' U R' z R2 U R' U' R2 U D","U x R2 F R F' R U2 r' U r U2 x'","U' L' U' L F L' U' L U L F' L2 U L"]},{"id":"pll-jb","stage":"PLL","name":"Jb perm","variants":["R U R' F' R U R' U' R' F R2 U' R' U'"]},{"id":"pll-na","stage":"PLL","name":"Na perm","variants":["R U R' U R U R' F' R U R' U' R' F R2 U' R' U2 R U' R'","z D R' U R2 D' R U' D R' U R2 D' R U' z'","z U R' D R2' U' R D' U R' D R2 U' R D' z'"]},{"id":"pll-nb","stage":"PLL","name":"Nb perm","variants":["z D' R U' R2 D R' U D' R U' R2 D R' U z'","R' U R U' R' F' U' F R U R' F R' F' R U' R","R' U R U' R' F' U' F R U R' U' R U' f R f'","z U' R D' R2' U R' D U' R D' R2' U R' D z'"]},{"id":"pll-y","stage":"PLL","name":"Y perm","variants":["F R U' R' U' R U R' F' R U R' U' R' F R F'","F R' F R2 U' R' U' R U R' F' R U R' U' F'"]},{"id":"pll-f","stage":"PLL","name":"F perm","variants":["R' U' F' R U R' U' R' F R2 U' R' U' R U R' U R","U R' U2 R' d' R' F' R2 U' R' U R' F R U' F"]},{"id":"pll-t","stage":"PLL","name":"T perm","variants":["R U R' U' R' F R2 U' R' U' R U R' F'"]},{"id":"pll-v","stage":"PLL","name":"V perm","variants":["R' U R' d' R' F' R2 U' R' U R' F R F","z D' R2 D R2' U R' D' R U' R U R' D R U' z'","U2 L' U L' U' y' R' F' R2 U' R' U R' F R F","R' U R U' x' U R U2' R' U' R U' R' U2' R U R' U' x","R' U R' U' R D' R' D R' U D' R2 U' R2' D R2"]}]}
;
var nativeF2LCatalog=[{"id":"f2l-1","name":"F2L 1","variants":["U R U' R'","R' F R F'","U2 R U2 R'","U2 R' F R F' U2 R' F R F'"]},{"id":"f2l-2","name":"F2L 2","variants":["y U' L' U L","F R' F' R","y' U' R' U R","r U R' U' M"]},{"id":"f2l-3","name":"F2L 3","variants":["y L' U' L","F' U' F","y' R' U' R","U M R U' R' U M'"]},{"id":"f2l-4","name":"F2L 4","variants":["R U R'","R U' R2 F R F' R U2 R'","R' D' r U r' D R2 U R'","U2 L R U' R' U L'"]},{"id":"f2l-5","name":"F2L 5","variants":["U' R U R' U2 R U' R'","U' R U R' U' R U2 R'","U' R U R' U R' F R F'","F2 L' U' L U F2"]},{"id":"f2l-6","name":"F2L 6","variants":["d R' U' R U2 R' U R","y U L' U' L U2 L' U L","d R' U' R U R' U2 R","U' r U' R' U R U r'"]},{"id":"f2l-7","name":"F2L 7","variants":["U' R U2 R' U2 R U' R'","U' R U2 R' U' R U2 R'","U' R U2 R' U R' F R F'","M' U' M U2 r U' r'"]},{"id":"f2l-8","name":"F2L 8","variants":["d R' U2 R U2 R' U R","r' U2 R2 U R2' U r","d R' U2 R U R' U2 R","y U L' U2 L U2 L' U L"]},{"id":"f2l-9","name":"F2L 9","variants":["U' R U' R' U F' U' F","F R U R' U' F' R U' R'","U' R U' R' d R' U' R","y' U R' U' R U' R' U' R"]},{"id":"f2l-10","name":"F2L 10","variants":["U' R U R' U R U R'","U y' R' U R U' y R U R'","U2 R U' R' U' R U R'","d R' U R d' R U R'"]},{"id":"f2l-11","name":"F2L 11","variants":["U' R U2 R' U F' U' F","U' R U2 R' d R' U' R","y' R U2 R2 U' R2 U' R'","R U' R' U R U' R2 F R F' R U' R'"]},{"id":"f2l-12","name":"F2L 12","variants":["R U' R' U R U' R' U2 R U' R'","R' U2 R2 U R2 U R","U F' U2 F U' R U R'","U R U' R' U' R U R' U' R U R'"]},{"id":"f2l-13","name":"F2L 13","variants":["y' U R' U R U' R' U' R","d R' U R U' R' U' R","M' U' R U R' U' R U2 r'","U R U' R' U R U' R' U' F' U F"]},{"id":"f2l-14","name":"F2L 14","variants":["U' R U' R' U R U R'","y L' U2 L U2 F U2 F' U F U' F'","R U' R' U R U2 R' U2 R U' R'"]},{"id":"f2l-15","name":"F2L 15","variants":["R U R' U2 R U' R' U R U' R'","y L' U L U2 y' R U R'","R U2 R' U R U R' U R U' R'","y' R' U R U2 y R U R'"]},{"id":"f2l-16","name":"F2L 16","variants":["R U' R' U2 F' U' F","R U' R' U2 y L' U' L","R2 B' R' B R' U2 R U' R'","U F R U R' U' F' U R U' R'"]},{"id":"f2l-17","name":"F2L 17","variants":["R U2 R' U' R U R'","U' R U' R' U2 R U2 R2 F R F'","y L F' L' F L' U L U' L' U L"]},{"id":"f2l-18","name":"F2L 18","variants":["y' R' U2 R U R' U' R","R' F R F' R U' R' U R U' R'","y L' U2 L U L' U' L","R U R' U' R U R' U' F R' F' R"]},{"id":"f2l-19","name":"F2L 19","variants":["U R U2 R' U R U' R'","R U' R' U R U' R' U R U R'","U R U2 R2 F R F'","y U F U2 F2 L F L'"]},{"id":"f2l-20","name":"F2L 20","variants":["y' U' R' U2 R U' R' U R","y U' L' U2 L U' L' U L","y L' U L U' L' U L U' L' U' L","y' R' U R U' R' U R U' R' U' R"]},{"id":"f2l-21","name":"F2L 21","variants":["R U' R' U2 R U R'","U2 R U R' U R U' R'","y F R U2' R' F'","U2 R U R2 F R F'"]},{"id":"f2l-22","name":"F2L 22","variants":["F' L' U2 L F","y' R' U R U2 R' U' R","r U' r' U2 r U r'","y L' U L U2 L' U' L"]},{"id":"f2l-23","name":"F2L 23","variants":["R U R' U2 R U R' U' R U R'","U R U' R' U' R U' R' U R U' R'","U2 R2 U2 R' U' R U' R2","U R U' R' U' R U' R2 F R F'"]},{"id":"f2l-24","name":"F2L 24","variants":["F U R U' R' F' R U' R'","y U' L' U L U L' U L U' L' U L","U' R U R2' F R F' R U' R'","U' R U R' U R U' R' U y' R' U R"]},{"id":"f2l-25","name":"F2L 25","variants":["R' F' R U R U' R' F","R' U' R' U' R' U R U R","y U' L' U L y' U R U' R'","U' R' F R F' R U R'"]},{"id":"f2l-26","name":"F2L 26","variants":["U R U' R' U' F' U F","y r U r' U' r' F r F'","U R U' R' F R' F' R","U R U' R' d' L' U L"]},{"id":"f2l-27","name":"F2L 27","variants":["R U' R' U R U' R'","R U' R2 F R F'","y L' U' L U2 F U' F'"]},{"id":"f2l-28","name":"F2L 28","variants":["y L' U L U' L' U L","y' R' U R U' R' U R","R U R' U' F R' F' R","R U R' U2 F' U F"]},{"id":"f2l-29","name":"F2L 29","variants":["R' F R F' R' F R F'","y' R' U' R U R' U' R","y L' U' L U L' U' L","M' U R U' r' U R U' R'"]},{"id":"f2l-30","name":"F2L 30","variants":["R U R' U' R U R'","F R' F' R F R' F' R","y U' L' U' L U2 F U2 F' U F U' F'"]},{"id":"f2l-31","name":"F2L 31","variants":["U' R' F R F' R U' R'","l F' R U' R' U l'","R U' R' y L' U2 L","U' R' F R F' U' R' F R F'"]},{"id":"f2l-32","name":"F2L 32","variants":["R U R' U' R U R' U' R U R'","U R U' R' U R U' R' U R U' R'","y L' U' L U L' U' L U L' U' L","R2 U R2' U R2 U2' R2"]},{"id":"f2l-33","name":"F2L 33","variants":["U' R U' R' U2 R U' R'","U' R U' R' U R' F R F'","U' R U' R' U' R U2 R'","y R' D R U' R' D' R"]},{"id":"f2l-34","name":"F2L 34","variants":["U R U R' U2 R U R'","U' R U2' R' U R U R'","y U L' U L U2 L' U L","y' U R' U R U2' R' U R"]},{"id":"f2l-35","name":"F2L 35","variants":["U' R U R' U F' U' F","U M' U R U' r' R U' R'","U2 R U R' F R' F' R","U' l U' R U l' U R'"]},{"id":"f2l-36","name":"F2L 36","variants":["U2 R' F R F' U2 R U R'","U F' U' F U' R U R'","R2 u R U R' U' u' R' U R'","y U L' U' L y' U' R U R'"]},{"id":"f2l-37","name":"F2L 37","variants":["R2 U2 F R2 F' U2 R' U R'","R U' R' d R' U2 R U2' R' U R","R U R' U2' R U2 R' U F' U' F","R' F R F' R U' R' U R U' R' U2 R U' R'"]},{"id":"f2l-38","name":"F2L 38","variants":["R U R' U' R U2 R' U' R U R'","R2 U2 R' U' R U' R' U2 R'","R U' R' U' R U R' U2 R U' R'","F R' F' R2 U2 R' U' R U R'"]},{"id":"f2l-39","name":"F2L 39","variants":["R U R' U2 R U' R' U R U R'","R U' R' U R U2 R' U R U' R'","R U2 R' U R U' R' U R U R'","R U2 R U R' U R U2 R2"]},{"id":"f2l-40","name":"F2L 40","variants":["R U' R' F R U R' U' F' R U' R'","r U' r' U2 r U r' R U R'","F' L' U2 L F R U R'","R F U R U' R' F' U' R'"]},{"id":"f2l-41","name":"F2L 41","variants":["R U R' U' R U' R' U2 y' R' U' R","R U F R U R' U' F' R'","R U' R' F' L' U2 L F","R U' M' U' r' U2 r U r'"]}];
/* Reconstruction evidence and state-verified case alternatives. GPL-3.0-only. */
var reviewCorners=[[8,9,20],[6,18,38],[0,36,47],[2,45,11],[29,26,15],[27,44,24],[33,53,42],[35,17,51]];
var reviewEdges=[[5,10],[7,19],[3,37],[1,46],[32,16],[28,25],[30,43],[34,52],[23,12],[21,41],[50,39],[48,14]];
function reviewSlots(face) {
 var solved=mathlib.SOLVED_FACELET;
 return reviewCorners.map(function(ids,i){var colors=ids.map(function(n){return solved[n];});if(colors.indexOf(face)<0)return null;
  var sides=colors.filter(function(c){return c!==face;}),edge=reviewEdges.findIndex(function(e){return e.every(function(n){return sides.indexOf(solved[n])>=0;});});
  return {corner:i,edge:edge,name:colors.join('')};
 }).filter(function(s){return s!==null;});
}
function reviewPairSolved(c,s){return c.ca[s.corner]===s.corner&&c.ea[s.edge]===s.edge*2;}
function reviewCrossSolved(c,face){return reviewEdges.every(function(e,i){return !e.some(function(n){return mathlib.SOLVED_FACELET[n]===face;})||c.ea[i]===i*2;});}
function reviewF2LVariants(start,face,targets,slots) {
 var preserved=slots.filter(function(s){return reviewPairSolved(start,s);}),seen={},out=[];
 var top='URFDLB'.indexOf({U:'D',D:'U',R:'L',L:'R',F:'B',B:'F'}[face]);
 nativeF2LCatalog.forEach(function(item){item.variants.forEach(function(algorithm,variantIndex){
  var best=null;
  ['', 'x','x2',"x'",'z',"z'"].forEach(function(tilt){['','y','y2',"y'"].forEach(function(y){
   var oriented=new mathlib.CubieCube().init(start.ca,start.ea);[tilt,y].filter(Boolean).forEach(function(m){oriented.selfMoveStr(m);});
   if(Math.floor(mathlib.CubieCube.rotMulM[oriented.ori||0][0]/3)!==top)return;
   ['','U','U2',"U'"].forEach(function(pre){
    var c=new mathlib.CubieCube().init(start.ca,start.ea),seq=[tilt,y,pre].filter(Boolean).concat(tokens(algorithm));
    seq.forEach(function(m){c.selfMoveStr(m);});
    if(!reviewCrossSolved(c,face)||!targets.every(function(s){return reviewPairSolved(c,s);})||!preserved.every(function(s){return reviewPairSolved(c,s);}))return;
    var moves=recons.getMoveCnt(seq.join(' ')),rotations=seq.filter(function(m){return /^[xyz]/.test(m);}).length;
    if(!best||moves<best.moves||moves===best.moves&&rotations<best.rotations)best={name:item.name,caseId:item.id,variantIndex:variantIndex,algorithm:seq.join(' '),moves:moves,rotations:rotations,verified:true,endState:c.toFaceCube()};
   });
  });});
  if(best&&!seen[best.algorithm]){seen[best.algorithm]=true;out.push(best);}
 });});
 return out.sort(function(a,b){return a.moves-b.moves||a.rotations-b.rotations;});
}
var reviewViewRotations={U:'z2',D:'',F:"x'",B:'x',R:'z',L:"z'"};
var reviewInverseView={U:'z2',D:'',F:'x',B:"x'",R:"z'",L:'z'};
var reviewRotationNames=null;
function reviewViewAlgorithm(algorithm,face,stage){
 var moves=tokens(algorithm);
 if(stage==='Cross'){
  var frame={U:'DLFURB',F:'BRUFLD',B:'FRDBLU',R:'LUFRDB',L:'RDFLUB',D:'URFDLB'}[face];
  return moves.map(function(m){return 'URFDLB'[frame.indexOf(m[0])]+m.slice(1);}).join(' ');
 }
 if(!reviewRotationNames){
  reviewRotationNames={0:[]};var queue=[{ori:0,moves:[]}];
  for(var i=0;i<queue.length;i++)['x',"x'",'x2','y',"y'",'y2','z',"z'",'z2'].forEach(function(m){var c=new mathlib.CubieCube();c.ori=queue[i].ori;c.selfMoveStr(m);if(reviewRotationNames[c.ori]===undefined){var seq=queue[i].moves.concat(m);reviewRotationNames[c.ori]=seq;queue.push({ori:c.ori,moves:seq});}});
 }
 var seq=(reviewInverseView[face]?[reviewInverseView[face]]:[]).concat(moves),c=new mathlib.CubieCube(),at=0;
 while(at<seq.length&&/^[xyz]/.test(seq[at]))c.selfMoveStr(seq[at++]);
 return reviewRotationNames[c.ori||0].concat(seq.slice(at)).join(' ');
}
function nativeReview(record) {
 var moves=record.moves||[],splits=record.analysis&&record.analysis.splits||[],face=record.analysis&&record.analysis.cross||record.cross||'D';
 var total=splits.reduce(function(n,s){return n+((s.rawMoves===undefined?s.moves:s.rawMoves)||0);},0);
 if(total!==moves.length||!splits.length)return {groups:[],classified:false};
 var state=nativeCube(record.initialState),states=[new mathlib.CubieCube().init(state.ca,state.ea)],groups=[],offset=0,slots=reviewSlots(face);
 moves.forEach(function(m){state.selfMoveStr(m.move);states.push(new mathlib.CubieCube().init(state.ca,state.ea));});
 function add(stage,label,start,end,targets){
  var own=moves.slice(start,end).map(function(m){return m.move;}).join(' '),variants=[],names=[],ids=[],group={stage:stage,label:label,rawStart:start,rawEnd:end,own:own,variants:variants};
  if(start===end){group.caseName='Skipped';groups.push(group);return;}
  if(stage==='F2L'&&targets.length)variants=reviewF2LVariants(states[start],face,targets,slots);
  else if(stage==='Cross'){
   var route=JSON.parse(nativeRequest(JSON.stringify({route:'native-cross-dashboard',body:{state:states[start].toFaceCube(),faces:face}})));
   variants=route.routes.map(function(r){return {name:r.kind==='xcross'?'Xcross':'Cross',algorithm:r.fixedMoves.join(' '),moves:r.fixedMoves.length,verified:r.verified};});
  }else if(stage==='OLL'||stage==='PLL'){
   // The existing verifier tests orientation/AUF and the complete phase goal.
   var data=JSON.parse(nativeRequest(JSON.stringify({route:'native-algorithms',body:{record:record,stage:stage}})));
   variants=data.variants.map(function(v){v.verified=true;return v;});
   (data.matches||[]).forEach(function(m){if(ids.indexOf(m.id)<0)ids.push(m.id);if(names.indexOf(m.name)<0)names.push(m.name);});
   if(data.caseId&&ids.indexOf(data.caseId)<0)ids.push(data.caseId);
  }
  variants.forEach(function(v){if(v.caseId&&ids.indexOf(v.caseId)<0)ids.push(v.caseId);if(v.name&&v.caseId&&names.indexOf(v.name)<0)names.push(v.name);});
  if(stage==='PLL'&&!ids.length){
   var top={U:'D',D:'U',R:'L',L:'R',F:'B',B:'F'}[face];
   ['', '2', "'"].forEach(function(suffix){var c=new mathlib.CubieCube().init(states[start].ca,states[start].ea);c.selfMoveStr(top+suffix);if(c.ca.every(function(v,i){return v===i;})&&c.ea.every(function(v,i){return v===i*2;}))variants.push({name:'AUF',algorithm:top+suffix,moves:1,verified:true});});
  }
  variants.forEach(function(v){v.viewAlgorithm=reviewViewAlgorithm(v.algorithm,face,stage);});
  group.variants=variants;group.caseIds=ids;
  group.caseName=stage==='Cross'?'Cross':stage==='PLL'&&variants.some(function(v){return v.name==='AUF';})?'AUF':ids.length===1?(names[0]||ids[0]):ids.length>1?'Multiple matching patterns':'Unclassified';
  groups.push(group);
 }
 splits.forEach(function(split){
  var n=split.rawMoves===undefined?split.moves:split.rawMoves,end=offset+n,stage=split.stage;
  if(stage!=='F2L'||!n){add(stage,stage,offset,end,[]);offset=end;return;}
  var events={};
  slots.forEach(function(slot){
   if(!reviewPairSolved(states[end],slot))return;
   // Opening a face for an insertion temporarily displaces already solved slots.
   // Compare cross-restored checkpoints, not those intermediate trigger states.
   var last=-1;for(var i=offset;i<end;i++)if(reviewCrossSolved(states[i],face)&&!reviewPairSolved(states[i],slot))last=i;
   if(last<0)return;
   var at=last+1;while(at<end&&(!reviewCrossSolved(states[at],face)||!reviewPairSolved(states[at],slot)))at++;
   (events[at]||(events[at]=[])).push(slot);
  });
  var before=offset,count=0;
  Object.keys(events).map(Number).sort(function(a,b){return a-b;}).forEach(function(at){count++;add('F2L','Pair '+count,before,at,events[at]);before=at;});
  if(before<end)add('F2L','F2L',before,end,slots.filter(function(s){return !reviewPairSolved(states[before],s)&&reviewPairSolved(states[end],s);}));
  offset=end;
 });
 return {groups:groups,classified:true};
}

/* Headless csTimer solver bridge. GPL-3.0-only. No UI, DOM, network or history. */
function nativeMoves(text) { return text.trim().split(/\s+/).filter(function(x){return !!x;}); }
function nativeCube(state) { var c=new mathlib.CubieCube(); if(c.fromFacelet(state)===-1) throw Error('Invalid cube state');return c; }
function nativeApply(state,moves) {var c=nativeCube(state);moves.forEach(function(m){c.selfMoveStr(m);});return c.toFaceCube();}

function nativeVerifiedCrossRoute(state,face,rotation,moves,kind){
 var c=nativeCube(state),fixed=[];
 nativeMoves(rotation).forEach(function(m){c.selfMoveStr(m);});
 moves.forEach(function(m){var index=c.selfMoveStr(m);if(typeof index!=="number")throw Error('Unsupported solver turn');fixed.push('URFDLB'.charAt(Math.floor(index/3))+['','2',"'"][index%3]);});
 var end=c.toFaceCube(),solved=mathlib.SOLVED_FACELET;
 var edges=[[5,10],[7,19],[3,37],[1,46],[32,16],[28,25],[30,43],[34,52],[23,12],[21,41],[50,39],[48,14]];
 var corners=[[8,9,20],[6,18,38],[0,36,47],[2,45,11],[29,26,15],[27,44,24],[33,53,42],[35,17,51]];
 function correct(ids){return ids.every(function(i){return end.charAt(i)===solved.charAt(i);});}
 var ok=edges.filter(function(e){return e.some(function(i){return solved.charAt(i)===face;});}).every(correct);
 if(kind==='xcross')ok=ok&&corners.some(function(corner){
  if(!corner.some(function(i){return solved.charAt(i)===face;})||!correct(corner))return false;
  var sides=corner.map(function(i){return solved.charAt(i);}).filter(function(f){return f!==face;});
  return edges.some(function(e){return e.every(function(i){return sides.indexOf(solved.charAt(i))>=0;})&&correct(e);});
 });
 if(!ok)throw Error('Cross route failed state verification');
 return {face:face,rotation:rotation,moves:moves,length:moves.length,kind:kind,fixedMoves:fixed,verified:true};
}

function nativeRequest(raw) {
 var request=JSON.parse(raw), body=request.body||{}, result;
 if(request.route==='native-review') { return JSON.stringify(nativeReview(body.record)); }
 if(request.route==='native-algorithms') {
  var r=body.record,method=body.stage||'PLL',splits=r.analysis&&r.analysis.splits||[],split=splits.filter(function(s){return s.stage===method;})[0];
  if(!split||!r.moves||!r.moves.length)throw Error('No reconstructed case');
  var before=0;for(var i=0;i<splits.length&&splits[i]!==split;i++)before+=splits[i].rawMoves||splits[i].moves;
  var start=nativeCube(r.initialState);r.moves.slice(0,before).forEach(function(m){start.selfMoveStr(m.move);});
  var own=r.moves.slice(before,before+(split.rawMoves||split.moves)).map(function(m){return m.move;}).join(' '),variants=[],seen={},matches=[];
  var cases=nativeCatalog.cases.filter(function(c){return c.stage===method&&(!split.caseId||c.id===split.caseId);});
  cases.forEach(function(item){item.variants.forEach(function(alg,variantIndex){var candidate=align(alg,start,method);if(candidate&&!matches.some(function(m){return m.id===item.id;}))matches.push({id:item.id,name:item.name});if(!candidate||seen[candidate.algorithm]||tokens(candidate.algorithm).join(' ')===tokens(own).join(' '))return;seen[candidate.algorithm]=true;candidate.name=item.name;candidate.caseId=item.id;candidate.variantIndex=variantIndex;variants.push(candidate);});});
  result={own:own,stage:method,caseId:split.caseId,matches:matches,variants:variants.sort(function(a,b){return a.moves-b.moves||a.rotations-b.rotations;})};
 } else if(request.route==='native-technique') {
  var record=body.record;
  result=technique([null,null,null,null,[record.moves.map(function(m){return m.move+'@'+Math.round(m.t);}).join(' '),'333']]);
 } else if(request.route==='scramble') {
  var state=min2phase.randomCube(), text=new min2phase.Search().solution(state,21,100000000,50,2).trim();
  if(/^Error/.test(text)||nativeApply(mathlib.SOLVED_FACELET,nativeMoves(text))!==state)throw Error('Scramble verification failed');
  result={scramble:text,state:state};
 } else if(request.route==='recovery') {
  var current=nativeCube(body.state),target=nativeCube(body.target),inverse=new mathlib.CubieCube(),relative=new mathlib.CubieCube();
  inverse.invFrom(target);mathlib.CubieCube.CubeMult(inverse,current,relative);
  var text=new min2phase.Search().solution(relative.toFaceCube(),21,100000000,50,0).trim(),moves=nativeMoves(text);
  if(/^Error/.test(text)||nativeApply(body.state,moves)!==body.target)throw Error('Recovery verification failed');
  result={moves:moves};
 } else if(request.route==='cross'||request.route==='native-tools'||request.route==='native-cross-dashboard') {
  var scramble=body.scramble,state=body.state;
  if(state){scramble=new min2phase.Search().solution(state,21,100000000,50,2).trim();if(/^Error/.test(scramble)||nativeApply(mathlib.SOLVED_FACELET,nativeMoves(scramble))!==state)throw Error('Initial-state scramble verification failed');}
  else {if(!scramble)throw Error('No scramble or initial state');state=nativeApply(mathlib.SOLVED_FACELET,nativeMoves(scramble));}
  var parsed=cubeutil.parseScramble(scramble,'FRUBLD');
  var faces='DULRFB',rotations=['','z2',"z'",'z',"x'",'x'];
  var all=cross.solve(parsed),routes=[];
  if(request.route==='native-cross-dashboard') {
   var requested=body.faces||'UD';
   requested.split('').forEach(function(face){var i=faces.indexOf(face);if(i<0)throw Error('Unknown cross face');
    routes.push(nativeVerifiedCrossRoute(state,face,rotations[i],all[i].map(function(m){return m.trim();}),'cross'));
    routes.push(nativeVerifiedCrossRoute(state,face,rotations[i],cross.solveXCross(parsed,i).map(function(m){return m.trim();}),'xcross'));
   });
   result={routes:routes};
  } else {
   var indices=body.kind==='xcross'?[body.face||0]:[0,1,2,3,4,5];
   routes=indices.map(function(i){var moves=body.kind==='xcross'?cross.solveXCross(parsed,i):all[i];return nativeVerifiedCrossRoute(state,faces[i],rotations[i],moves.map(function(m){return m.trim();}),body.kind==='xcross'?'xcross':'cross');});
   if(request.route==='cross') {var r=routes[faces.indexOf(body.cross||'U')];result={distance:r.length,moves:(r.rotation?[r.rotation]:[]).concat(r.moves),cross:r.face};}
   else result={routes:routes};
  }
 } else throw Error('Unsupported local solver request');
 return JSON.stringify(result);
}
