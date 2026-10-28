// Presentation only: never rewrite token order or the active scramble cursor.
const escape = value => String(value).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&apos;'}[c]));
export function notation(tokens) {
    return (tokens ?? []).map(t => {
        const value = escape(t.text);
        if (t.kind === 1) return `<span alpha="35%">${value}</span>`;
        if (t.kind === 2) return `<span foreground="#8dc8ff" underline="single">${value}</span>`;
        if (t.kind === 3) return `<span foreground="#ff8a98">${value}</span>`;
        if (t.kind === 4) return `<span foreground="#edc879" underline="single">${value}</span>`;
        return value;
    }).join(' ');
}
export function splitRows(tokens) {
    const items = tokens ?? [];
    if (items.length < 2) return [items, []];
    // Balanced by monospaced glyph width; fixed slots prevent cursor-induced reflow.
    const widths = items.map(t => [...String(t.text)].length + 1);
    const total = widths.reduce((a, b) => a + b, 0);
    let width = 0, best = Infinity, cut = 1;
    for (let i = 1; i < items.length; i++) {
        width += widths[i - 1];
        const cost = Math.max(width, total - width);
        if (cost < best) { best = cost; cut = i; }
    }
    return [items.slice(0, cut), items.slice(cut)];
}

export function scrambleReady(state) {
    return !!state?.connected && !state.running && Array.isArray(state.tokens)
        && state.tokens.length > 0 && state.tokens.every(token => token.kind === 1);
}
