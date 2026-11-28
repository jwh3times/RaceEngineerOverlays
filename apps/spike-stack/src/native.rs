//! PROTOTYPE: the Relative drawn with Direct2D/DirectWrite into composition
//! swap chains, hosted by DirectComposition in click-through windows.

use std::fmt::Write as _;

use anyhow::Result;
use windows::Win32::Foundation::{HMODULE, HWND};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1,
    D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_ROUNDED_RECT, D2D1CreateFactory, ID2D1Bitmap1, ID2D1DeviceContext, ID2D1Factory1,
    ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_MEASURING_MODE_NATURAL, DWRITE_TEXT_ALIGNMENT_TRAILING,
    DWriteCreateFactory, IDWriteFactory, IDWriteFontCollection, IDWriteTextFormat, IDWriteTextLayout,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, DXGI_PRESENT, DXGI_SCALING_STRETCH,
    DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT,
    IDXGIAdapter, IDXGIDevice, IDXGIFactory2, IDXGIOutput, IDXGISurface, IDXGISwapChain1,
};
use windows::core::{Interface, w};
use windows_numerics::Vector2;

use crate::relative::{ROWS, Snap};
use crate::{Layout, Placement, Renderer, WIDGET_H, WIDGET_W, win};

const COLS: usize = 4;
/// Column x ranges: position, number, name, gap (right-aligned).
const COL_X: [(f32, f32); COLS] = [(12.0, 60.0), (62.0, 116.0), (120.0, 290.0), (290.0, WIDGET_W as f32 - 12.0)];

/// One text cell: its string and the DirectWrite layout built for it, rebuilt
/// only when the string changes.
#[derive(Default)]
struct Cell {
    text: String,
    layout: Option<IDWriteTextLayout>,
}

struct Widget {
    swap: IDXGISwapChain1,
    target: ID2D1Bitmap1,
    _visual: IDCompositionVisual,
    cells: Vec<Cell>,
    kinds: [i8; ROWS],
    count: usize,
    drawn: bool,
}

pub struct Native {
    ctx: ID2D1DeviceContext,
    _dcomp: IDCompositionDevice,
    _hosts: Vec<(HWND, IDCompositionTarget, IDCompositionVisual)>,
    widgets: Vec<Widget>,
    left: IDWriteTextFormat,
    right: IDWriteTextFormat,
    bg: ID2D1SolidColorBrush,
    me: ID2D1SolidColorBrush,
    white: ID2D1SolidColorBrush,
    ahead: ID2D1SolidColorBrush,
    behind: ID2D1SolidColorBrush,
    dw: IDWriteFactory,
    /// This frame's cell strings and row kinds (2 = the player), built once
    /// and compared with what each widget last drew.
    content: Vec<String>,
    kinds: [i8; ROWS],
    count: usize,
    wide: Vec<u16>,
    pub layouts_built: u64,
    pub frames_skipped: u64,
    end_draw_s: f64,
    present_s: f64,
    draws: u64,
}

const fn rgba(r: f32, g: f32, b: f32, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F { r, g, b, a }
}

impl Native {
    pub fn new(placements: &[Placement], layout: Layout) -> Result<Self> {
        // SAFETY: COM object creation; every out-param is a valid local.
        unsafe {
            let mut device: Option<ID3D11Device> = None;
            D3D11CreateDevice(
                None::<&IDXGIAdapter>,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )?;
            let device = device.expect("D3D11 device");
            let dxgi: IDXGIDevice = device.cast()?;
            let factory: IDXGIFactory2 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))?;
            let d2d: ID2D1Factory1 = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let ctx = d2d.CreateDevice(&dxgi)?.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
            let dcomp: IDCompositionDevice = DCompositionCreateDevice(&dxgi)?;

            let dw: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let fmt = || {
                dw.CreateTextFormat(
                    w!("Segoe UI"),
                    None::<&IDWriteFontCollection>,
                    DWRITE_FONT_WEIGHT_SEMI_BOLD,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    17.0,
                    w!("en-us"),
                )
            };
            let left = fmt()?;
            let right = fmt()?;
            right.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_TRAILING)?;
            let brush = |c: D2D1_COLOR_F| ctx.CreateSolidColorBrush(&c, None);

            let mut widgets = Vec::new();
            let mut new_widget = || -> Result<Widget> {
                let desc = DXGI_SWAP_CHAIN_DESC1 {
                    Width: WIDGET_W as u32,
                    Height: WIDGET_H as u32,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                    BufferCount: 2,
                    Scaling: DXGI_SCALING_STRETCH,
                    SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                    AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
                    ..Default::default()
                };
                let swap = factory.CreateSwapChainForComposition(&device, &desc, None::<&IDXGIOutput>)?;
                let surface: IDXGISurface = swap.GetBuffer(0)?;
                let props = D2D1_BITMAP_PROPERTIES1 {
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    },
                    dpiX: 96.0,
                    dpiY: 96.0,
                    bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                    ..Default::default()
                };
                let target = ctx.CreateBitmapFromDxgiSurface(&surface, Some(&props))?;
                let visual = dcomp.CreateVisual()?;
                visual.SetContent(&swap)?;
                Ok(Widget {
                    swap,
                    target,
                    _visual: visual,
                    cells: (0..ROWS * COLS).map(|_| Cell::default()).collect(),
                    kinds: [0; ROWS],
                    count: 0,
                    drawn: false,
                })
            };

            let mut hosts = Vec::new();
            match layout {
                Layout::PerWidget => {
                    for p in placements {
                        let hwnd = win::overlay_window(p.x, p.y, WIDGET_W, WIDGET_H)?;
                        let wdg = new_widget()?;
                        let target = dcomp.CreateTargetForHwnd(hwnd, true)?;
                        target.SetRoot(&wdg._visual)?;
                        hosts.push((hwnd, target, wdg._visual.clone()));
                        widgets.push(wdg);
                    }
                }
                Layout::PerMonitor => {
                    let mut mons: Vec<_> = placements.iter().map(|p| p.monitor).collect();
                    mons.sort_by_key(|r| (r.left, r.top));
                    mons.dedup_by_key(|r| (r.left, r.top));
                    for m in mons {
                        let hwnd = win::overlay_window(m.left, m.top, m.right - m.left, m.bottom - m.top)?;
                        let root = dcomp.CreateVisual()?;
                        for p in placements.iter().filter(|p| p.monitor == m) {
                            let wdg = new_widget()?;
                            wdg._visual.SetOffsetX2((p.x - m.left) as f32)?;
                            wdg._visual.SetOffsetY2((p.y - m.top) as f32)?;
                            root.AddVisual(&wdg._visual, false, None::<&IDCompositionVisual>)?;
                            widgets.push(wdg);
                        }
                        let target = dcomp.CreateTargetForHwnd(hwnd, true)?;
                        target.SetRoot(&root)?;
                        hosts.push((hwnd, target, root));
                    }
                }
            }
            dcomp.Commit()?;

            Ok(Self {
                bg: brush(rgba(0.06, 0.07, 0.09, 0.82))?,
                me: brush(rgba(0.20, 0.35, 0.55, 0.90))?,
                white: brush(rgba(0.95, 0.95, 0.95, 1.0))?,
                ahead: brush(rgba(1.0, 0.45, 0.35, 1.0))?,
                behind: brush(rgba(0.45, 0.70, 1.0, 1.0))?,
                ctx,
                _dcomp: dcomp,
                _hosts: hosts,
                widgets,
                left,
                right,
                dw,
                content: (0..ROWS * COLS).map(|_| String::with_capacity(32)).collect(),
                kinds: [0; ROWS],
                count: 0,
                wide: Vec::with_capacity(64),
                layouts_built: 0,
                frames_skipped: 0,
                end_draw_s: 0.0,
                present_s: 0.0,
                draws: 0,
            })
        }
    }

    /// Builds this frame's strings once, for every widget to compare against.
    fn build_content(&mut self, snap: &Snap, names: bool) {
        let session = snap.session.as_deref();
        self.count = snap.count.min(ROWS);
        for (n, row) in snap.rows[..self.count].iter().enumerate() {
            self.kinds[n] = if row.is_player { 2 } else { row.lap_diff };
            let driver = session.and_then(|s| s.driver(row.car_idx));
            let number = driver.map_or("?", |d| d.car_number.as_str());
            let c = &mut self.content[n * COLS..n * COLS + COLS];
            c.iter_mut().for_each(String::clear);
            let _ = write!(c[0], "P{}", row.position);
            let _ = write!(c[1], "#{number}");
            match driver {
                Some(d) if names => c[2].push_str(&d.name),
                _ => {
                    let _ = write!(c[2], "Driver {number}");
                }
            }
            if !row.is_player {
                let _ = write!(c[3], "{:+.1}", row.gap);
            }
        }
    }
}

const ROW_H: f32 = 30.0;

impl Renderer for Native {
    fn draw(&mut self, snap: &Snap, names: bool) -> Result<()> {
        self.build_content(snap, names);
        let count = self.count;
        for i in 0..self.widgets.len() {
            let w = &mut self.widgets[i];
            let same = w.drawn
                && w.count == count
                && w.kinds[..count] == self.kinds[..count]
                && w.cells[..count * COLS]
                    .iter()
                    .zip(&self.content[..count * COLS])
                    .all(|(c, s)| c.text == *s);
            if same {
                self.frames_skipped += 1;
                continue;
            }
            // Rebuild only the layouts whose text changed.
            for k in 0..count * COLS {
                let cell = &mut w.cells[k];
                if cell.layout.is_some() && cell.text == self.content[k] {
                    continue;
                }
                cell.text.clear();
                cell.text.push_str(&self.content[k]);
                self.wide.clear();
                self.wide.extend(cell.text.encode_utf16());
                let (x0, x1) = COL_X[k % COLS];
                let fmt = if k % COLS == 3 { &self.right } else { &self.left };
                // SAFETY: plain DirectWrite factory call.
                cell.layout = Some(unsafe { self.dw.CreateTextLayout(&self.wide, fmt, x1 - x0, ROW_H)? });
                self.layouts_built += 1;
            }
            w.kinds = self.kinds;
            w.count = count;
            w.drawn = true;

            // SAFETY: one device context, used only on this thread.
            unsafe {
                self.ctx.SetTarget(&w.target);
                self.ctx.BeginDraw();
                self.ctx.Clear(Some(&rgba(0.0, 0.0, 0.0, 0.0)));
                let bg = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F { left: 0.0, top: 0.0, right: WIDGET_W as f32, bottom: WIDGET_H as f32 },
                    radiusX: 8.0,
                    radiusY: 8.0,
                };
                self.ctx.FillRoundedRectangle(&bg, &self.bg);
                for n in 0..count {
                    let y = 8.0 + n as f32 * ROW_H;
                    if self.kinds[n] == 2 {
                        let r = D2D_RECT_F { left: 4.0, top: y, right: WIDGET_W as f32 - 4.0, bottom: y + ROW_H };
                        self.ctx.FillRectangle(&r, &self.me);
                    }
                    let brush = match self.kinds[n] {
                        1 => &self.ahead,
                        -1 => &self.behind,
                        _ => &self.white,
                    };
                    for c in 0..COLS {
                        if let Some(layout) = &w.cells[n * COLS + c].layout {
                            let origin = Vector2 { X: COL_X[c].0, Y: y + 4.0 };
                            self.ctx.DrawTextLayout(origin, layout, brush, D2D1_DRAW_TEXT_OPTIONS_NONE);
                        }
                    }
                }
                let t0 = std::time::Instant::now();
                self.ctx.EndDraw(None, None)?;
                let t1 = std::time::Instant::now();
                w.swap.Present(0, DXGI_PRESENT(0)).ok()?;
                self.present_s += t1.elapsed().as_secs_f64();
                self.end_draw_s += (t1 - t0).as_secs_f64();
                self.draws += 1;
            }
        }
        Ok(())
    }

    fn stats(&self) -> String {
        let d = self.draws.max(1) as f64;
        format!(
            "layouts built {}, widget frames skipped (unchanged) {}, widget draws {}, mean EndDraw {:.0} us, mean Present {:.0} us",
            self.layouts_built, self.frames_skipped, self.draws, 1e6 * self.end_draw_s / d, 1e6 * self.present_s / d
        )
    }
}
