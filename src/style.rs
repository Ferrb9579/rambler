pub const APP_CSS: &str = r#"
:root {
    color-scheme: light dark;
    font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    font-synthesis: none;
    background: transparent;
}

* { box-sizing: border-box; }

html, body, #main, .app-root {
    width: 100%;
    height: 100%;
    margin: 0;
    overflow: hidden;
    background: transparent;
}

.app-root {
    --text: #171817;
    --muted: #6f716f;
    --surface: #ffffff;
    --surface-raised: #ffffff;
    --surface-soft: #f2f3f2;
    --surface-strong: #e8e9e8;
    --border: rgba(0, 0, 0, 0.09);
    --hover: rgba(0, 0, 0, 0.07);
    --accent: #315fdb;
    --accent-soft: #e8eeff;
    --danger: #b12f2a;
    --success: #277c43;
    color: var(--text);
}

.app-root.theme-dark {
    --text: #f1f3f1;
    --muted: #a7aaa7;
    --surface: #202220;
    --surface-raised: #292b29;
    --surface-soft: #2d302d;
    --surface-strong: #353835;
    --border: rgba(255, 255, 255, 0.11);
    --hover: rgba(255, 255, 255, 0.09);
    --accent: #91acff;
    --accent-soft: #29375f;
    --danger: #ff938b;
    --success: #7bd598;
}

@media (prefers-color-scheme: dark) {
    .app-root.theme-system {
        --text: #f1f3f1;
        --muted: #a7aaa7;
        --surface: #202220;
        --surface-raised: #292b29;
        --surface-soft: #2d302d;
        --surface-strong: #353835;
        --border: rgba(255, 255, 255, 0.11);
        --hover: rgba(255, 255, 255, 0.09);
        --accent: #91acff;
        --accent-soft: #29375f;
        --danger: #ff938b;
        --success: #7bd598;
    }
}

button, textarea, input { font: inherit; }
button { color: inherit; }
html { touch-action: none; }

.app-shell {
    position: relative;
    width: 100%;
    height: 100%;
    padding: 2px;
    background: transparent;
    user-select: none;
}

.toolbar {
    position: relative;
    z-index: 1;
    width: 180px;
    height: 50px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 2px;
    padding: 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
    box-shadow: 0 3px 12px rgba(0, 0, 0, 0.16);
}

.icon-button, .round-button {
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    flex: 0 0 40px;
    padding: 0;
    color: var(--text);
    background: transparent;
    border: 0;
    border-radius: 50%;
    cursor: pointer;
    transition: color 120ms ease, background-color 120ms ease, transform 80ms ease;
}

.icon-button:hover:not(:disabled), .round-button:hover:not(:disabled) { background: var(--hover); }
.icon-button:active:not(:disabled), .round-button:active:not(:disabled) { transform: scale(0.9); }
.icon-button:focus-visible, .round-button:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
.icon-button:disabled, .round-button:disabled { cursor: default; opacity: 0.34; }
.drag-handle { color: var(--muted); cursor: grab; }
.drag-handle:active { cursor: grabbing; }

.primary-actions {
    height: 42px;
    display: flex;
    align-items: center;
    padding: 1px;
    background: var(--surface-strong);
    border-radius: 999px;
}

.copy-button.copied { color: var(--success); opacity: 1; }
.mic-button.recording { color: #ffffff; background: #d4493f; }
.mic-button.processing { color: var(--muted); opacity: 1; }
.processing-icon { animation: turn 1s linear infinite; }
.menu-button.active { background: var(--hover); }

.menu-dismiss-layer {
    position: absolute;
    z-index: 10;
    inset: 0;
    width: 100%;
    height: 100%;
    padding: 0;
    background: transparent;
    border: 0;
    cursor: default;
    opacity: 1;
    transition: opacity 90ms linear;
}
.menu-dismiss-layer.closed { pointer-events: none; opacity: 0; }

.menu-panel {
    position: relative;
    z-index: 11;
    width: 160px;
    margin: 6px 0 0 auto;
    padding: 5px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 16px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.20);
    opacity: 1;
    will-change: opacity;
    transition: opacity 90ms linear;
}
.menu-panel.closed { pointer-events: none; opacity: 0; }

.menu-item {
    width: 100%;
    height: 39px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 11px;
    color: var(--text);
    background: transparent;
    border: 0;
    border-radius: 11px;
    font-size: 13px;
    font-weight: 550;
    text-align: left;
    cursor: pointer;
}
.menu-item:hover:not(:disabled) { background: var(--hover); }
.menu-item:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
.menu-item.danger { color: var(--danger); }

.settings-page {
    width: calc(100% - 12px);
    height: calc(100% - 12px);
    margin: 6px;
    overflow: hidden;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 22px;
    box-shadow: 0 12px 38px rgba(0, 0, 0, 0.24);
}

.settings-header {
    height: 72px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px 10px 12px;
    border-bottom: 1px solid var(--border);
    user-select: none;
    cursor: grab;
}
.settings-header:active { cursor: grabbing; }
.settings-header .round-button { cursor: pointer; }
.settings-heading { min-width: 0; flex: 1; }
.settings-heading h1 { margin: 0; font-size: 19px; line-height: 1.25; }
.settings-heading p { margin: 3px 0 0; color: var(--muted); font-size: 12px; }
.header-mark { color: var(--muted); }

.settings-content {
    height: calc(100% - 72px);
    padding: 12px;
    overflow-y: auto;
    scrollbar-width: thin;
}

.settings-section {
    margin-bottom: 10px;
    padding: 14px;
    background: var(--surface-soft);
    border: 1px solid var(--border);
    border-radius: 17px;
}
.section-title { display: flex; align-items: flex-start; gap: 10px; }
.section-icon {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    flex: 0 0 32px;
    color: var(--accent);
    background: var(--accent-soft);
    border-radius: 50%;
}
.section-title h2 { margin: 0; font-size: 14px; line-height: 1.3; }
.section-title p { margin: 3px 0 0; color: var(--muted); font-size: 11.5px; line-height: 1.45; }
.section-body { margin-top: 12px; }

.vocabulary-editor {
    width: 100%;
    min-height: 106px;
    resize: vertical;
    padding: 10px 11px;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    outline: none;
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 12px;
    line-height: 1.55;
    user-select: text;
}
.vocabulary-editor:focus { border-color: var(--accent); box-shadow: 0 0 0 2px var(--accent-soft); }

.section-actions {
    min-height: 34px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 10px;
}
.status-text { flex: 1; color: var(--muted); font-size: 11px; overflow-wrap: anywhere; }

.action-button {
    min-height: 34px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    padding: 0 12px;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: 12px;
    font-weight: 650;
    cursor: pointer;
}
.action-button:hover:not(:disabled) { background: var(--hover); }
.action-button.primary { color: #ffffff; background: #315fdb; border-color: transparent; }
.action-button:disabled { cursor: default; opacity: 0.45; }
.round-button.subtle { width: 34px; height: 34px; flex-basis: 34px; background: var(--surface); border: 1px solid var(--border); }

.model-row, .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
}
.model-row div, .setting-row div { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
.model-row strong, .setting-row strong { font-size: 12.5px; }
.model-row span, .setting-row div span { color: var(--muted); font-size: 11px; }
.status-badge { flex: 0 0 auto; padding: 5px 8px; border-radius: 999px; font-size: 10.5px !important; font-weight: 650; }
.status-badge.installed { color: var(--success); background: color-mix(in srgb, var(--success) 12%, transparent); }
.status-badge.missing { color: var(--danger); background: color-mix(in srgb, var(--danger) 12%, transparent); }

.setting-row { min-height: 42px; cursor: pointer; }
.setting-row.stacked { align-items: flex-start; flex-direction: column; cursor: default; }
.segmented-control { width: 100%; display: grid !important; grid-template-columns: repeat(3, 1fr); gap: 3px !important; padding: 3px; background: var(--surface-strong); border-radius: 999px; }
.segmented-control button { height: 30px; color: var(--muted); background: transparent; border: 0; border-radius: 999px; font-size: 11.5px; cursor: pointer; }
.segmented-control button.selected { color: var(--text); background: var(--surface); box-shadow: 0 1px 4px rgba(0, 0, 0, 0.14); }

.switch {
    width: 38px;
    height: 22px;
    flex: 0 0 38px;
    appearance: none;
    background: var(--surface-strong);
    border-radius: 999px;
    cursor: pointer;
    transition: background 120ms ease;
}
.switch::after { content: ""; display: block; width: 18px; height: 18px; margin: 2px; background: #ffffff; border-radius: 50%; box-shadow: 0 1px 4px rgba(0, 0, 0, 0.25); transition: transform 120ms ease; }
.switch:checked { background: #315fdb; }
.switch:checked::after { transform: translateX(16px); }

.about-grid { display: grid; grid-template-columns: 78px 1fr; gap: 7px 12px; font-size: 11.5px; }
.about-grid span { color: var(--muted); }
.about-grid strong { font-weight: 600; }
.settings-save-status { margin: 4px 2px 12px; color: var(--muted); font-size: 11px; text-align: center; }

.sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
}

@keyframes turn { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) {
    *, *::before, *::after { animation-duration: 0.01ms !important; }
}
"#;
