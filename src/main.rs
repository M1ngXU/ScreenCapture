use std::iter::once;
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;

use winapi::shared::minwindef::{HINSTANCE, UINT};
use winapi::shared::windef::{COLORREF, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE, HGDIOBJ, HWND, POINT, RECT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush,
    DeleteDC, DeleteObject, SelectObject, SRCCOPY
};
use winapi::um::winuser::{
    CF_BITMAP, ClientToScreen, CloseClipboard, CreateWindowExW, CS_HREDRAW, CS_VREDRAW,
    DefWindowProcW, DispatchMessageW, EmptyClipboard, GetClientRect, GetDC, GetFocus,
    GetMessageW, IDC_ARROW, LoadCursorW, LWA_ALPHA, LWA_COLORKEY, MOD_ALT, MOD_NOREPEAT,
    MSG, OpenClipboard, PostQuitMessage, RegisterClassExW, RegisterHotKey, ReleaseDC,
    SetClipboardData, SetFocus, SetForegroundWindow, SetLayeredWindowAttributes,
    SetThreadDpiAwarenessContext, SetTimer, WM_DESTROY, WM_HOTKEY, WM_TIMER, WNDCLASSEXW,
    WS_EX_CLIENTEDGE, WS_EX_LAYERED, WS_OVERLAPPEDWINDOW, WS_VISIBLE
};

const BACKGROUND_TRANSPARENT_COLOR: COLORREF = 0x123456;
const TICK_DECREASE_STEP: u8 = 20;
const HOTKEY_ID: i32 = 123456;
const TIMER_ID: usize = 6543;

static mut DRAWING_TICKS: u8 = 0;

unsafe fn capture(hwnd: HWND) -> isize {
    let mut tl: POINT = POINT {
        x: 0,
        y: 0
    };
    ClientToScreen(hwnd, &mut tl);
    // get screen dimensions
    let x = tl.x;
    let y = tl.y;

    let mut cr: RECT = zeroed();
    GetClientRect(hwnd, &mut cr);
    let w = cr.right - cr.left; // left should be 0
    let h = cr.bottom - cr.top; // top should be 0

    let dc = GetDC(null_mut());
    let hdc = CreateCompatibleDC(dc);
    let bitmap = CreateCompatibleBitmap(dc, w, h);
    let old_obj = SelectObject(hdc, bitmap as HGDIOBJ);

    BitBlt(hdc, 0, 0, w, h, dc, x, y, SRCCOPY);

    OpenClipboard(null_mut());
    EmptyClipboard();
    SetClipboardData(CF_BITMAP, bitmap as HGDIOBJ);
    CloseClipboard();
    //core::fmt::

    DRAWING_TICKS = u8::MAX;

    SelectObject(hdc, old_obj);
    DeleteDC(hdc);
    ReleaseDC(null_mut(), dc);
    DeleteObject(bitmap as HGDIOBJ);
    0
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }, WM_TIMER if wparam == TIMER_ID && DRAWING_TICKS > 0 => {
            SetLayeredWindowAttributes(hwnd, BACKGROUND_TRANSPARENT_COLOR, u8::MAX - if DRAWING_TICKS < u8::MAX / 2 { DRAWING_TICKS } else { u8::MAX - DRAWING_TICKS } * 2, LWA_ALPHA | LWA_COLORKEY);
            DRAWING_TICKS = DRAWING_TICKS.saturating_sub(TICK_DECREASE_STEP);
            0
        }, WM_HOTKEY if wparam as i32 == HOTKEY_ID => {
            if GetFocus() == hwnd {
                capture(hwnd)
            } else {
                SetForegroundWindow(hwnd);
                SetFocus(hwnd);
                0
            }
        },
        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

fn main(){
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE);

        let win = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as UINT,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(null_mut()) as HINSTANCE,
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: CreateSolidBrush(BACKGROUND_TRANSPARENT_COLOR),
            lpszMenuName: null_mut(),
            lpszClassName: "Main Capture Window".encode_utf16().chain(once(0)).collect::<Vec<u16>>().as_ptr(),
            hIconSm: null_mut(),
        };

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_CLIENTEDGE,
            RegisterClassExW(&win) as *const u16,
            "Capture".encode_utf16().chain(once(0)).collect::<Vec<u16>>().as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            200, 200,
            500, 500,
            null_mut(),
            null_mut(),
            win.hInstance,
            null_mut(),
        );
        SetLayeredWindowAttributes(hwnd, BACKGROUND_TRANSPARENT_COLOR, 0, LWA_COLORKEY);

        // ALT+C
        RegisterHotKey(hwnd, HOTKEY_ID, (MOD_NOREPEAT | MOD_ALT) as u32, 0x43);
        SetTimer(hwnd, TIMER_ID, 5, None);

        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            DispatchMessageW(&msg);
        }
    }
}