pub const APP_CSS: &str = r#"
:root { color-scheme: light; font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; color: #20211f; background: #f3f1eb; font-synthesis: none; }
* { box-sizing: border-box; }
body { margin: 0; min-width: 320px; min-height: 100vh; background: radial-gradient(circle at 15% 12%, rgba(212,231,198,.68), transparent 30rem), radial-gradient(circle at 88% 88%, rgba(237,218,187,.72), transparent 27rem), #f3f1eb; }
button { font: inherit; }
.shell { min-height: 100vh; display: grid; place-items: center; padding: 28px; }
.app-card { width: min(100%, 680px); min-height: min(740px, calc(100vh - 56px)); display: flex; flex-direction: column; align-items: center; padding: 34px 60px 26px; overflow: hidden; background: rgba(255,255,252,.82); border: 1px solid rgba(255,255,255,.9); border-radius: 32px; box-shadow: 0 30px 80px rgba(61,57,46,.12), inset 0 1px 0 #fff; }
.brand { align-self: flex-start; display: flex; align-items: center; gap: 10px; color: #2e3729; font-size: 17px; font-weight: 700; letter-spacing: -.02em; }
.brand-mark { width: 30px; height: 30px; display: grid; place-items: center; color: white; background: #4d6544; border-radius: 10px 10px 10px 4px; font-size: 14px; }
.hero { max-width: 500px; margin-top: 74px; text-align: center; }
.eyebrow { margin: 0 0 15px; color: #728169; font-size: 11px; font-weight: 700; letter-spacing: .16em; }
h1 { margin: 0; color: #242722; font-size: 50px; line-height: 1.08; letter-spacing: -.055em; }
.intro { max-width: 430px; margin: 20px auto 0; color: #6a6c67; font-size: 16px; line-height: 1.6; }
.recorder { display: flex; flex-direction: column; align-items: center; margin: 46px 0 30px; }
.record-button { width: 104px; height: 104px; display: grid; place-items: center; padding: 0; color: white; background: #4d6544; border: 0; border-radius: 50%; box-shadow: 0 14px 28px rgba(62,86,54,.24), 0 0 0 10px rgba(77,101,68,.09); cursor: pointer; transition: transform .2s ease, box-shadow .2s ease; }
.record-button:hover:not(:disabled) { transform: translateY(-2px) scale(1.02); box-shadow: 0 18px 34px rgba(62,86,54,.3), 0 0 0 12px rgba(77,101,68,.08); }
.record-button:active:not(:disabled) { transform: scale(.97); }
.record-button:disabled { cursor: wait; opacity: .8; }
.record-button svg { width: 38px; fill: currentColor; }
.record-button.recording { background: #b75a50; box-shadow: 0 14px 28px rgba(160,68,58,.22), 0 0 0 10px rgba(183,90,80,.09); animation: breathe 2s ease-in-out infinite; }
.stop-icon { width: 28px; height: 28px; background: white; border-radius: 7px; }
.spinner { width: 30px; height: 30px; border: 3px solid rgba(255,255,255,.35); border-top-color: white; border-radius: 50%; animation: spin .8s linear infinite; }
.status-row { height: 24px; display: flex; align-items: center; gap: 8px; margin-top: 24px; }
.status { margin: 0; color: #767772; font-size: 13px; }
.live-dot { width: 7px; height: 7px; background: #c86358; border-radius: 50%; animation: blink 1.3s ease infinite; }
.result { width: 100%; margin-bottom: 28px; padding: 22px 24px 25px; background: #fafaf7; border: 1px solid #e7e6df; border-radius: 20px; animation: reveal .35s ease both; }
.result-heading { display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px; color: #8a8b85; font-size: 10px; font-weight: 700; letter-spacing: .15em; }
.copy-button { padding: 7px 11px; color: #53634d; background: #edf1e9; border: 0; border-radius: 9px; font-size: 12px; font-weight: 600; letter-spacing: normal; cursor: pointer; }
.copy-button:hover { background: #e2e9dd; }
.transcript { margin: 0; color: #333531; font-size: 17px; line-height: 1.65; white-space: pre-wrap; }
.error { width: 100%; margin: -2px 0 26px; padding: 13px 15px; color: #94463f; background: #fbefec; border: 1px solid #efd7d3; border-radius: 13px; font-size: 13px; line-height: 1.45; }
footer { margin-top: auto; display: flex; align-items: center; gap: 7px; color: #989992; font-size: 10px; text-align: center; }
.privacy-dot { width: 6px; height: 6px; background: #89a17e; border-radius: 50%; }
@keyframes spin { to { transform: rotate(360deg); } }
@keyframes blink { 50% { opacity: .35; } }
@keyframes breathe { 50% { box-shadow: 0 16px 34px rgba(160,68,58,.27), 0 0 0 16px rgba(183,90,80,.04); } }
@keyframes reveal { from { opacity: 0; transform: translateY(8px); } }
@media (max-width: 600px) { .shell { padding: 0; } .app-card { min-height: 100vh; border-radius: 0; border: 0; padding: 25px 24px; } .hero { margin-top: 64px; } h1 { font-size: 40px; } }
@media (prefers-reduced-motion: reduce) { *, *::before, *::after { animation-duration: .01ms !important; animation-iteration-count: 1 !important; } }
"#;
