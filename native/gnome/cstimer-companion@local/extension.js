// GNOME Shell UI only. The Rust application remains the sole Bluetooth owner.
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GioUnix from 'gi://GioUnix';
import GLib from 'gi://GLib';
import Pango from 'gi://Pango';
import St from 'gi://St';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {notation, splitRows, scrambleReady} from './notation.js';

const NAME = 'org.cstimer.Native.Companion';
const PATH = '/org/cstimer/Native/Companion';
export default class CubeCompanion extends Extension {
    enable() {
        this._closed = false;
        const generation=this._generation=(this._generation??0)+1;
        this._pending = false;
        this._state = null;
        this._cancel = new Gio.Cancellable();
        this._button = new PanelMenu.Button(0.0, 'Cube companion', false);
        this._button.add_style_class_name('cube-companion');
        this._button.connect('destroy', () => this._cleanup());
        this._row = new St.BoxLayout({style_class: 'cube-companion-row', y_align: Clutter.ActorAlign.CENTER});
        this._dot = new St.Label({text: '●', style_class: 'cube-companion-dot', y_align: Clutter.ActorAlign.CENTER});
        this._status = new St.BoxLayout({style_class: 'cube-companion-status', y_align: Clutter.ActorAlign.CENTER});
        this._dot.x_align = Clutter.ActorAlign.CENTER;
        this._dot.x_expand = true;
        this._readyIcon = new St.Icon({icon_name: 'object-select-symbolic', icon_size: 8, style_class: 'cube-companion-ready', visible: false, y_align: Clutter.ActorAlign.CENTER});
        this._status.add_child(this._dot);
        this._status.add_child(this._readyIcon);
        this._statsBox = new St.BoxLayout({style_class: 'cube-companion-metrics', vertical: true, y_align: Clutter.ActorAlign.CENTER});
        this._time = new St.Label({text: '—', style_class: 'cube-companion-time'});
        this._stats = new St.BoxLayout({style_class: 'cube-companion-stats'});
        this._tps = new St.Label({text: '— tps', style_class: 'cube-companion-tps'});
        this._moves = new St.Label({text: '—', style_class: 'cube-companion-moves'});
        this._metricSeparator = new St.Label({text: '·', style_class: 'cube-companion-metric-separator'});
        this._stats.add_child(this._tps);
        this._stats.add_child(this._metricSeparator);
        this._stats.add_child(this._moves);
        const digits = new Pango.AttrList();
        digits.insert(Pango.attr_font_features_new('tnum'));
        for (const label of [this._time, this._tps, this._moves]) label.clutter_text.set_attributes(digits);
        this._statsBox.add_child(this._time);
        this._statsBox.add_child(this._stats);
        this._scrambleBox = new St.BoxLayout({style_class: 'cube-companion-notation', vertical: true, y_align: Clutter.ActorAlign.CENTER, clip_to_allocation: true});
        this._scrambleRows = [0, 1].map(() => {
            const row = new St.BoxLayout({style_class: 'cube-companion-scramble-row'});
            this._scrambleBox.add_child(row);
            return row;
        });
        this._row.add_child(this._status);
        this._row.add_child(this._scrambleBox);
        this._row.add_child(this._statsBox);
        this._button.add_child(this._row);
        const detail = new PopupMenu.PopupBaseMenuItem({reactive: false, can_focus: false});
        this._detail = new St.Label({text: '—', style_class: 'cube-companion-detail'});
        this._detail.clutter_text.line_wrap = true;
        this._detail.clutter_text.line_wrap_mode = Pango.WrapMode.WORD_CHAR;
        this._detail.set_style('width: 360px;');
        detail.add_child(this._detail);
        this._button.menu.addMenuItem(detail);
        const metrics = new PopupMenu.PopupBaseMenuItem({reactive: false, can_focus: false});
        this._metricDetail = new St.Label({text: '', style_class: 'cube-companion-detail'});
        metrics.add_child(this._metricDetail);
        this._button.menu.addMenuItem(metrics);
        this._button.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this._cubeStatus = new PopupMenu.PopupMenuItem('', {reactive: false});
        this._cubeStatus.label.clutter_text.line_wrap = true;
        this._cubeStatus.label.set_style('max-width: 320px;');
        this._button.menu.addMenuItem(this._cubeStatus);
        this._connect = this._button.menu.addAction('Connect cube', () => {
            if (!this._proxy?.g_name_owner) { this._startService(); return; }
            const c = this._state?.connection;
            this._cubeCall(!c?.powered ? 'PowerOnBluetooth' : c.connecting || this._state?.connected ? 'DisconnectCube' : c.saved ? 'ConnectCube' : 'ScanCubes', c?.powered && !c.connecting && !this._state?.connected && c.saved ? new GLib.Variant('(s)', ['saved']) : null);
        });
        this._devices = new PopupMenu.PopupMenuSection();
        this._button.menu.addMenuItem(this._devices);
        this._scan = this._button.menu.addAction('Find another cube', () => this._cubeCall('ScanCubes'));
        this._button.menu.connect('open-state-changed', (_menu, open) => { if (open) this._poll(); });
        this._button.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this._button.menu.addAction('Open timer', () => this._open(false));
        this._button.menu.addAction('Floating companion', () => this._open(true));
        this._button.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this._button.menu.addAction('Keep in background', () => {
            this._proxy?.call('HideMain', null, Gio.DBusCallFlags.NO_AUTO_START, 1000,
                this._cancel, (proxy, result) => { try { proxy.call_finish(result); } catch (_) {} });
        });
        this._button.menu.addAction('Quit', () => {
            this._proxy?.call('Quit', null, Gio.DBusCallFlags.NO_AUTO_START, 1000,
                this._cancel, (proxy, result) => { try { proxy.call_finish(result); } catch (_) {} });
        });
        Main.panel.addToStatusArea(this.uuid, this._button, -1, 'left');
        this._resizeSignal = Main.panel.connect('notify::allocation', () => this._fit());
        Gio.DBusProxy.new_for_bus(Gio.BusType.SESSION,
            Gio.DBusProxyFlags.DO_NOT_AUTO_START | Gio.DBusProxyFlags.DO_NOT_LOAD_PROPERTIES,
            null, NAME, PATH, NAME, this._cancel, (_source, result) => {
                if (this._closed || generation!==this._generation) return;
                try {
                    this._proxy = Gio.DBusProxy.new_for_bus_finish(result);
                    this._ownerSignal = this._proxy.connect('notify::g-name-owner', () => {
                        if (!this._proxy?.g_name_owner) this._offline();
                        this._poll();
                    });
                    this._poll();
                } catch (error) { if (!this._closed) console.warn(`Qbing companion: ${error.message}`); }
            });
        this._timer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 100, () => {
            if (this._closed) return GLib.SOURCE_REMOVE;
            const now = GLib.get_monotonic_time();
            if (now - (this._lastPoll ?? 0) >= (this._state?.connected || this._state?.running ? 100000 : 500000)) this._poll();
            return GLib.SOURCE_CONTINUE;
        });
        this._fit();
    }
    _poll() {
        if (this._closed || this._pending || !this._proxy?.g_name_owner) return;
        this._lastPoll = GLib.get_monotonic_time();
        this._pending = true;
        const generation=this._generation;
        this._proxy.call('GetSnapshot', null, Gio.DBusCallFlags.NO_AUTO_START, 1000, this._cancel, (proxy, result) => {
            if (this._closed || generation!==this._generation) return;
            this._pending = false;
            try {
                const [json] = proxy.call_finish(result).deep_unpack();
                this._state = JSON.parse(json);
                if(this._state.updatedAt && Date.now()-this._state.updatedAt>3000){this._offline();return;}
                this._render();
            } catch (_error) { this._offline(); }
        });
    }
    _render() {
        const s = this._state;
        this._setText(this._time, s.time ?? '—');
        this._setText(this._tps, `${s.tpsLabel ?? '—'} tps`);
        this._setText(this._moves, `${s.moves ?? '—'}`);
        this._metricDetail.text = `${s.time ?? '—'}   ${s.tpsLabel ?? '—'} tps   ${s.moves ?? '—'} moves`;
        const ready = scrambleReady(s);
        this._readyIcon.visible = ready;
        this._dot.visible = !ready;
        if (s.connected) this._dot.add_style_class_name('connected');
        else this._dot.remove_style_class_name('connected');
        const rows = splitRows(s.tokens);
        rows.forEach((tokens, i) => {
            const row = this._scrambleRows[i];
            let labels = row.get_children();
            while (labels.length > tokens.length) labels.pop().destroy();
            for (let j = 0; j < tokens.length; j++) {
                let label = labels[j];
                if (!label) {
                    label = new St.Label({style_class: 'cube-companion-token'});
                    label.clutter_text.ellipsize = Pango.EllipsizeMode.END;
                    row.add_child(label);
                }
                this._setText(label, String(tokens[j].text));
                if (label._kind !== tokens[j].kind) {
                    const state = ['', ' done', ' current', ' repair', ' partial'][tokens[j].kind] ?? '';
                    label.set_style_class_name(`cube-companion-token${state}`);
                    label._kind = tokens[j].kind;
                }
            }
        });
        const detailMarkup = notation(s.tokens) || '—';
        if (this._detailMarkup !== detailMarkup) {
            this._detail.clutter_text.set_markup(detailMarkup);
            this._detailMarkup = detailMarkup;
        }
        this._button.accessible_name = `Qbing. ${s.connected ? 'Cube connected' : 'Cube disconnected'}. ${ready ? 'Scramble ready. ' : ''}${this._metricDetail.text}`;
        this._renderConnection();
        this._fit();
    }
    _setText(label, text) {
        if (label.text !== text) label.text = text;
    }
    _offline() {
        if (this._closed) return;
        this._state = null;
        this._readyIcon.visible = false;
        this._dot.visible = true;
        this._dot.remove_style_class_name('connected');
        this._setText(this._time, '—');
        this._setText(this._tps, '— tps');
        this._setText(this._moves, '—');
        this._detailMarkup = null;
        this._metricDetail.text = '';
        this._scrambleRows.forEach(row => row.destroy_all_children());
        this._detail.text = '—';
        this._renderConnection();
        this._fit();
    }
    _fit() {
        if (this._closed) return;
        const left = Main.panel._leftBox;
        // Window titles surrender natural width before the companion does.
        const others = left.get_children().filter(actor => actor !== this._button.container && actor !== this._button)
            .reduce((width, actor) => width + actor.get_preferred_width(-1)[0], 0);
        const scale = St.ThemeContext.get_for_stage(global.stage).scale_factor;
        const center = Main.panel._centerBox.width;
        const free = Math.max(16 * scale, Main.panel.width / 2 - center / 2 - others - 8 * scale);
        const hasScramble = this._scrambleRows.some(row => row.get_n_children() > 0);
        const showMetrics = !!this._state && (!hasScramble || free >= 244 * scale);
        this._statsBox.visible = showMetrics;
        if (this._statsBox.width !== 88 * scale) this._statsBox.width = 88 * scale;
        // 8px status + 8px button padding, 6px before notation, and a 10px
        // gutter before the fixed 78px metrics. No empty trailing column.
        const fixed = (16 + (hasScramble ? 6 : 0) + (showMetrics ? 88 : 0)) * scale;
        this._scrambleBox.visible = hasScramble && free - fixed >= 50 * scale;
        const layoutKey = JSON.stringify([free, scale, showMetrics, this._scrambleRows.map(row => row.get_children().map(label => label.text))]);
        if (this._layoutKey === layoutKey) return;
        this._layoutKey = layoutKey;
        let notationWidth = 0;
        if (this._scrambleBox.visible) {
            const available = Math.floor(free - fixed);
            for (const size of [10, 9, 8]) {
                const style = `font-size: ${size}px;`;
                if (this._scrambleStyle !== style) {
                    this._scrambleBox.set_style(style);
                    this._scrambleStyle = style;
                }
                notationWidth = Math.ceil(Math.max(...this._scrambleRows.map(row => row.get_preferred_width(-1)[1])));
                if (notationWidth <= available) break;
            }
            notationWidth = Math.min(notationWidth, available);
            if (this._scrambleBox.width !== notationWidth + 6 * scale) this._scrambleBox.width = notationWidth + 6 * scale;
        }
        const width = Math.round(Math.min(free, (16 + (showMetrics ? 88 : 0)) * scale + (notationWidth ? notationWidth + 6 * scale : 0)));
        this._button.visible = true;
        if (this._button.width !== width) this._button.width = width;
    }
    _startService() {
        if (this._starting) return;
        this._starting = true;
        this._powerRequested = false;
        this._renderConnection();
        try {
            const child = Gio.Subprocess.new(['systemctl', '--user', 'start', 'qbing.service'], Gio.SubprocessFlags.STDOUT_SILENCE | Gio.SubprocessFlags.STDERR_SILENCE);
            child.wait_check_async(this._cancel, (process, result) => {
                if (this._closed) return;
                this._starting = false;
                try { process.wait_check_finish(result); this._connectAfterStart = true; this._cubeError = null; }
                catch (_) { this._cubeError = 'Could not start the cube service.'; }
                this._poll();
                this._renderConnection();
            });
        } catch (_) { this._starting = false; this._cubeError = 'Could not start the cube service.'; this._renderConnection(); }
    }
    _cubeCall(method, parameters = null) {
        if (!this._proxy?.g_name_owner || !this._state?.connection) { this._open(false); return; }
        this._connect.setSensitive(false);
        this._proxy.call(method, parameters, Gio.DBusCallFlags.NO_AUTO_START, 1000, this._cancel, (proxy, result) => {
            if (this._closed) return;
            try { proxy.call_finish(result); this._cubeError = null; }
            catch (error) { this._cubeError = error.message; }
            this._poll();
            this._renderConnection();
        });
    }
    _renderConnection() {
        const s = this._state, c = s?.connection;
        const online = !!this._proxy?.g_name_owner;
        this._connect.label.text = this._starting ? 'Starting…' : !online ? 'Connect cube' : !c ? 'Open connection settings' : !c.powered ? 'Turn on Bluetooth' : c.connecting ? 'Cancel connection' : s.connected ? 'Disconnect cube' : 'Connect cube';
        this._connect.setSensitive(!this._starting && !c?.scanning);
        this._scan.visible = !!c?.powered && !s?.connected && !c.connecting;
        this._scan.setSensitive(!c?.scanning);
        this._scan.label.text = c?.scanning ? 'Scanning…' : 'Find another cube';
        const message = this._cubeError || (c?.connecting ? 'Connecting…' : s?.connected ? c?.name : c?.scanning ? 'Scanning…' : c && !c.powered ? c.adapterStatus || c.message : c?.message);
        this._cubeStatus.label.text = message || '';
        this._cubeStatus.visible = !!message;
        if (s?.connected || c?.connecting) this._connectAfterStart = false;
        if (this._connectAfterStart && c && !c.connecting && !s.connected) {
            if (c.powered) {
                this._connectAfterStart = false;
                this._powerRequested = false;
                this._cubeCall(c.saved ? 'ConnectCube' : 'ScanCubes', c.saved ? new GLib.Variant('(s)', ['saved']) : null);
            } else if (!this._powerRequested) {
                this._powerRequested = true;
                this._cubeCall('PowerOnBluetooth');
            }
        }
        const devices = c?.powered && !s?.connected && !c.connecting ? c.devices ?? [] : [];
        const key = JSON.stringify(devices);
        if (this._deviceKey !== key) {
            this._devices.removeAll();
            for (const device of devices) {
                this._devices.addAction(device.name || 'GAN cube', () => this._cubeCall('ConnectCube', new GLib.Variant('(s)', [device.id])));
            }
            this._deviceKey = key;
        }
    }
    _open(companion) {
        if (this._proxy?.g_name_owner) {
            this._proxy.call(companion ? 'ShowCompanion' : 'ShowMain', null, Gio.DBusCallFlags.NO_AUTO_START,
                1000, this._cancel, (proxy, result) => { try { proxy.call_finish(result); } catch (_) {} });
        } else {
            const app = GioUnix.DesktopAppInfo.new('org.cstimer.Native.desktop');
            if (!app) return;
            const context = global.create_app_launch_context(0, -1);
            if (companion) app.launch_action('Floating', context);
            else app.launch([], context);
        }
    }
    _cleanup() {
        if(this._closed)return;
        this._closed = true;
        this._cancel?.cancel();
        if (this._timer) GLib.source_remove(this._timer);
        if (this._resizeSignal) Main.panel.disconnect(this._resizeSignal);
        if (this._ownerSignal) this._proxy?.disconnect(this._ownerSignal);
        this._timer = this._resizeSignal = this._ownerSignal = 0;
        this._proxy = null;
    }
    disable() {
        this._cleanup();
        this._button?.destroy();
        this._button = this._row = this._detail = this._metricDetail = this._state = null;
        this._status = this._readyIcon = null;
        this._connect = this._scan = this._devices = this._cubeStatus = null;
        this._metricSeparator = this._time = this._tps = this._moves = this._stats = this._statsBox = this._scrambleBox = this._scrambleRows = null;
    }
}
