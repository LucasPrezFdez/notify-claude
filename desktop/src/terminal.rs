//! El terminal de cada aviso: la ventana donde trabaja la sesión de Claude Code que lo ha
//! mandado, para traerla al frente cuando pulsas al gato.
//!
//! - Al lanzarse, `mascota` sube por sus procesos padre hasta el primero que tiene consola, que
//!   es el de Claude Code (`session_pid`). Ese PID viaja con el aviso.
//! - Al pulsar, la app se engancha un momento a esa consola (`find`). En Windows Terminal, la
//!   ventana de la consola es una `PseudoConsoleWindow` invisible cuya dueña es la ventana del
//!   terminal, y el título de la consola es el de su pestaña. En la consola clásica, la ventana
//!   de la consola es la de verdad. Con un pseudoterminal sin dueña (VS Code, por ejemplo) no
//!   hay ventana que traer.
//! - La ventana se saca si está minimizada y se trae al frente con su tamaño (`raise`). Va
//!   primero porque Windows solo deja hacerlo al programa que ha recibido el último clic.
//! - Después, con UI Automation, se selecciona la pestaña con ese título (`select_tab`).

use std::collections::HashMap;
use std::ffi::c_void;

use windows_sys::Win32::Foundation::{CloseHandle, HWND, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Console::{AttachConsole, FreeConsole, GetConsoleTitleW, GetConsoleWindow};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetAncestor, GetClassNameW, IsIconic, IsWindowVisible, SetForegroundWindow, ShowWindow, GA_ROOTOWNER, SW_RESTORE,
};

/// Cuántos procesos padre se miran como mucho.
const MAX_DEPTH: usize = 8;

/// La ventana del terminal de una sesión y el título de su consola.
pub struct Target {
    pub window: HWND,
    pub title: Option<String>,
}

/// El PID del primer proceso padre cuya consola está en un terminal que se ve, o 0 si no hay
/// ninguno. Un proceso intermedio con una consola oculta no cuenta.
pub fn session_pid() -> u32 {
    unsafe {
        let parents = parents();
        let mut pid = GetCurrentProcessId();
        for _ in 0..MAX_DEPTH {
            match parents.get(&pid) {
                Some(&parent) if parent != 0 && parent != pid => pid = parent,
                _ => return 0,
            }
            if with_console(pid, |console| window_of(console).is_some()) == Some(true) {
                return pid;
            }
        }
        0
    }
}

/// La ventana del terminal donde corre el proceso `pid`, con el título de su consola.
pub fn find(pid: u32) -> Option<Target> {
    if pid == 0 {
        return None;
    }
    unsafe {
        let (window, title) = with_console(pid, |console| (window_of(console), console_title()))?;
        Some(Target { window: window?, title })
    }
}

/// Saca la ventana si está minimizada (vuelve con el tamaño que tenía) y la trae al frente.
pub fn raise(window: HWND) {
    unsafe {
        if IsIconic(window) != 0 {
            ShowWindow(window, SW_RESTORE);
        }
        SetForegroundWindow(window);
    }
}

/// Selecciona la pestaña de `window` que corresponde al título `title`. Devuelve si la ha
/// encontrado. Fuera de Windows Terminal no hay pestañas y no hace nada.
pub fn select_tab(window: HWND, title: &str) -> bool {
    unsafe { uia::select_tab(window, |names| pick_tab(names, title)).unwrap_or(false) }
}

/// La pestaña que se llama `title` o, si ninguna, la primera que solo cambia en el icono del
/// principio: Claude Code pone uno que va cambiando mientras trabaja (✳, ◐, ◓…).
pub fn pick_tab(names: &[String], title: &str) -> Option<usize> {
    if title.trim().is_empty() {
        return None;
    }
    names.iter().position(|n| n == title).or_else(|| {
        let t = without_icon(title);
        names.iter().position(|n| !t.is_empty() && without_icon(n) == t)
    })
}

/// `s` sin su primera palabra si esta no tiene letras ni cifras.
fn without_icon(s: &str) -> &str {
    let s = s.trim();
    match s.split_once(char::is_whitespace) {
        Some((icon, rest)) if !icon.chars().any(char::is_alphanumeric) => rest.trim_start(),
        _ => s,
    }
}

/// El padre de cada proceso que está corriendo.
unsafe fn parents() -> HashMap<u32, u32> {
    let mut map = HashMap::new();
    let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if snap == INVALID_HANDLE_VALUE {
        return map;
    }
    let mut e: PROCESSENTRY32W = std::mem::zeroed();
    e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    if Process32FirstW(snap, &mut e) != 0 {
        loop {
            map.insert(e.th32ProcessID, e.th32ParentProcessID);
            if Process32NextW(snap, &mut e) == 0 {
                break;
            }
        }
    }
    CloseHandle(snap);
    map
}

/// Se engancha un momento a la consola de `pid` y le pasa a `f` su ventana. Falla si el
/// proceso no tiene consola o si esta app ya tiene una (no es el caso: es de subsistema
/// `windows`).
unsafe fn with_console<T>(pid: u32, f: impl FnOnce(HWND) -> T) -> Option<T> {
    if AttachConsole(pid) == 0 {
        return None;
    }
    let r = f(GetConsoleWindow());
    FreeConsole();
    Some(r)
}

/// La ventana que se ve de una consola: su dueña en Windows Terminal, ella misma en la consola
/// clásica, o ninguna si es un pseudoterminal sin dueña o una consola oculta. Minimizada
/// también cuenta.
unsafe fn window_of(console: HWND) -> Option<HWND> {
    if console.is_null() {
        return None;
    }
    let root = GetAncestor(console, GA_ROOTOWNER);
    let window = if !root.is_null() && root != console {
        root
    } else if class_name(console) != "PseudoConsoleWindow" {
        console
    } else {
        return None;
    };
    (IsWindowVisible(window) != 0).then_some(window)
}

unsafe fn class_name(window: HWND) -> String {
    let mut buf = [0u16; 256];
    let n = GetClassNameW(window, buf.as_mut_ptr(), buf.len() as i32);
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

unsafe fn console_title() -> Option<String> {
    let mut buf = [0u16; 1024];
    let n = GetConsoleTitleW(buf.as_mut_ptr(), buf.len() as u32) as usize;
    (n > 0).then(|| String::from_utf16_lossy(&buf[..n.min(buf.len())]))
}

mod uia {
    use super::c_void;
    use windows::core::Result;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationSelectionItemPattern, TreeScope_Descendants, UIA_ControlTypePropertyId,
        UIA_SelectionItemPatternId, UIA_TabItemControlTypeId,
    };

    /// Busca las pestañas de `window`, le pasa sus nombres a `pick` y selecciona la que elija.
    pub unsafe fn select_tab(window: *mut c_void, pick: impl FnOnce(&[String]) -> Option<usize>) -> Result<bool> {
        let init = CoInitializeEx(None, COINIT_MULTITHREADED);
        // los objetos COM se sueltan dentro, antes de CoUninitialize
        let r = (|| {
            let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
            let root = uia.ElementFromHandle(HWND(window))?;
            let is_tab = uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(UIA_TabItemControlTypeId.0))?;
            let tabs = root.FindAll(TreeScope_Descendants, &is_tab)?;
            let mut names = Vec::new();
            for i in 0..tabs.Length()? {
                names.push(tabs.GetElement(i)?.CurrentName()?.to_string());
            }
            let Some(i) = pick(&names) else { return Ok(false) };
            let item: IUIAutomationSelectionItemPattern = tabs.GetElement(i as i32)?.GetCurrentPatternAs(UIA_SelectionItemPatternId)?;
            item.Select()?;
            Ok(true)
        })();
        if init.is_ok() {
            CoUninitialize();
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(n: &[&str]) -> Vec<String> {
        n.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn pestaña_por_titulo() {
        let tabs = names(&["◐ Cambios de HUD", "✳ Juegos multijugador", "◐ Maximizar terminal"]);
        assert_eq!(pick_tab(&tabs, "◐ Maximizar terminal"), Some(2));
        // el icono ha cambiado entre que se lee el título y se leen las pestañas
        assert_eq!(pick_tab(&tabs, "✳ Maximizar terminal"), Some(2));
        assert_eq!(pick_tab(&tabs, "◓ Juegos multijugador"), Some(1));
    }

    #[test]
    fn primero_el_titulo_exacto() {
        let tabs = names(&["✳ notify", "◐ notify"]);
        assert_eq!(pick_tab(&tabs, "◐ notify"), Some(1));
        assert_eq!(pick_tab(&tabs, "◓ notify"), Some(0));
    }

    #[test]
    fn sin_pestaña() {
        let tabs = names(&["✳ otra cosa", "Windows PowerShell", ""]);
        assert_eq!(pick_tab(&tabs, "◐ Maximizar terminal"), None);
        assert_eq!(pick_tab(&tabs, ""), None);
        assert_eq!(pick_tab(&tabs, "  "), None);
        assert_eq!(pick_tab(&names(&["✳"]), "◐"), None);
    }

    #[test]
    fn icono() {
        assert_eq!(without_icon("✳ Hola"), "Hola");
        assert_eq!(without_icon("✳️  Hola mundo "), "Hola mundo");
        assert_eq!(without_icon("¿Qué hago?"), "¿Qué hago?");
        assert_eq!(without_icon("Windows PowerShell"), "Windows PowerShell");
        assert_eq!(without_icon("✳"), "✳");
    }
}
