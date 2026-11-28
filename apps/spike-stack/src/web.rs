//! PROTOTYPE: the Relative as HTML in WebView2 (through wry), hosted in the same
//! click-through windows as the native stack. Each snapshot is pushed as one
//! `evaluate_script` call per WebView; the page updates only changed cells.

use std::fmt::Write as _;
use std::num::NonZeroIsize;

use anyhow::{Result, anyhow};
use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use windows::Win32::Foundation::HWND;

use crate::relative::{ROWS, Snap};
use crate::{Layout, Placement, Renderer, WIDGET_H, WIDGET_W, win};

struct Parent(HWND);

impl HasWindowHandle for Parent {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let h = Win32WindowHandle::new(NonZeroIsize::new(self.0.0 as isize).ok_or(HandleError::Unavailable)?);
        // SAFETY: the window lives as long as `self`.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(h)) })
    }
}

pub struct Web {
    views: Vec<(wry::WebView, Parent)>,
    js: String,
    last: String,
    sent: u64,
    skipped: u64,
}

fn page(widgets: &[(i32, i32)]) -> String {
    let mut html = String::from(
        r#"<!doctype html><html><head><meta charset="utf-8"><style>
html,body{margin:0;background:transparent;overflow:hidden;font:600 17px "Segoe UI";color:#f2f2f2}
.w{position:absolute;box-sizing:border-box;padding:8px 4px;background:rgba(15,18,23,.82);border-radius:8px}
.r{display:flex;height:30px;align-items:center;padding:0 8px}
.r.me{background:rgba(51,89,140,.9)}
.r.a{color:#ff7359}.r.b{color:#73b3ff}
.p{width:48px}.n{width:56px}.d{width:170px;white-space:nowrap;overflow:hidden}.g{flex:1;text-align:right}
</style></head><body>"#,
    );
    for (x, y) in widgets {
        let _ = write!(html, r#"<div class="w" style="left:{x}px;top:{y}px;width:{WIDGET_W}px;height:{WIDGET_H}px">"#);
        for _ in 0..ROWS {
            html.push_str(r#"<div class="r"><span class="p"></span><span class="n"></span><span class="d"></span><span class="g"></span></div>"#);
        }
        html.push_str("</div>");
    }
    html.push_str(
        r#"<script>
const rows=[...document.querySelectorAll('.w')].map(w=>[...w.children].map(r=>({r,c:[...r.children],k:''})));
function set(e,t){if(e.textContent!==t)e.textContent=t}
function u(d){for(const w of rows){for(let i=0;i<w.length;i++){const x=w[i],v=d[i];
 if(!v){if(x.k!=='x'){x.r.style.visibility='hidden';x.k='x'}continue}
 const k=v[4]===2?'r me':v[4]===1?'r a':v[4]===-1?'r b':'r';
 if(x.k!==k){x.r.className=k;x.r.style.visibility='';x.k=k}
 set(x.c[0],'P'+v[0]);set(x.c[1],'#'+v[1]);set(x.c[2],v[2]);set(x.c[3],v[3]);}}}
</script></body></html>"#,
    );
    html
}

impl Web {
    pub fn new(placements: &[Placement], layout: Layout) -> Result<Self> {
        let mut views = Vec::new();
        let mut add = |x: i32, y: i32, w: i32, h: i32, widgets: &[(i32, i32)]| -> Result<()> {
            let parent = Parent(win::overlay_window(x, y, w, h)?);
            let view = wry::WebViewBuilder::new()
                .with_transparent(true)
                .with_html(page(widgets))
                .build(&parent)
                .map_err(|e| anyhow!("webview: {e}"))?;
            views.push((view, parent));
            Ok(())
        };
        match layout {
            Layout::PerWidget => {
                for p in placements {
                    add(p.x, p.y, WIDGET_W, WIDGET_H, &[(0, 0)])?;
                }
            }
            Layout::PerMonitor => {
                let mut mons: Vec<_> = placements.iter().map(|p| p.monitor).collect();
                mons.sort_by_key(|r| (r.left, r.top));
                mons.dedup_by_key(|r| (r.left, r.top));
                for m in mons {
                    let ws: Vec<_> = placements
                        .iter()
                        .filter(|p| p.monitor == m)
                        .map(|p| (p.x - m.left, p.y - m.top))
                        .collect();
                    add(m.left, m.top, m.right - m.left, m.bottom - m.top, &ws)?;
                }
            }
        }
        Ok(Self { views, js: String::with_capacity(1024), last: String::with_capacity(1024), sent: 0, skipped: 0 })
    }
}

fn push_json_str(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out.push('"');
}

impl Renderer for Web {
    fn draw(&mut self, snap: &Snap, names: bool) -> Result<()> {
        let session = snap.session.as_deref();
        let js = &mut self.js;
        js.clear();
        js.push_str("u([");
        let mut tmp = String::new();
        for row in &snap.rows[..snap.count.min(ROWS)] {
            let driver = session.and_then(|s| s.driver(row.car_idx));
            let number = driver.map_or("?", |d| d.car_number.as_str());
            let _ = write!(js, "[{},", row.position);
            push_json_str(js, number);
            js.push(',');
            tmp.clear();
            match driver {
                Some(d) if names => tmp.push_str(&d.name),
                _ => {
                    let _ = write!(tmp, "Driver {number}");
                }
            }
            push_json_str(js, &tmp);
            if row.is_player {
                js.push_str(",\"\",2],");
            } else {
                let _ = write!(js, ",\"{:+.1}\",{}],", row.gap, row.lap_diff);
            }
        }
        js.push_str("])");
        if *js == self.last {
            self.skipped += 1;
            return Ok(());
        }
        self.last.clone_from(js);
        self.sent += 1;
        for (view, _) in &self.views {
            view.evaluate_script(js).map_err(|e| anyhow!("evaluate_script: {e}"))?;
        }
        Ok(())
    }

    fn stats(&self) -> String {
        format!("updates sent {} (to {} WebViews each), skipped (unchanged) {}", self.sent, self.views.len(), self.skipped)
    }
}
