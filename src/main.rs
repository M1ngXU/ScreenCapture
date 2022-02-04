#![allow(bad_style, non_camel_case_types)]

use std::iter::once;
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;

type handle = *mut i32;

#[repr(C)]
struct MSG {
    hWnd: handle,
    message: u32,
    wParam: usize,
    lParam: isize,
    time: u32,
    pt: POINT,
}

#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
}

#[repr(C)]
struct RECT {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct WNDCLASSEXW {
    cbSize: u32,
    style: u32,
    lpfnWndProc: Option<unsafe extern "system" fn(hWnd: handle, msg: u32, wparam: usize, lparam: isize) -> isize>,
    cbClsExtra: i32,
    cbWndExtra: i32,
    hInstance: handle,
    hIcon: handle,
    hCursor: handle,
    hbrBackground: handle,
    lpszMenuName: *const u16,
    lpszClassName: *const u16,
    hIconSm: handle,
}

#[link(name = "user32")]
extern "system" {
    fn ClientToScreen(hWnd: handle, lpPoint: *mut POINT) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn CreateWindowExW(
        dwExStyle: u32, lpClassName: *const u16, lpWindowName: *const u16, dwStyle: u32,
        x: i32, y: i32, w: i32, h: i32,
        hWndParent: handle, hMenu: handle, hInstance: handle, lparam: handle
    ) -> handle;
    fn DefWindowProcW(hWnd: handle, Msg: u32, wParam: usize, lParam: isize) -> isize;
    fn DispatchMessageW(lpmsg: *const MSG) -> isize;
    fn GetClientRect(hWnd: handle, lpRect: *mut RECT) -> i32;
    fn GetDC(hWnd: handle) -> handle;
    fn GetFocus() -> handle;
    fn GetMessageW(lpMsg: *mut MSG, hWnd: handle, wMsgFilterMin: u32, wMsgFilterMax: u32) -> i32;
    fn LoadCursorW(hInstance: handle, lpCursorName: *const u16) -> handle;
    fn OpenClipboard(hWnd: handle) -> i32;
    fn PostQuitMessage(nExitCode: i32);
    fn RegisterClassExW(lpWndClass: *const WNDCLASSEXW) -> u16;
    fn RegisterHotKey(hwnd: handle, id: i32, fsModifiers: u32, vk: u32) -> i32;
    fn ReleaseDC(hWnd: handle, hDC: handle) -> i32;
    fn SetClipboardData(uFormat: u32, hMem: handle) -> handle;
    fn SetFocus(hWnd: handle) -> handle;
    fn SetForegroundWindow(hWnd: handle) -> i32;
    fn SetLayeredWindowAttributes(hwnd: handle, crKey: u32, bAlpha: u8, dwFlags: u32) -> i32;
    fn SetThreadDpiAwarenessContext(dpiContext: handle) -> handle;
    fn SetTimer(hWnd: handle, nIDEvent: usize, uElapse: u32, proc: Option<unsafe extern "system" fn (handle, u32, usize, u32) -> ()>) -> usize;
    fn GetModuleHandleW(lpModuleName: *const u16) -> handle;
}

#[link(name = "gdi32")]
extern "stdcall" {
    fn BitBlt(hdc: handle, x: i32, y: i32, cx: i32, cy: i32, hdcSrc: handle, x1: i32, y1: i32, rop: u32) -> i32;
    fn CreateCompatibleBitmap(hdc: handle, cx: i32, cy: i32) -> handle;
    fn CreateCompatibleDC(hdc: handle) -> handle;
    fn CreateSolidBrush(color: u32) -> handle;
    fn DeleteDC(hdc: handle) -> i32;
    fn DeleteObject(ho: handle) -> i32;
    fn SelectObject(hdc: handle, h: handle) -> handle;
}

const BACKGROUND_TRANSPARENT_COLOR: u32 = 0x123456;
const TICK_DECREASE_STEP: u8 = 20;
const HOTKEY_ID: i32 = 123456;
const TIMER_ID: usize = 6543;

static mut DRAWING_TICKS: u8 = 0;

unsafe fn capture(hwnd: handle) -> isize {
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
    let old_obj = SelectObject(hdc, bitmap);

    BitBlt(hdc, 0, 0, w, h, dc, x, y, 0x00CC0020); // copy into dest.

    OpenClipboard(null_mut());
    EmptyClipboard();
    // write to clipboard bitmap
    SetClipboardData(2, bitmap);
    CloseClipboard();

    DRAWING_TICKS = u8::MAX;

    SelectObject(hdc, old_obj);
    DeleteDC(hdc);
    ReleaseDC(null_mut(), dc);
    DeleteObject(bitmap);
    0
}

unsafe fn set_window_transparency(hwnd: handle) {
    SetLayeredWindowAttributes(
        hwnd,
        BACKGROUND_TRANSPARENT_COLOR,
        u8::MAX - if DRAWING_TICKS < u8::MAX / 2 { DRAWING_TICKS } else { u8::MAX - DRAWING_TICKS } * 2, 0b11
    );
}

unsafe extern "system" fn window_proc(hwnd: handle, msg: u32, wparam: usize, lparam: isize) -> isize {
    match msg {
        // destroy
        0x0002 => {
            PostQuitMessage(0);
            0
        },
        // timer
        0x0113 if wparam == TIMER_ID && DRAWING_TICKS > 0 => {
            set_window_transparency(hwnd);
            DRAWING_TICKS = DRAWING_TICKS.saturating_sub(TICK_DECREASE_STEP);
            0
        },
        // hotkey
        0x0312 if wparam as i32 == HOTKEY_ID => {
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

fn main() {
    unsafe {
        // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE
        SetThreadDpiAwarenessContext(-2isize as handle);

        let win = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: 0b11, // VHRedraw
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(null_mut()),
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), 32512 as *const _), // arrow cursor
            hbrBackground: CreateSolidBrush(BACKGROUND_TRANSPARENT_COLOR),
            lpszMenuName: null_mut(),
            lpszClassName: "Main Capture Window".encode_utf16().chain(once(0)).collect::<Vec<u16>>().as_ptr(),
            hIconSm: null_mut(),
        };

        let hwnd = CreateWindowExW(
            0x00080200, //layered & client_edge
            RegisterClassExW(&win) as *const u16,
            "Capture".encode_utf16().chain(once(0)).collect::<Vec<u16>>().as_ptr(),
            0x10CF0000, // overlapped window & visible
            200, 200,
            500, 500,
            null_mut(),
            null_mut(),
            win.hInstance as handle,
            null_mut(),
        );
        set_window_transparency(hwnd);

        // ALT+C
        RegisterHotKey(hwnd, HOTKEY_ID, 0x4001, 0x43); // only triggers once; ALT+C
        SetTimer(hwnd, TIMER_ID, 5, None);

        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            DispatchMessageW(&msg);
        }
    }
}