//! mascota: el gato de notify-claude en el escritorio.
//!
//! Se lanza con cada aviso y se cierra cuando el aviso se atiende. Es una ventana Win32
//! *layered* con alfa por píxel: lo transparente deja pasar los clics, y el borde inferior
//! de la ventana coincide con el de la zona de trabajo, así que el gato parece salir de
//! detrás de la barra de tareas.

#![cfg_attr(not(test), windows_subsystem = "windows")]

mod anim;
mod args;
mod bubble;
mod cat;
mod font_data;
mod scene;

use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows_sys::Win32::System::DataExchange::COPYDATASTRUCT;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use args::Request;
use bubble::Bubble;
use scene::{Director, Kind, Layout};

const CLASS: &str = "NotifyClaudeMascota";
const MUTEX: &str = "Local\\notify-claude-mascota";
const COPYDATA_ID: usize = 0x6d61_7363;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn main() {
    let Ok(req) = args::parse(std::env::args().skip(1)) else {
        std::process::exit(2);
    };
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // Instancia única. Si ya hay una, se le pasa la petición; si esa se está cerrando
        // justo ahora, se reintenta hasta heredar el mutex o encontrar su ventana.
        let (mutex_name, class) = (wide(MUTEX), wide(CLASS));
        for _ in 0..60 {
            let mutex = CreateMutexW(null(), 0, mutex_name.as_ptr());
            if GetLastError() != ERROR_ALREADY_EXISTS {
                if let Request::Show { kind, session, message, project } = req {
                    run(kind, session, &message, &project);
                }
                // un --dismiss sin nada abierto no tiene nada que cerrar
                return;
            }
            CloseHandle(mutex);
            let hwnd = FindWindowW(class.as_ptr(), null());
            if !hwnd.is_null() && forward(hwnd, &req) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

unsafe fn forward(hwnd: HWND, req: &Request) -> bool {
    let payload = req.encode();
    let cds = COPYDATASTRUCT { dwData: COPYDATA_ID, cbData: payload.len() as u32, lpData: payload.as_ptr() as *mut c_void };
    let mut accepted = 0usize;
    let sent = SendMessageTimeoutW(hwnd, WM_COPYDATA, 0, &cds as *const _ as LPARAM, SMTO_ABORTIFHUNG, 2000, &mut accepted);
    sent != 0 && accepted == 1
}

/// Búfer de 32 bits que se le entrega a `UpdateLayeredWindow`.
struct Surface {
    dc: HDC,
    bmp: HBITMAP,
    bits: *mut u32,
    w: i32,
    h: i32,
}

impl Surface {
    unsafe fn new(w: i32, h: i32) -> Surface {
        let screen = GetDC(null_mut());
        let dc = CreateCompatibleDC(screen);
        ReleaseDC(null_mut(), screen);
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h; // de arriba abajo
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB as u32;
        let mut bits: *mut c_void = null_mut();
        let bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        SelectObject(dc, bmp as HGDIOBJ);
        Surface { dc, bmp, bits: bits as *mut u32, w, h }
    }

    fn pixels(&mut self) -> &mut [u32] {
        if self.bits.is_null() {
            return &mut [];
        }
        // SAFETY: la sección DIB mide w*h píxeles de 32 bits y vive tanto como `self`.
        unsafe { std::slice::from_raw_parts_mut(self.bits, (self.w * self.h) as usize) }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            DeleteDC(self.dc);
            DeleteObject(self.bmp as HGDIOBJ);
        }
    }
}

struct App {
    hwnd: HWND,
    director: Director,
    bubble: Bubble,
    layout: Layout,
    surface: Surface,
    session: String,
    /// Zona de trabajo del monitor donde aparece el gato, y sus escalas enteras.
    work: RECT,
    scales: (i32, i32),
    start: Instant,
    frame: u64,
}

impl App {
    unsafe fn set_alert(&mut self, message: &str, project: &str) {
        self.bubble = Bubble::new(message, project);
        self.layout = Layout::new(&self.bubble, self.scales.0, self.scales.1);
        if (self.layout.w, self.layout.h) != (self.surface.w, self.surface.h) {
            self.surface = Surface::new(self.layout.w, self.layout.h);
        }
    }

    unsafe fn handle(&mut self, req: Request) {
        match req {
            Request::Show { kind, session, message, project } => {
                self.session = session;
                self.set_alert(&message, &project);
                self.director.alert(kind);
                self.render();
            }
            // solo cierra el aviso que se está viendo
            Request::Dismiss { session } => {
                if session.is_empty() || session == self.session {
                    self.director.dismiss();
                }
            }
        }
    }

    unsafe fn tick(&mut self) {
        let frame = (self.start.elapsed().as_secs_f64() * anim::FPS) as u64;
        if frame == self.frame {
            return;
        }
        let dt = ((frame - self.frame) as f64 / anim::FPS).min(0.25);
        self.frame = frame;
        if self.director.advance(dt) {
            self.render();
        } else {
            DestroyWindow(self.hwnd);
        }
    }

    unsafe fn render(&mut self) {
        let (pose, dy) = self.director.frame();
        let cat = cat::draw_cat(&pose);
        let bubble = self.director.bubble_visible().then(|| (&self.bubble, self.director.bubble_offset()));
        let lay = self.layout;
        scene::compose(self.surface.pixels(), &lay, &cat, dy, bubble);

        let pos = POINT { x: self.work.right - lay.w - 4 * lay.s, y: self.work.bottom - lay.h };
        let size = SIZE { cx: lay.w, cy: lay.h };
        let src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
        UpdateLayeredWindow(self.hwnd, null_mut(), &pos, &size, self.surface.dc, &src, 0, &blend, ULW_ALPHA);
    }
}

/// Zona de trabajo y DPI del monitor de la ventana activa (o del principal).
unsafe fn target_monitor() -> (RECT, u32) {
    let monitor = MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTOPRIMARY);
    let mut info: MONITORINFO = std::mem::zeroed();
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    GetMonitorInfoW(monitor, &mut info);
    let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
    if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) != 0 {
        dpi_x = 96;
    }
    (info.rcWork, dpi_x)
}

unsafe fn run(kind: Kind, session: String, message: &str, project: &str) {
    let (work, dpi) = target_monitor();
    let scales = scene::scales(dpi);
    let instance = GetModuleHandleW(null());
    let class = wide(CLASS);
    let mut wc: WNDCLASSW = std::mem::zeroed();
    wc.lpfnWndProc = Some(wndproc);
    wc.hInstance = instance;
    wc.hCursor = LoadCursorW(null_mut(), IDC_HAND);
    wc.lpszClassName = class.as_ptr();
    RegisterClassW(&wc);

    // Sin borde, siempre encima, sin botón en la barra de tareas y sin robar el foco.
    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        class.as_ptr(),
        wide("notify-claude").as_ptr(),
        WS_POPUP,
        0,
        0,
        0,
        0,
        null_mut(),
        null_mut(),
        instance,
        null(),
    );
    if hwnd.is_null() {
        return;
    }

    let bubble = Bubble::new(message, project);
    let layout = Layout::new(&bubble, scales.0, scales.1);
    let app = Box::into_raw(Box::new(App {
        hwnd,
        director: Director::new(kind),
        surface: Surface::new(layout.w, layout.h),
        bubble,
        layout,
        session,
        work,
        scales,
        start: Instant::now(),
        frame: 0,
    }));
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, app as isize);
    (*app).render();
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);

    // El temporizador de Windows va a saltos de ~15 ms; con 1 ms de resolución los 30 fps
    // salen regulares. La animación va por tiempo real, así que un tick tardío no la frena.
    timeBeginPeriod(1);
    SetTimer(hwnd, 1, 10, None);
    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    timeEndPeriod(1);
    drop(Box::from_raw(app));
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let app = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
    if app.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let app = &mut *app;
    match msg {
        WM_TIMER => {
            app.tick();
            0
        }
        // pulsar al gato (o su bocadillo) es decirle "ya lo he visto"
        WM_LBUTTONDOWN => {
            app.director.dismiss();
            0
        }
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        WM_COPYDATA => {
            let cds = &*(lparam as *const COPYDATASTRUCT);
            if cds.dwData != COPYDATA_ID || cds.lpData.is_null() {
                return 0;
            }
            let bytes = std::slice::from_raw_parts(cds.lpData as *const u8, cds.cbData as usize);
            match std::str::from_utf8(bytes).ok().and_then(Request::decode) {
                Some(req) => {
                    app.handle(req);
                    1
                }
                None => 0,
            }
        }
        WM_DESTROY => {
            KillTimer(hwnd, 1);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
