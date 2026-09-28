// ==========================================================
// signals.js — محرك تفاعلية دقيق (fine-grained reactivity)
// مستوحى من Solid.js — خفيف، سريع، صفر تبعيات
// ==========================================================
'use strict';

let _currentSub = null;
let _batchDepth = 0;
const _pendingEffects = new Set();

// ─── signal ────────────────────────────────────────────────
function signal(initial) {
    let value = initial;
    const subs = new Set();

    function get() {
        if (_currentSub) subs.add(_currentSub);
        return value;
    }

    function set(v) {
        if (Object.is(v, value)) return;
        value = v;
        if (_batchDepth > 0) {
            for (const s of subs) _pendingEffects.add(s);
        } else {
            // نسخة لتفادي مشاكل التعديل أثناء التكرار
            const snapshot = Array.from(subs);
            for (const s of snapshot) {
                try { s(); } catch (e) { console.error("effect error:", e); }
            }
        }
    }

    function update(fn) { set(fn(value)); }
    function peek() { return value; }
    function subscribe(fn) {
        if (_currentSub) subs.add(_currentSub);
        subs.add(fn);
        return () => subs.delete(fn);
    }

    return { get, set, update, peek, subscribe, _subs: subs, _isSignal: true };
}

// ─── effect ────────────────────────────────────────────────
function createEffect(fn) {
    let _prev = _currentSub;
    let mounted = false;

    function run() {
        if (mounted) return; // منع إعادة الدخول
        mounted = true;
        _currentSub = run;
        try { fn(); }
        finally { _currentSub = _prev; mounted = false; }
    }

    run();
    return run;
}

// ─── memo ──────────────────────────────────────────────────
function createMemo(fn) {
    const s = signal(undefined);
    createEffect(() => {
        // نستخدم try/finally لضمان عدم تعطّل باقي الإشارات عند الخطأ
        try { s.set(fn()); } catch (e) { console.error("memo error:", e); }
    });
    return s;
}

// ─── batch ─────────────────────────────────────────────────
function batch(fn) {
    _batchDepth++;
    try { fn(); }
    finally {
        _batchDepth--;
        if (_batchDepth === 0 && _pendingEffects.size > 0) {
            const pending = Array.from(_pendingEffects);
            _pendingEffects.clear();
            for (const e of pending) {
                try { e(); } catch (err) { console.error("batch effect error:", err); }
            }
        }
    }
}

// ─── helpers ───────────────────────────────────────────────
function untrack(fn) {
    const prev = _currentSub;
    _currentSub = null;
    try { return fn(); }
    finally { _currentSub = prev; }
}

// نستخدمها في مولّد الكود للإشارة لقائمة في JS
function _html_list(items, renderFn) {
    let out = "";
    if (!items) return out;
    for (const item of items) out += renderFn(item);
    return out;
}

// إشارة القائمة الحالية لمحرر HTML — تُستخدم في stmt_to_html_js
function _list_render(src, renderFn) {
    const arr = (src && src._isSignal) ? src.get() : src;
    return _html_list(arr, renderFn);
}